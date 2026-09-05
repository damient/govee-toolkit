//! The `lan` transport: UDP on the local network.
//!
//! Carries the bytes [`crate::codec`] produces. It discovers devices,
//! remembers where they are, keeps one socket for everything and tracks each
//! device's health — and it does exactly that, for one mode.
//!
//! Three rules shape it, all from `docs/modes.md`:
//!
//! - **It never chooses a mode.** There is nothing to fall back to here. A
//!   device that cannot be reached produces an error, and what to do about it
//!   is the facade's decision, made from the user's configuration.
//! - **Health is state already known.** [`Transport::send`] never waits for a
//!   timeout to decide whether to send; the breaker answers from what the last
//!   commands did.
//! - **Nothing is approximated.** Ranges are already enforced by the codec, and
//!   this layer does not soften them.
//!
//! ```no_run
//! use govee_toolkit::codec::{self, Args, Catalog, Mode};
//! use govee_toolkit::lan::{DeviceId, Options, Transport, Verify};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let catalog = Catalog::embedded()?;
//! let transport = Transport::start(Options::default()).await?;
//! transport.scan(std::time::Duration::from_secs(2)).await?;
//!
//! let id = DeviceId::new("aa:bb:cc:dd:ee:ff");
//! let device = catalog.device(&transport.sku(&id).unwrap_or_default())?;
//! let on = codec::encode(device, Mode::Lan, "power", &Args::new().int("on", 1))?;
//! transport.send(&id, &on, Verify::None).await?;
//! # Ok(())
//! # }
//! ```

pub mod breaker;
pub mod cache;
pub mod discovery;
pub mod error;
pub mod status;
pub mod transport;

mod socket;

#[cfg(test)]
mod arbitrary;

pub use breaker::{Breaker, Policy, State, Transition};
pub use cache::{Cache, CachedDevice, Change};
pub use discovery::{DiscoveredDevice, Endpoints};
pub use error::{Error, Result};
use serde::{Deserialize, Serialize};
pub use status::DeviceStatus;
pub use transport::{Event, Health, KnownDevice, Options, Sent, Transport, Verify};

/// A device's identity: the MAC address it reports in a `scan` reply.
///
/// Addresses are not identity — a DHCP lease renews and the device is at a
/// different one, still the same device. The cache, the breaker and the user's
/// configuration all key on this instead.
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
