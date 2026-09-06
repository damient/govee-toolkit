//! The scan, the link, the budget and the breaker, tied together.
//!
//! The send path is the same shape as every other mode's: resolve the device
//! from what is already known, ask the breaker, write. Between the last two
//! this mode adds a connection, because a device answers nothing until one is
//! up, and a write budget the firmware imposes.
//!
//! A reply carries no request id, so a read waits under an open subscription
//! and matches the `reply:` layout the device file declares.

mod discover;
mod impl_transport;
mod options;
mod read;
mod shared;

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};

pub use self::options::Options;
use self::shared::Shared;
#[cfg(test)]
use self::shared::Tracked;
use crate::ble::pace::Budget;
use crate::codec::{Encoded, Mode};
use crate::transport::error::{Error, Result};
use crate::transport::events::health_of;
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, Discovered, Event, Health, KnownDevice, Reply, Sent, Verify};

/// The `ble` transport.
///
/// Cheap to clone; every clone shares one adapter, one set of connections and
/// one set of breakers.
#[derive(Clone)]
pub struct Transport {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport").finish_non_exhaustive()
    }
}

impl Transport {
    /// Build the transport.
    ///
    /// Claims no adapter. A scan or a command opens one, so this succeeds on a
    /// machine whose radio is off, and the first command reports it.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if the write budget cannot be honoured — see
    /// [`Budget::new`](crate::ble::Budget::new).
    pub fn start(options: Options) -> Result<Self> {
        let budget = Budget::new(options.writes_per_second, options.burst)?;
        let (events, _) = broadcast::channel(256);
        Ok(Self {
            shared: Arc::new(Shared::new(options, budget, events)),
        })
    }

