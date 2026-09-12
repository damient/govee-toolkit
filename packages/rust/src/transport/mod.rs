//! What every mode has in common: one trait, one identity, one error.
//!
//! A transport carries the bytes [`crate::codec`] produced to one device and
//! reports what came back. It must **not** choose a mode: this layer has no
//! fallback, and a device it cannot reach produces an error. The facade
//! decides what to do about that, from the user's explicit list of modes —
//! `docs/modes.md`.

pub mod breaker;
pub mod error;
pub mod events;
pub(crate) mod registry;
pub mod reply;
pub mod status;

#[cfg(test)]
pub(crate) mod arbitrary;

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
pub use breaker::{Breaker, Policy, State, Transition};
pub use error::{Error, Result};
pub use events::{Change, Discovered, Event, Health, KnownDevice, Sent};
pub use reply::Reply;
use serde::{Deserialize, Serialize};
pub use status::DeviceStatus;
use tokio::sync::{broadcast, watch};

use crate::codec::{Encoded, Mode};

/// A duration as whole milliseconds, saturating rather than wrapping.
pub(crate) fn millis(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// What to do about a command once it has been written out.
#[derive(Debug, Clone)]
pub enum Verify {
    /// Nothing. The breaker learns nothing from this command, which is right
    /// for a stream of frames: verification traffic would compete with them.
    None,
    /// Ask for the device's status afterwards, and record the answer or its
    /// absence against the breaker. The caller supplies the request, shared
    /// rather than copied.
    With(Arc<Encoded>),
}

/// One way of reaching devices. Every method answers for its own mode.
///
/// The read-only methods must answer from recorded state and touch no adapter:
/// choosing a mode by a trial would cost the fast path a round-trip.
#[async_trait]
pub trait Transport: Debug + Send + Sync + 'static {
    /// The mode this transport serves.
    fn mode(&self) -> Mode;

    /// Subscribe to what it reports.
    fn events(&self) -> broadcast::Receiver<Event>;

    /// Every device it knows, from a scan or from its cache.
    fn devices(&self) -> Vec<KnownDevice>;

    /// The SKU a device reports, if it is known.
    fn sku(&self, id: &DeviceId) -> Option<String>;

    /// A device's health in this mode, if it is known.
    fn health(&self, id: &DeviceId) -> Option<Health>;

    /// The last status heard from a device, without asking for a new one.
    fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus>;

    /// Watch a device's status as answers arrive.
    ///
    /// A subscription requests nothing; use [`Transport::status`] for that.
    fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>>;

    /// How long a scan on this mode must listen. A property of the wire: a
    /// window taken from another mode reports a device that is there as
    /// absent.
    fn scan_window(&self) -> Duration;

    /// Look for devices for `window`, and return what answered.
    ///
    /// Nothing on the send path calls this. [`Transport::scan_window`] is the
    /// window for this mode; pass another only to ask for a shorter or a longer
    /// one deliberately.
    ///
    /// # Errors
    ///
    /// Whatever the mode's discovery can fail with.
    async fn scan(&self, window: Duration) -> Result<Vec<Discovered>>;

    /// Look for one device, and answer as soon as it is found.
    ///
    /// What it reports for that device is what [`Transport::scan`] reports.
    /// The window is the longest this waits, not how long it takes: a mode
    /// that hears devices one by one returns at the first match. A mode whose
    /// discovery answers for every device at once returns when that answer is
    /// in, because there is nothing earlier to wait for.
    ///
    /// Nothing on the send path calls this.
    ///
    /// # Errors
    ///
    /// As for [`Transport::scan`].
    async fn scan_for(&self, id: &DeviceId, window: Duration) -> Result<Option<Discovered>>;

    /// Write a command out.
    ///
    /// Returns as soon as the bytes are gone. A successful return means the
    /// device got the command, never that it applied it. [`Verify::With`]
    /// answers that question.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if nothing is known under this identity,
    /// [`Error::Unavailable`] if the breaker refuses this mode right now, or
    /// [`Error::Io`] if the write fails.
    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify) -> Result<Sent>;

    /// Ask a device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`Transport::send`], plus [`Error::Unreachable`] if nothing
    /// answers in time.
    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus>;

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    ///
    /// How a value the SDK does not model reaches a caller — see [`Reply`].
    ///
    /// # Errors
    ///
    /// As for [`Transport::status`], plus [`Error::NoReplyLayout`] where the
    /// command declares no reply to read, or where the mode's replies are not
    /// frames at all.
    async fn read(&self, id: &DeviceId, request: &Encoded) -> Result<Reply>;

    /// Release what this transport holds, before it is dropped.
    ///
    /// A mode whose wire takes no acknowledgement holds the link open here,
    /// long enough for the last frame to leave. The default does nothing,
    /// which is right for a socket the operating system owns. A transport
    /// answers commands again after this call.
    ///
    /// # Errors
    ///
    /// Whatever the mode reports.
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// A device's identity: the MAC address it reports.
///
/// Not its address — a DHCP lease renews, and the same unit answers on an
/// unrelated Bluetooth address. Everything keys on the MAC, so one
/// configuration entry covers a device reachable over several modes.
/// [`DeviceId::new`] normalizes to uppercase: firmwares are not consistent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct DeviceId(String);

impl DeviceId {
    /// Normalize a reported identity.
    #[must_use]
    pub fn new(raw: impl AsRef<str>) -> Self {
        Self(raw.as_ref().trim().to_uppercase())
    }

    /// The normalized form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DeviceId {
    fn from(raw: String) -> Self {
        Self::new(raw)
    }
}

impl From<&str> for DeviceId {
    fn from(raw: &str) -> Self {
        Self::new(raw)
    }
}

impl From<DeviceId> for String {
    fn from(id: DeviceId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn one_device_has_one_identity_whatever_the_firmware_reports() {
        assert_eq!(
            DeviceId::new("aa:bb:cc:dd:ee:ff"),
            DeviceId::new(" AA:BB:CC:DD:EE:FF ")
        );
    }

    #[test]
    fn survives_a_json_round_trip() {
        let id = DeviceId::new("aa:bb:cc:dd:ee:ff");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"AA:BB:CC:DD:EE:FF\"");
        assert_eq!(
            serde_json::from_str::<DeviceId>("\"aa:bb:cc:dd:ee:ff\"").expect("deserialize"),
            id
        );
    }
}
