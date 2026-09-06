//! The state every [`Transport`](super::Transport) clone shares, and the send
//! path that reads it.
//!
//! Everything here answers from memory. The one method that waits on the
//! network is [`Shared::request_status`], and it is deliberately off the send
//! path — see the module documentation of [`super`].

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};

use crate::codec::{Encoded, Mode};
use crate::lan::cache::Cache;
use crate::lan::discovery::{DiscoveredDevice, Endpoints};
use crate::lan::socket::Socket;
use crate::transport::breaker::{Breaker, Policy};
use crate::transport::error::{Error, Result};
use crate::transport::events::Event;
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, millis};

/// One device, as the transport tracks it.
pub(super) struct Tracked {
    pub(super) ip: IpAddr,
    pub(super) sku: String,
    pub(super) breaker: Breaker,
    pub(super) status: watch::Sender<Option<DeviceStatus>>,
    /// A status request is out; another waiter should listen rather than send.
    pub(super) probing: bool,
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    pub(super) verified_at: Option<Instant>,
}

impl Tracked {
    pub(super) fn new(ip: IpAddr, sku: String, policy: Policy) -> Self {
        Self {
            ip,
            sku,
            breaker: Breaker::new(policy),
            status: watch::Sender::new(None),
            probing: false,
            verified_at: None,
        }
    }
}

pub(super) struct Shared {
    pub(super) socket: Socket,
    pub(super) endpoints: Endpoints,
    pub(super) policy: Policy,
    pub(super) status_timeout: Duration,
    pub(super) verify_interval: Option<Duration>,
    pub(super) devices: Mutex<HashMap<DeviceId, Tracked>>,
    pub(super) cache: Mutex<Cache>,
    pub(super) events: broadcast::Sender<Event>,
    pub(super) replies: broadcast::Sender<DiscoveredDevice>,
}

impl Shared {
    /// Make the cached devices usable before any scan has run.
    pub(super) fn adopt_cached_devices(&self) {
        let Ok(cache) = self.cache.lock() else {
            return;
        };
        let Ok(mut devices) = self.devices.lock() else {
            return;
        };
        for cached in cache.devices() {
            devices.insert(
                cached.id.clone(),
                Tracked::new(cached.ip, cached.sku.clone(), self.policy),
            );
        }
    }

    /// Where to send, and whether this command should pay for a verification.
    ///
    /// Both answers come from memory, under one lock: the send path takes it
    /// exactly once and waits on nothing, which is the rule this whole design
    /// exists for. Claiming marks the device verified, so a burst of commands
    /// produces one probe.
    pub(super) fn route_and_claim(
        &self,
        id: &DeviceId,
        now: Instant,
        claim: bool,
    ) -> Result<(SocketAddr, bool)> {
        let mut devices = self.devices.lock().map_err(|_| Error::ShutDown)?;
        let tracked = devices
            .get_mut(id)
            .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
        if !tracked.breaker.allows(now) {
            return Err(Error::Unavailable {
                id: id.clone(),
                mode: Mode::Lan,
                state: tracked.breaker.state(),
            });
        }
        let addr = SocketAddr::new(tracked.ip, self.endpoints.control_port);

        let claimed = claim
            && self.verify_interval.is_some_and(|interval| {
                tracked
                    .verified_at
                    .is_none_or(|at| now.duration_since(at) >= interval)
            });
        if claimed {
            tracked.verified_at = Some(now);
        }
        Ok((addr, claimed))
    }

    pub(super) async fn request_status(
        &self,
        id: &DeviceId,
        request: &Encoded,
        timeout: Duration,
    ) -> Result<DeviceStatus> {
        let now = Instant::now();
        let (addr, _) = self.route_and_claim(id, now, false)?;

        // Subscribe before sending, or a reply that arrives first is missed.
        let (mut watcher, send_it) = {
            let mut devices = self.devices.lock().map_err(|_| Error::ShutDown)?;
            let tracked = devices
                .get_mut(id)
                .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
            let watcher = tracked.status.subscribe();
            let send_it = !tracked.probing;
            tracked.probing = true;
            (watcher, send_it)
        };
        watcher.mark_unchanged();

        if send_it {
            let bytes = datagram(request)?;
            if let Err(e) = self.socket.send_to(&bytes, addr).await {
                self.clear_probe(id);
                return Err(e);
            }
        }

        let outcome = tokio::time::timeout(timeout, watcher.changed()).await;
        self.clear_probe(id);

        let unreachable = || Error::Unreachable {
            id: id.clone(),
            endpoint: addr.to_string(),
            timeout_ms: millis(timeout),
        };
        match outcome {
            Ok(Ok(())) => {
                let status = watcher.borrow_and_update().clone();
                self.record(id, true, Instant::now());
                status.ok_or_else(unreachable)
            }
            // The device is gone from the map: nothing left to wait on.
            Ok(Err(_)) => Err(Error::UnknownDevice { id: id.clone() }),
            Err(_elapsed) => {
                self.record(id, false, Instant::now());
                Err(unreachable())
            }
        }
    }

    fn clear_probe(&self, id: &DeviceId) {
        if let Ok(mut devices) = self.devices.lock()
            && let Some(tracked) = devices.get_mut(id)
        {
            tracked.probing = false;
        }
    }

    /// Feed the breaker and publish the transition, if there was one.
    fn record(&self, id: &DeviceId, answered: bool, now: Instant) {
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
            tracing::info!(%id, from = %transition.from, to = %transition.to, "lan health changed");
            let _ = self.events.send(Event::HealthChanged {
                id: id.clone(),
                mode: Mode::Lan,
                transition,
            });
        }
    }
}

pub(super) fn datagram(command: &Encoded) -> Result<Vec<u8>> {
    command.to_bytes().map_err(|e| Error::Serialize {
        cmd: command.cmd.clone(),
        reason: e.to_string(),
    })
}
