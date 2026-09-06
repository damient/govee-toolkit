//! The state every [`Transport`](super::Transport) clone shares, and the send
//! path that reads it.
//!
//! Routing answers from memory, as it does for every mode. The link is what
//! differs: a device answers only over a connection, so the send path can have
//! to open one, and it opens one connection per device at a time.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use btleplug::api::{Central as _, Manager as _, Peripheral as _};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::sync::{OnceCell, broadcast, watch};

use crate::ble::link::{Link, adapter as adapter_error};
use crate::ble::pace::{Budget, Pacer};
use crate::ble::transport::Options;
use crate::codec::{Encoded, Mode};
use crate::transport::DeviceId;
use crate::transport::breaker::{Breaker, Policy};
use crate::transport::error::{Error, Result};
use crate::transport::events::Event;
use crate::transport::registry::Devices;
use crate::transport::status::DeviceStatus;

/// One device, as the transport tracks it.
pub(super) struct Tracked {
    /// The handle the platform addresses the peripheral by. Not the identity:
    /// see [`super::Transport::bind`].
    pub(super) endpoint: String,
    pub(super) sku: String,
    pub(super) breaker: Breaker,
    pub(super) status: watch::Sender<Option<DeviceStatus>>,
    /// One budget per device, because the limit is one firmware's.
    pub(super) pacer: Arc<Pacer>,
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    pub(super) verified_at: Option<Instant>,
}

impl crate::transport::registry::Tracked for Tracked {
    fn sku(&self) -> &str {
        &self.sku
    }

    fn breaker(&self) -> &Breaker {
        &self.breaker
    }

    fn breaker_mut(&mut self) -> &mut Breaker {
        &mut self.breaker
    }

    fn status(&self) -> &watch::Sender<Option<DeviceStatus>> {
        &self.status
    }

    fn verified_at(&mut self) -> &mut Option<Instant> {
        &mut self.verified_at
    }
}

impl Tracked {
    pub(super) fn new(endpoint: String, sku: String, policy: Policy, budget: Budget) -> Self {
        Self {
            endpoint,
            sku,
            breaker: Breaker::new(policy),
            status: watch::Sender::new(None),
            pacer: Arc::new(Pacer::new(budget)),
            verified_at: None,
        }
    }
}

/// Where to send, and what to send it with.
pub(super) struct Route {
    pub(super) endpoint: String,
    pub(super) pacer: Arc<Pacer>,
    /// Whether this command pays for a verification.
    pub(super) verifying: bool,
}

pub(super) struct Shared {
    pub(super) options: Options,
    /// The write budget, checked once when the transport was built.
    pub(super) budget: Budget,
    /// Claimed on first use. The transport must start on a machine whose radio
    /// is off, so the first command is what reports it.
    adapter: OnceCell<Adapter>,
    pub(super) devices: Devices<Tracked>,
    /// One open connection per device, reused across commands. A device
    /// accepts only one, and a new connection costs seconds.
    links: tokio::sync::Mutex<HashMap<DeviceId, Arc<Link>>>,
    pub(super) events: broadcast::Sender<Event>,
}

impl Shared {
    pub(super) fn new(options: Options, budget: Budget, events: broadcast::Sender<Event>) -> Self {
        Self {
            options,
            budget,
            adapter: OnceCell::new(),
            devices: Devices::new(),
            links: tokio::sync::Mutex::new(HashMap::new()),
            events,
        }
    }

