//! The account, the throttle and the breaker, tied together.

mod impl_transport;
mod options;
mod shared;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, watch};

pub use self::options::Options;
use self::shared::{Shared, Tracked};
use crate::cloud::{api, status};
use crate::codec::{Encoded, Mode};
use crate::transport::error::{Error, Result};
use crate::transport::registry::{Devices, publish_sent};
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, Discovered, Event, Health, KnownDevice, Sent, Verify};

/// The `cloud` transport. Cheap to clone: every clone shares one HTTP client,
/// one device table and one set of breakers.
#[derive(Clone)]
pub struct Transport {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Transport {
    /// Prints no key — see `docs/security.md`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("api", &self.shared.client)
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// Build the client for one account. It sends no request, so a build with
    /// no network starts fine and fails at the first command.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] on an empty key.
    pub fn start(options: Options) -> Result<Self> {
        if options.key.trim().is_empty() {
            return Err(Error::Option {
                field: "key".to_owned(),
                reason: "this mode needs an API key; see docs/protocol/cloud.md".to_owned(),
            });
        }
        let client = api::Client::new(options.key, &options.base_url, options.request_timeout)?;
        let (events, _) = broadcast::channel(256);

        Ok(Self {
            shared: Arc::new(Shared {
                client,
                policy: options.policy,
                request_timeout: options.request_timeout,
                min_interval: options.min_interval,
                max_wait: options.max_wait,
                devices: Devices::new(),
                events,
            }),
        })
    }

    /// Subscribe to transport events.
    #[must_use]
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.shared.events.subscribe()
    }

    /// How long a scan takes at most. One request, so this is its timeout.
    #[must_use]
    pub fn scan_window(&self) -> Duration {
        self.shared.request_timeout
    }

    /// List the account's devices. This is the whole of discovery in this
    /// mode, and it spends a request.
    ///
    /// # Errors
    ///
    /// [`Error::Api`], [`Error::RateLimited`] or [`Error::Io`].
    pub async fn scan(&self) -> Result<Vec<Discovered>> {
        let listed = self.shared.client.devices().await?;
        let endpoint = self.shared.client.endpoint("");
        let mut out = Vec::with_capacity(listed.len());

        // Nothing here waits, so one lock serves the whole list.
        let mut devices = self.shared.devices.lock()?;
        for device in listed {
            let id = DeviceId::new(&device.device);
            let tracked = devices.entry(id.clone()).or_insert_with(|| {
                Tracked::new(
                    device.sku.clone(),
                    device.device_name.clone(),
                    self.shared.policy,
                )
            });
            tracked.sku.clone_from(&device.sku);
            tracked.name.clone_from(&device.device_name);
            let found = Discovered {
                id,
                endpoint: endpoint.clone(),
                sku: device.sku,
                firmware: None,
            };
            let _ = self.shared.events.send(Event::Discovered {
                mode: Mode::Cloud,
                device: found.clone(),
                change: crate::transport::Change::Refreshed,
            });
            out.push(found);
        }
        Ok(out)
    }

    /// List the account's devices, and pick one out.
    ///
    /// One request answers for every device, so there is nothing to return
    /// early from: this costs what [`Transport::scan`] costs.
    ///
    /// # Errors
    ///
    /// As for [`Transport::scan`].
    pub async fn scan_for(&self, id: &DeviceId) -> Result<Option<Discovered>> {
        Ok(self
            .scan()
            .await?
            .into_iter()
            .find(|device| &device.id == id))
    }

    /// Every device the last scan listed.
    #[must_use]
    pub fn devices(&self) -> Vec<KnownDevice> {
        let endpoint = self.shared.client.endpoint("");
        self.shared.devices.known(|_| endpoint.clone())
    }

    /// The SKU the account reports for a device.
    #[must_use]
    pub fn sku(&self, id: &DeviceId) -> Option<String> {
        self.shared.devices.sku(id)
    }

    /// The name the account gave a device. Never identity.
    #[must_use]
    pub fn name(&self, id: &DeviceId) -> Option<String> {
        Some(self.shared.devices.lock().ok()?.get(id)?.name.clone())
    }

    /// A device's health in this mode, if it is known.
    #[must_use]
    pub fn health(&self, id: &DeviceId) -> Option<Health> {
        self.shared.devices.health(id)
    }

    /// The last status heard, without asking for a new one.
    #[must_use]
    pub fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        self.shared.devices.last_status(id)
    }

    /// Watch a device's status as answers arrive.
    #[must_use]
    pub fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        self.shared.devices.watch_status(id)
    }

    /// Write one capability.
    ///
    /// The answer is the verification: [`Verify`] is read for nothing, and
    /// what the API answered feeds the breaker.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`], [`Error::Unavailable`],
    /// [`Error::RateLimited`], [`Error::Serialize`] or [`Error::Api`].
    pub async fn send(&self, id: &DeviceId, command: &Encoded, _verify: Verify) -> Result<Sent> {
        let capability = capability(command)?;
        let sku = self.shared.claim(id).await?;

        let outcome = self
            .shared
            .client
            .control(&sku, id.as_str(), capability)
            .await;
        self.shared.record(id, outcome.is_ok());
        outcome?;

        let sent = Sent {
            id: id.clone(),
            mode: Mode::Cloud,
            cmd: command.cmd.clone(),
            endpoint: self.shared.client.endpoint(""),
        };
        publish_sent(&self.shared.events, &sent);
        Ok(sent)
    }

    /// Read a device's state.
    ///
    /// # Errors
    ///
    /// As for [`Transport::send`], plus [`Error::NoReplyLayout`] on a command
    /// that declares no `reads:`.
    pub async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        let reads = request
            .request
            .as_ref()
            .is_some_and(|r| !r.reads.is_empty());
        if !reads {
            return Err(Error::NoReplyLayout {
                mode: Mode::Cloud,
                reason: "the command declares no `reads:`, so the answer reaches nothing"
                    .to_owned(),
            });
        }
        let sku = self.shared.claim(id).await?;

        let outcome = self.shared.client.state(&sku, id.as_str()).await;
        self.shared.record(id, outcome.is_ok());
        let status = status::read(id.clone(), request, &outcome?);

        self.shared
            .devices
            .publish_status(&self.shared.events, Mode::Cloud, status.clone());
        Ok(status)
    }
}

fn capability(command: &Encoded) -> Result<&serde_json::Value> {
    if command.request.is_none() {
        return Err(Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "encoded for another mode: it carries no cloud request".to_owned(),
        });
    }
    command
        .message
        .as_ref()
        .and_then(|message| message.get("capability"))
        .ok_or_else(|| Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "the command carries no capability to write".to_owned(),
        })
}