    /// Subscribe to transport events.
    #[must_use]
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.shared.events.subscribe()
    }

    /// Listen for advertisements and record what answered.
    ///
    /// Nothing on the send path calls this. This transport reports a device
    /// under the identity it can observe — see [`Transport::bind`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if no adapter is available, or if the scan cannot be
    /// started.
    pub async fn scan(&self, window: Duration) -> Result<Vec<Discovered>> {
        Shared::scan(&self.shared, window).await
    }

    /// Say that the device known by `id` is the one answering at `endpoint`.
    ///
    /// This crate identifies a device by its Wi-Fi MAC, and a scan reports the
    /// handle the platform addresses the peripheral by. The two name one unit,
    /// and **nothing here infers one from the other**: a scan reports a device
    /// under that handle until a caller says otherwise. One configuration
    /// entry can then cover a device reachable over both `lan` and `ble`.
    ///
    /// The device keeps everything already recorded about it, health included.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if no scan has heard that handle.
    pub fn bind(&self, id: &DeviceId, endpoint: &str) -> Result<()> {
        let mut devices = self.shared.devices.lock().map_err(|_| Error::ShutDown)?;
        let known = devices.iter().find_map(|(known, tracked)| {
            tracked
                .endpoint
                .eq_ignore_ascii_case(endpoint)
                .then(|| known.clone())
        });
        let known = known.ok_or_else(|| Error::UnknownDevice {
            id: DeviceId::new(endpoint),
        })?;
        if &known == id {
            return Ok(());
        }
        if let Some(tracked) = devices.remove(&known) {
            devices.insert(id.clone(), tracked);
        }
        Ok(())
    }

    /// Every device the transport knows, from a scan.
    ///
    /// Empty until a scan has run: only an advertisement relates a device to a
    /// handle, and this mode caches nothing across restarts.
    #[must_use]
    pub fn devices(&self) -> Vec<KnownDevice> {
        let now = Instant::now();
        let Ok(devices) = self.shared.devices.lock() else {
            return Vec::new();
        };
        let mut out: Vec<KnownDevice> = devices
            .iter()
            .map(|(id, tracked)| KnownDevice {
                id: id.clone(),
                endpoint: tracked.endpoint.clone(),
                sku: tracked.sku.clone(),
                health: health_of(&tracked.breaker, now),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The SKU a device advertises, if it is known.
    #[must_use]
    pub fn sku(&self, id: &DeviceId) -> Option<String> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| d.sku.clone())
    }

    /// A device's health in this mode, if it is known.
    #[must_use]
    pub fn health(&self, id: &DeviceId) -> Option<Health> {
        let now = Instant::now();
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| health_of(&d.breaker, now))
    }

    /// Watch a device's status as answers arrive.
    ///
    /// Nothing is requested by subscribing; use [`Transport::status`] for that.
    #[must_use]
    pub fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| d.status.subscribe())
    }

    /// The last status heard from a device, without asking for a new one.
    #[must_use]
    pub fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).and_then(|d| d.status.borrow().clone())
    }

    /// Write a command out, one frame at a time and at the device's budget.
    ///
    /// Returns once the last frame is written. The characteristic takes writes
    /// without a response, so a successful return means the frames went out,
    /// never that the device applied them. [`Verify::With`] is for that.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if no scan has heard this device,
    /// [`Error::Unavailable`] if the breaker refuses this mode right now,
    /// [`Error::Serialize`] if the command carries no frames, or [`Error::Io`]
    /// if the connection or a write fails.
    pub async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent> {
        let route =
            self.shared
                .route_and_claim(id, Instant::now(), matches!(verify, Verify::With(_)))?;
        let link = match self.shared.link(id, &route.endpoint).await {
            Ok(link) => link,
            Err(e) => {
                // A device that will not take a connection is unreachable. A
                // record now spares the next command the same wait.
                self.shared.record(id, false, Instant::now());
                return Err(e);
            }
        };
        self.shared.write_frames(id, &route, &link, command).await?;

        let sent = Sent {
            id: id.clone(),
            mode: Mode::Ble,
            cmd: command.cmd.clone(),
            endpoint: route.endpoint.clone(),
        };
        // Cloning `Sent` allocates twice; with nobody listening the broadcast
        // would drop it straight away.
        if self.shared.events.receiver_count() > 0 {
            let _ = self.shared.events.send(Event::Sent(sent.clone()));
        }

        if let Verify::With(request) = verify
            && route.verifying
        {
            let shared = Arc::clone(&self.shared);
            let id = id.clone();
            let request = request.clone();
            let timeout = self.shared.options.status_timeout;
            tokio::spawn(async move {
                // The breaker already holds the result and the event stream
                // already carries it.
                let _ = shared.request_status(&id, &request, timeout).await;
            });
        }

        Ok(sent)
    }

    /// Ask a device for its state and wait for the answer.
    ///
    /// The breaker records the answer or the silence. The device file says
    /// what the reply means: its `reply:` layouts say which bytes carry what,
    /// and the roles on its arguments say which of those the SDK models.
    /// [`DeviceStatus::raw`] keeps everything captured.
    ///
    /// # Errors
    ///
    /// As for [`Transport::send`], plus [`Error::NoReplyLayout`] if the status
    /// command declares no reply to read, and [`Error::Unreachable`] if nothing
    /// answers in time.
    pub async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        self.shared
            .request_status(id, request, self.shared.options.status_timeout)
            .await
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    ///
    /// # Errors
    ///
    /// As for [`Transport::status`].
    pub async fn read(&self, id: &DeviceId, request: &Encoded) -> Result<Reply> {
        self.shared
            .read(id, request, self.shared.options.status_timeout)
            .await
    }

    /// Record a device the caller already knows how to reach.
    #[cfg(test)]
    pub(crate) fn insert(&self, id: &DeviceId, endpoint: &str, sku: &str) {
        if let Ok(mut devices) = self.shared.devices.lock() {
            devices.insert(
                id.clone(),
                Tracked::new(
                    endpoint.to_owned(),
                    sku.to_owned(),
                    &self.shared.options,
                    self.shared.budget,
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn transport() -> Transport {
        Transport::start(Options::default()).expect("the default budget is usable")
    }

    #[test]
    fn a_device_nothing_has_heard_is_not_a_device() {
        let ble = transport();
        let id = DeviceId::new("aa:bb:cc:dd:ee:ff");
        assert!(ble.devices().is_empty());
        assert!(ble.sku(&id).is_none());
        assert!(ble.health(&id).is_none());
        assert!(ble.last_status(&id).is_none());
    }

    #[test]
    fn binding_says_which_identity_an_address_belongs_to() {
        let ble = transport();
        let discovered = DeviceId::new("11:22:33:44:55:66");
        ble.insert(&discovered, "11:22:33:44:55:66", "H0004");

        let wifi = DeviceId::new("aa:bb:cc:dd:ee:ff");
        ble.bind(&wifi, "11:22:33:44:55:66").expect("it was heard");

        assert_eq!(ble.sku(&wifi).as_deref(), Some("H0004"));
        assert!(ble.sku(&discovered).is_none());
        assert_eq!(ble.devices()[0].endpoint, "11:22:33:44:55:66");
    }

    #[test]
    fn a_write_budget_nothing_could_be_sent_under_is_refused() {
        let options = Options {
            writes_per_second: 0.0,
            ..Options::default()
        };
        let error = Transport::start(options).expect_err("a rate of zero is not a budget");
        assert_eq!(error.code(), "out_of_range");
    }

    #[test]
    fn an_address_nothing_has_heard_cannot_be_bound() {
        let ble = transport();
        let error = ble
            .bind(&DeviceId::new("aa:bb:cc:dd:ee:ff"), "11:22:33:44:55:66")
            .expect_err("nothing to relate it to");
        assert_eq!(error.code(), "unknown_device");
    }
}
