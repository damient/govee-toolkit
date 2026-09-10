//! The state every [`Transport`](super::Transport) clone shares, and the
//! throttle in front of it.
//!
//! One device gets one request every [`Options::min_interval`]. The gate is
//! taken before the request goes out and released when the slot is claimed,
//! so two callers never spend two slots on one interval.

use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};

use crate::cloud::api::Client;
use crate::codec::Mode;
use crate::transport::breaker::{Breaker, Policy};
use crate::transport::error::{Error, Result};
use crate::transport::events::Event;
use crate::transport::registry::Devices;
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, millis};

/// One device, as the transport tracks it.
pub(super) struct Tracked {
    pub(super) sku: String,
    /// The name the account gave it. Reported, never read as identity.
    pub(super) name: String,
    pub(super) breaker: Breaker,
    pub(super) status: watch::Sender<Option<DeviceStatus>>,
    pub(super) verified_at: Option<Instant>,
    /// When the last request about this device went out.
    pub(super) last_request: Option<Instant>,
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
    pub(super) fn new(sku: String, name: String, policy: Policy) -> Self {
        Self {
            sku,
            name,
            breaker: Breaker::new(policy),
            status: watch::Sender::new(None),
            verified_at: None,
            last_request: None,
        }
    }
}

pub(super) struct Shared {
    pub(super) client: Client,
    pub(super) policy: Policy,
    pub(super) request_timeout: Duration,
    pub(super) min_interval: Duration,
    pub(super) max_wait: Duration,
    pub(super) devices: Devices<Tracked>,
    pub(super) events: broadcast::Sender<Event>,
}

impl Shared {
    /// The SKU to address this device by, after the breaker has allowed it and
    /// the throttle has been paid.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if no scan has listed the device,
    /// [`Error::Unavailable`] if the breaker refuses this mode, or
    /// [`Error::RateLimited`] if the slot is further away than
    /// [`Shared::max_wait`].
    pub(super) async fn claim(&self, id: &DeviceId) -> Result<String> {
        let now = Instant::now();
        let (throttled, _) =
            self.devices
                .route_and_claim(id, Mode::Cloud, now, None, |tracked| {
                    let wait = tracked
                        .last_request
                        .map(|at| self.min_interval.saturating_sub(now.duration_since(at)))
                        .unwrap_or_default();
                    if wait > self.max_wait {
                        return Err(Error::RateLimited {
                            mode: Mode::Cloud,
                            retry_after_ms: millis(wait),
                        });
                    }
                    // Claimed here rather than after the wait: a second caller
                    // then queues behind this slot instead of sharing it.
                    tracked.last_request = Some(now + wait);
                    Ok((tracked.sku.clone(), wait))
                })?;
        let (sku, wait) = throttled?;

        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        Ok(sku)
    }

    /// Feed the breaker with what the API answered, and publish the
    /// transition.
    pub(super) fn record(&self, id: &DeviceId, answered: bool) {
        self.devices
            .record(&self.events, id, Mode::Cloud, answered, Instant::now());
    }
}
