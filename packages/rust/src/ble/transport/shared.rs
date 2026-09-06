//! The state every [`Transport`](super::Transport) clone shares, and the send
//! path that reads it.
//!
//! Routing answers from memory, as it does for every mode. What is different
//! here is the link: reaching a device means holding a connection to it, so
//! the send path may have to open one, and one device's connection is opened
//! at a time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use btleplug::api::{Central as _, Manager as _, Peripheral as _};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::sync::{OnceCell, broadcast, watch};

use crate::ble::link::{Link, adapter as adapter_error};
use crate::ble::pace::{Budget, Pacer};
use crate::ble::transport::Options;
use crate::codec::{Encoded, Mode};
use crate::transport::DeviceId;
use crate::transport::breaker::Breaker;
use crate::transport::error::{Error, Result};
use crate::transport::events::Event;
use crate::transport::status::DeviceStatus;

/// One device, as the transport tracks it.
pub(super) struct Tracked {
    /// The Bluetooth address to connect to. Not the identity: see
    /// [`super::Transport::bind`].
    pub(super) address: String,
    pub(super) sku: String,
    pub(super) breaker: Breaker,
    pub(super) status: watch::Sender<Option<DeviceStatus>>,
    /// One budget per device, because the limit is one firmware's.
    pub(super) pacer: Arc<Pacer>,
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    pub(super) verified_at: Option<Instant>,
}

impl Tracked {
    pub(super) fn new(address: String, sku: String, options: &Options, budget: Budget) -> Self {
        Self {
            address,
            sku,
            breaker: Breaker::new(options.policy),
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
    /// Whether this command should pay for a verification.
    pub(super) verifying: bool,
}

pub(super) struct Shared {
    pub(super) options: Options,
    /// The write budget, checked once when the transport was built.
    pub(super) budget: Budget,
    /// Claimed on first use. Starting the transport must not fail on a machine
    /// whose radio is off or arrives later; the first command is what reports
    /// it.
    adapter: OnceCell<Adapter>,
    pub(super) devices: Mutex<HashMap<DeviceId, Tracked>>,
    /// One open connection per device, reused across commands. A device
    /// accepts only one, and reconnecting costs seconds.
    links: tokio::sync::Mutex<HashMap<DeviceId, Arc<Link>>>,
    pub(super) events: broadcast::Sender<Event>,
}

impl Shared {
    pub(super) fn new(options: Options, budget: Budget, events: broadcast::Sender<Event>) -> Self {
        Self {
            options,
            budget,
            adapter: OnceCell::new(),
            devices: Mutex::new(HashMap::new()),
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
        let mut devices = self.devices.lock().map_err(|_| Error::ShutDown)?;
        let tracked = devices
            .get_mut(id)
            .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
        if !tracked.breaker.allows(now) {
            return Err(Error::Unavailable {
                id: id.clone(),
                mode: Mode::Ble,
                state: tracked.breaker.state(),
            });
        }

        let verifying = claim
            && self.options.verify_interval.is_some_and(|interval| {
                tracked
                    .verified_at
                    .is_none_or(|at| now.duration_since(at) >= interval)
            });
        if verifying {
            tracked.verified_at = Some(now);
        }
        Ok(Route {
            endpoint: tracked.address.clone(),
            pacer: Arc::clone(&tracked.pacer),
            verifying,
        })
    }

    /// The open connection to a device, opening one if there is none.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if the address has not been seen advertising,
    /// or [`Error::Io`] if the connection cannot be established.
    pub(super) async fn link(&self, id: &DeviceId, endpoint: &str) -> Result<Arc<Link>> {
        let mut links = self.links.lock().await;
        if let Some(link) = links.get(id)
            && link.is_live().await
        {
            return Ok(Arc::clone(link));
        }
        links.remove(id);

        let peripheral = self.peripheral(endpoint).await?;
        let link = Arc::new(Link::open(peripheral, endpoint).await?);
        links.insert(id.clone(), Arc::clone(&link));
        Ok(link)
    }

    /// Forget a device's connection, so the next command opens a new one.
    pub(super) async fn drop_link(&self, id: &DeviceId) {
        self.links.lock().await.remove(id);
    }

    /// The peripheral at an address, among those the adapter has seen.
    async fn peripheral(&self, endpoint: &str) -> Result<Peripheral> {
        let adapter = self.adapter().await?;
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| adapter_error(endpoint, "listing known peripherals", &e))?;
        peripherals
            .into_iter()
            .find(|p| p.address().to_string().eq_ignore_ascii_case(endpoint))
            .ok_or_else(|| {
                Error::io(
                    format!("{endpoint}: this address has not been seen advertising; scan first"),
                    std::io::ErrorKind::NotFound.into(),
                )
            })
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
            route.pacer.acquire().await;
            if let Err(e) = link.write_frame(&command.cmd, &route.endpoint, frame).await {
                self.drop_link(id).await;
                self.record(id, false, Instant::now());
                return Err(e);
            }
        }
        Ok(())
    }

    /// Hand a status to the device's watchers and to the event stream.
    pub(super) fn publish_status(&self, id: &DeviceId, status: DeviceStatus) {
        if let Ok(devices) = self.devices.lock()
            && let Some(tracked) = devices.get(id)
        {
            let _ = tracked.status.send(Some(status.clone()));
        }
        let _ = self.events.send(Event::Status {
            mode: Mode::Ble,
            status,
        });
    }

    /// Feed the breaker and publish the transition, if there was one.
    pub(super) fn record(&self, id: &DeviceId, answered: bool, now: Instant) {
        let transition = {
            let Ok(mut devices) = self.devices.lock() else {
                return;
            };
            let Some(tracked) = devices.get_mut(id) else {
                return;
            };
            if answered {
                tracked.breaker.record_success(now)
            } else {
                tracked.breaker.record_failure(now)
            }
        };
        if transition.changed() {
            tracing::info!(%id, from = %transition.from, to = %transition.to, "ble health changed");
            let _ = self.events.send(Event::HealthChanged {
                id: id.clone(),
                mode: Mode::Ble,
                transition,
            });
        }
    }
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
