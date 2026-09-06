//! What every mode has in common: one trait, one identity, one error.
//!
//! A transport carries the bytes [`crate::codec`] produced to one device and
//! reports what came back. It does **not** choose a mode: there is nothing to
//! fall back to at this layer. A device that cannot be reached produces an
//! error, and what to do about it is the facade's decision, made from the
//! user's configuration — `docs/modes.md`.
//!
//! The trait exists because the facade would otherwise ask the same question in
//! six places, once per mode. It removes that repetition and nothing else:
//! which transports a device may use stays the user's explicit list.

pub mod breaker;
pub mod error;
pub mod events;
pub mod status;

#[cfg(test)]
pub(crate) mod arbitrary;

use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
pub use breaker::{Breaker, Policy, State, Transition};
pub use error::{Error, Result};
pub use events::{Change, Discovered, Event, Health, KnownDevice, Sent};
use serde::{Deserialize, Serialize};
pub use status::DeviceStatus;
use tokio::sync::{broadcast, watch};

use crate::codec::{Encoded, Mode};

/// A duration as whole milliseconds, saturating rather than wrapping.
pub(crate) fn millis(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// What to do about a command once it has been written out.
#[derive(Debug, Clone, Copy)]
pub enum Verify<'a> {
    /// Nothing. The breaker learns nothing from this command — right for a
    /// stream of frames, where the verification traffic would compete with the
    /// frames themselves.
    None,
    /// Ask the device for its status afterwards, and record the answer, or its
    /// absence, against the breaker. The request is supplied by the caller
    /// because building it means reading the device file, which is the codec's
    /// job and not a transport's.
    With(&'a Encoded),
}

/// One way of reaching devices.
///
/// Implemented once per mode. Every method answers for the mode this transport
/// serves and for no other; the facade is what holds several of them and picks
/// between them.
///
/// The read-only methods answer from state already recorded and must touch no
/// adapter: choosing a mode by trying one would cost the fast path a
/// round-trip on every command.
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
    /// Nothing is requested by subscribing; use [`Transport::status`] for that.
    fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>>;

    /// Look for devices for `window`, and return what answered.
    ///
    /// Nothing on the send path calls this.
    ///
    /// # Errors
    ///
    /// Whatever the mode's discovery can fail with.
    async fn scan(&self, window: Duration) -> Result<Vec<Discovered>>;

    /// Write a command out.
    ///
    /// Returns as soon as the bytes are gone. A successful return means the
    /// command was sent, never that it was applied — which is what
    /// [`Verify::With`] is for.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if nothing is known under this identity,
    /// [`Error::Unavailable`] if the breaker refuses this mode right now, or
    /// [`Error::Io`] if the write fails.
    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent>;

    /// Ask a device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`Transport::send`], plus [`Error::Unreachable`] if nothing
    /// answers in time.
    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus>;

    /// Write the device cache out.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] if the file cannot be written.
    fn save_cache(&self) -> Result<()>;
}

/// A device's identity: the MAC address it reports.
///
/// Addresses are not identity — a DHCP lease renews and the device is at a
/// different one, still the same device, and the same unit answers on a
/// Bluetooth address that has nothing to do with either. The cache, the breaker
/// and the user's configuration all key on this instead, which is also what
/// lets one configuration entry cover a device reachable over several modes.
///
/// Normalized to uppercase on construction: firmwares are not consistent about
/// the case they report, and two spellings of one device would otherwise be two
/// devices.
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
