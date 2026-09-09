//! The account, the throttle and the breaker, tied together.
//!
//! Every call here is an internet round-trip, so nothing runs in the
//! background: no refresh loop, no verification probe. A scan spends a
//! request, and the send path spends one. What the API answered to the
//! command is what feeds the breaker.

mod impl_transport;
mod options;
mod shared;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, watch};

pub use self::options::Options;
use self::shared::{Shared, Tracked};
use crate::cloud::{api, status};
use crate::codec::cloud::Channel;
use crate::codec::{Encoded, Mode};
use crate::transport::error::{Error, Result};
use crate::transport::registry::{Devices, publish_sent};
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, Discovered, Event, Health, KnownDevice, Sent, Verify};

/// The `cloud` transport.
///
/// Cheap to clone; every clone shares one HTTP client, one device table and
/// one set of breakers.
#[derive(Clone)]
pub struct Transport {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Transport {
    /// Prints no key. It must never reach a log or a bug report — see
    /// `docs/security.md`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("api", &self.shared.client)
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// Build the client for one account.
    ///
    /// No request is made here, so a build with a key and no network starts
    /// fine and fails at the first command.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if the key is empty or the HTTP client cannot be
    /// built.
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

    /// List the account's devices.
    ///
    /// This is the whole of discovery in this mode: the API answers with what
    /// the account owns, wherever those devices are. It spends a request, so
    /// nothing calls it on the send path.
    ///
    /// # Errors
    ///
    /// [`Error::Api`], [`Error::RateLimited`] or [`Error::Io`], as the request
    /// fails.
    pub async fn scan(&self) -> Result<Vec<Discovered>> {
        let listed = self.shared.client.devices().await?;
        let endpoint = self.shared.client.endpoint("");
        let mut out = Vec::with_capacity(listed.len());

        for device in listed {
            let id = DeviceId::new(&device.device);
            {
                let mut devices = self.shared.devices.lock()?;
                let tracked = devices.entry(id.clone()).or_insert_with(|| {
                    Tracked::new(
                        device.sku.clone(),
                        device.device_name.clone(),
                        self.shared.policy,
                    )
                });
                tracked.sku.clone_from(&device.sku);
                tracked.name.clone_from(&device.device_name);
            }
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
    /// The API answers whether it accepted the command, so this mode needs no
    /// verification request: [`Verify`] is read for nothing, and the answer
    /// feeds the breaker. A second request would spend the quota to learn what
    /// the first one already said.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if no scan has listed the device,
    /// [`Error::Unavailable`] if the breaker refuses this mode,
    /// [`Error::RateLimited`] if the quota is spent, [`Error::Serialize`] if
    /// the command carries no capability, or [`Error::Api`] if the API
    /// refuses it.
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
    /// As for [`Transport::send`], and [`Error::NoReplyLayout`] if the command
    /// declares no `reads:` for the answer to land in.
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
        channel(request)?;
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

/// The capability object a write carries, and the channel it goes out on.
fn capability(command: &Encoded) -> Result<&serde_json::Value> {
    channel(command)?;
    command
        .message
        .as_ref()
        .and_then(|message| message.get("capability"))
        .ok_or_else(|| Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "the command carries no capability to write".to_owned(),
        })
}

/// A command on the account's MQTT channel is refused here rather than
/// approximated over HTTPS — see `docs/protocol/cloud.md`.
fn channel(command: &Encoded) -> Result<()> {
    match command.request.as_ref().map(|request| request.channel) {
        Some(Channel::Http) => Ok(()),
        Some(Channel::Iot) => Err(Error::NoReplyLayout {
            mode: Mode::Cloud,
            reason: format!(
                "`{}` travels on the account's MQTT channel, which this build does not carry",
                command.cmd
            ),
        }),
        None => Err(Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "encoded for another mode: it carries no cloud request".to_owned(),
        }),
    }
}