    /// The adapter, claimed if it has not been already.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the platform reports no usable adapter.
    pub(super) async fn adapter(&self) -> Result<&Adapter> {
        self.adapter
            .get_or_try_init(|| async {
                let manager = Manager::new()
                    .await
                    .map_err(|e| adapter_error("ble", "opening the Bluetooth manager", &e))?;
                manager
                    .adapters()
                    .await
                    .map_err(|e| adapter_error("ble", "listing Bluetooth adapters", &e))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        Error::io(
                            "ble: this machine reports no Bluetooth adapter",
                            std::io::ErrorKind::NotFound.into(),
                        )
                    })
            })
            .await
    }

    /// Where to send, decided from recorded state alone.
    ///
    /// Claiming marks the device verified, so a burst of commands produces one
    /// probe.
    pub(super) fn route_and_claim(
        &self,
        id: &DeviceId,
        now: Instant,
        claim: bool,
    ) -> Result<Route> {
        let interval = if claim {
            self.options.verify_interval
        } else {
            None
        };
        let ((endpoint, pacer), verifying) =
            self.devices
                .route_and_claim(id, Mode::Ble, now, interval, |tracked| {
                    (tracked.endpoint.clone(), Arc::clone(&tracked.pacer))
                })?;
        Ok(Route {
            endpoint,
            pacer,
            verifying,
        })
    }

    /// The open connection to a device, or a new one if there is none.
    ///
    /// # Errors
    ///
    /// [`Error::Unreachable`] if the connection takes longer than
    /// [`Options::connect_timeout`], or [`Error::Io`] if nothing advertises at
    /// the handle after a scan, or if the connection fails.
    pub(super) async fn link(&self, id: &DeviceId, endpoint: &str) -> Result<Arc<Link>> {
        let mut links = self.links.lock().await;
        if let Some(link) = links.get(id)
            && link.is_live().await
        {
            return Ok(Arc::clone(link));
        }
        links.remove(id);

        // A handle is good only while the platform still holds the peripheral
        // behind it, and macOS drops that when a link goes down: the device
        // must be heard advertising again before anything can connect to it.
        // The scan reaches the same device over the same mode, so it
        // substitutes nothing. It costs seconds, so it runs only once the
        // handle is gone.
        let peripheral = match self.peripheral(endpoint).await? {
            Some(peripheral) => peripheral,
            None => self.rediscover(endpoint).await?,
        };
        // A peripheral that never answers leaves `connect` pending for as long
        // as the platform waits, and every caller behind this lock waits with
        // it.
        let timeout = self.options.connect_timeout;
        let link = tokio::time::timeout(timeout, Link::open(peripheral, endpoint))
            .await
            .map_err(|_| Error::Unreachable {
                id: id.clone(),
                endpoint: endpoint.to_owned(),
                timeout_ms: crate::transport::millis(timeout),
            })??;
        let link = Arc::new(link);
        links.insert(id.clone(), Arc::clone(&link));
        Ok(link)
    }

    /// Forget a device's connection, so the next command opens a new one.
    pub(super) async fn drop_link(&self, id: &DeviceId) {
        self.links.lock().await.remove(id);
    }

    /// Scan again, and answer with the peripheral that came back.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the scan fails, or if nothing is advertising at the
    /// handle once it has run.
    async fn rediscover(&self, endpoint: &str) -> Result<Peripheral> {
        self.scan(self.options.rescan_window).await?;
        self.peripheral(endpoint).await?.ok_or_else(|| {
            Error::io(
                format!("{endpoint}: nothing at this handle is advertising"),
                std::io::ErrorKind::NotFound.into(),
            )
        })
    }

    /// The peripheral behind a handle, among those the adapter has seen, or
    /// `None` if the adapter is not holding one.
    ///
    /// A handle that several peripherals carry names no device, and a
    /// connection to one of them would write the command into whatever the
    /// adapter listed first.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the adapter cannot be listed, or if more than one
    /// peripheral carries the handle.
    async fn peripheral(&self, endpoint: &str) -> Result<Option<Peripheral>> {
        let adapter = self.adapter().await?;
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| adapter_error(endpoint, "listing known peripherals", &e))?;
        let mut matching = peripherals
            .into_iter()
            .filter(|p| p.id().to_string().eq_ignore_ascii_case(endpoint));
        let Some(found) = matching.next() else {
            return Ok(None);
        };
        let others = matching.count();
        if others > 0 {
            return Err(Error::io(
                format!(
                    "{endpoint}: {} peripherals carry this handle, so it names none of them",
                    others + 1
                ),
                std::io::ErrorKind::InvalidData.into(),
            ));
        }
        Ok(Some(found))
    }

    /// Write every frame of a command, at the device's budget.
    ///
    /// A failed write drops the link: the connection is gone or the device
    /// went away, and the next command must open a new one rather than write
    /// into a dead handle.
    pub(super) async fn write_frames(
        &self,
        id: &DeviceId,
        route: &Route,
        link: &Link,
        command: &Encoded,
    ) -> Result<()> {
        check_frames(command)?;
        for frame in &command.frames {
            self.write_frame(id, route, link, &command.cmd, frame)
                .await?;
        }
        Ok(())
    }

    /// Hand a status to the device's watchers and to the event stream.
    pub(super) fn publish_status(&self, status: DeviceStatus) {
        self.devices.publish_status(&self.events, Mode::Ble, status);
    }

    /// Feed the breaker and publish the transition, if there was one.
    pub(super) fn record(&self, id: &DeviceId, answered: bool, now: Instant) {
        self.devices
            .record(&self.events, id, Mode::Ble, answered, now);
    }

    /// The open connection to a device, opening one if there is none, and a
    /// failure recorded against the breaker if it cannot be opened.
    ///
    /// A device that will not take a connection is unreachable, and a record
    /// now spares the next command the same wait.
    ///
    /// # Errors
    ///
    /// As for [`Shared::link`].
    pub(super) async fn connect(&self, id: &DeviceId, endpoint: &str) -> Result<Arc<Link>> {
        match self.link(id, endpoint).await {
            Ok(link) => Ok(link),
            Err(e) => {
                self.record(id, false, Instant::now());
                Err(e)
            }
        }
    }
}

/// Which identity a handle is tracked under, if any.
pub(super) fn id_at(devices: &HashMap<DeviceId, Tracked>, endpoint: &str) -> Option<DeviceId> {
    devices.iter().find_map(|(id, tracked)| {
        tracked
            .endpoint
            .eq_ignore_ascii_case(endpoint)
            .then(|| id.clone())
    })
}

/// Refuse a command this wire has nothing to send for.
///
/// # Errors
///
/// [`Error::Serialize`] if the command carries no frames: `ble` writes frames
/// and nothing else, so an envelope-only command is a device file describing
/// another mode.
pub(super) fn check_frames(command: &Encoded) -> Result<()> {
    if command.frames.is_empty() {
        return Err(Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "the command carries no frames, and this wire carries nothing else".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_command_with_nothing_to_write_is_refused() {
        let command = Encoded {
            cmd: "power".to_owned(),
            message: Some(serde_json::json!({"msg": {}})),
            frames: Vec::new(),
            replies: Vec::new(),
            roles: std::collections::BTreeMap::new(),
        };
        let error = check_frames(&command).expect_err("nothing to write");
        assert_eq!(error.code(), "serialize");

        let carried = Encoded {
            frames: vec![vec![0; 20]],
            ..command
        };
        assert!(check_frames(&carried).is_ok());
    }
}
