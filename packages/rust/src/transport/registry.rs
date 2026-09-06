//! The per-device table every transport keeps, and the answers it builds.
//!
//! A mode adds its own fields to a record — an address, a connection, a write
//! budget — and this layer reads none of them. What it does own is the part
//! every mode repeats: the breaker gate, the verification claim, and the three
//! things a transport publishes.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};

use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::breaker::Breaker;
use crate::transport::error::{Error, Result};
use crate::transport::events::{Event, Health, KnownDevice, Sent, health_of};
use crate::transport::status::DeviceStatus;

/// What a transport's per-device record exposes to be tracked here.
pub(crate) trait Tracked {
    /// The SKU the device reports.
    fn sku(&self) -> &str;
    /// Its breaker.
    fn breaker(&self) -> &Breaker;
    /// Its breaker, to feed.
    fn breaker_mut(&mut self) -> &mut Breaker;
    /// The channel its status is published on.
    fn status(&self) -> &watch::Sender<Option<DeviceStatus>>;
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    fn verified_at(&mut self) -> &mut Option<Instant>;
}

/// Every device one transport tracks.
pub(crate) struct Devices<T>(Mutex<HashMap<DeviceId, T>>);

impl<T> Devices<T> {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// The table itself, for what only one mode does with it.
    ///
    /// # Errors
    ///
    /// [`Error::ShutDown`] if the lock is poisoned.
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, HashMap<DeviceId, T>>> {
        self.0.lock().map_err(|_| Error::ShutDown)
    }
}

impl<T: Tracked> Devices<T> {
    /// Every device, sorted by identity. The transport says how to spell an
    /// endpoint, because only it knows what one is.
    pub(crate) fn known(&self, endpoint_of: impl Fn(&T) -> String) -> Vec<KnownDevice> {
        let now = Instant::now();
        let Ok(devices) = self.lock() else {
            return Vec::new();
        };
        let mut out: Vec<KnownDevice> = devices
            .iter()
            .map(|(id, tracked)| KnownDevice {
                id: id.clone(),
                endpoint: endpoint_of(tracked),
                sku: tracked.sku().to_owned(),
                health: health_of(tracked.breaker(), now),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub(crate) fn sku(&self, id: &DeviceId) -> Option<String> {
        Some(self.lock().ok()?.get(id)?.sku().to_owned())
    }

    pub(crate) fn health(&self, id: &DeviceId) -> Option<Health> {
        let now = Instant::now();
        Some(health_of(self.lock().ok()?.get(id)?.breaker(), now))
    }

    pub(crate) fn watch_status(
        &self,
        id: &DeviceId,
    ) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        Some(self.lock().ok()?.get(id)?.status().subscribe())
    }

    pub(crate) fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        self.lock().ok()?.get(id)?.status().borrow().clone()
    }

    /// Where to send, and whether this command pays for a verification.
    ///
    /// Both answers come from memory, under one lock: the send path takes it
    /// exactly once and waits on nothing. `claim` carries the verification
    /// interval when the command may claim one, and `None` when it may not. A
    /// claim marks the device verified, so a burst of commands produces one
    /// probe.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if nothing is tracked under this identity, or
    /// [`Error::Unavailable`] if the breaker refuses this mode right now.
    pub(crate) fn route_and_claim<R>(
        &self,
        id: &DeviceId,
        mode: Mode,
        now: Instant,
        claim: Option<Duration>,
        route_of: impl FnOnce(&T) -> R,
    ) -> Result<(R, bool)> {
        let mut devices = self.lock()?;
        let tracked = devices
            .get_mut(id)
            .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
        if !tracked.breaker().allows(now) {
            return Err(Error::Unavailable {
                id: id.clone(),
                mode,
                state: tracked.breaker().state(),
            });
        }

        let claimed = claim.is_some_and(|interval| {
            tracked
                .verified_at()
                .is_none_or(|at| now.duration_since(at) >= interval)
        });
        if claimed {
            *tracked.verified_at() = Some(now);
        }
        Ok((route_of(tracked), claimed))
    }

    /// Feed the breaker and publish the transition, if there was one.
    pub(crate) fn record(
        &self,
        events: &broadcast::Sender<Event>,
        id: &DeviceId,
        mode: Mode,
        answered: bool,
        now: Instant,
    ) {
        let transition = {
            let Ok(mut devices) = self.lock() else {
                return;
            };
            let Some(tracked) = devices.get_mut(id) else {
                return;
            };
            if answered {
                tracked.breaker_mut().record_success(now)
            } else {
                tracked.breaker_mut().record_failure(now)
            }
        };
        if transition.changed() {
            tracing::info!(%id, %mode, from = %transition.from, to = %transition.to, "health changed");
            let _ = events.send(Event::HealthChanged {
                id: id.clone(),
                mode,
                transition,
            });
        }
    }

    /// Hand a status to the device's watchers and to the event stream.
    pub(crate) fn publish_status(
        &self,
        events: &broadcast::Sender<Event>,
        mode: Mode,
        status: DeviceStatus,
    ) {
        if let Ok(devices) = self.lock()
            && let Some(tracked) = devices.get(&status.id)
        {
            let _ = tracked.status().send(Some(status.clone()));
        }
        let _ = events.send(Event::Status { mode, status });
    }
}

/// Publish a command that went out.
///
/// Cloning `Sent` allocates twice; with nobody listening the broadcast would
/// drop it straight away.
pub(crate) fn publish_sent(events: &broadcast::Sender<Event>, sent: &Sent) {
    if events.receiver_count() > 0 {
        let _ = events.send(Event::Sent(sent.clone()));
    }
}
