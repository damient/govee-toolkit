//! Control Govee devices over the LAN or Bluetooth, including undocumented
//! commands.
//!
//! Unofficial, and not affiliated with Govee. The protocol is implemented once,
//! here; every other language binds to this crate — see `docs/architecture.md`.
//!
//! # Layers
//!
//! - [`codec`] — `devices/*.yaml` in, exact bytes out. No I/O, no SKU name, no
//!   command name.
//! - [`transport`] — what every mode shares: the `Transport` trait, the device
//!   identity, the breaker and the errors.
//! - [`lan`] — UDP: discovery, a device cache, one shared socket.
//! - [`ble`] — GATT: one connection per device, and paced writes.
//! - [`stream`] — the raw segment channel, armed once and fed frames.
//! - The facade, at the crate root — configuration, mode selection and events.
//!
//! # Features
//!
//! `lan` is on by default and `ble` is opt-in. With both off, what remains is
//! the codec alone: no socket and no async runtime. Every binding encodes
//! through that build, so it is the one the conformance vectors pin.
//!
//! # Choosing a mode
//!
//! The SDK picks among the modes the user enabled for that device, from
//! breaker state it already holds. It never sends a trial command and waits
//! for a timeout. A device no enabled mode reaches is an error, and a command
//! the chosen mode does not carry fails rather than being approximated.
//! `cloud` has no transport yet and is reported as such. Every command reports
//! which mode served it — `docs/modes.md`.
//!
//! ```no_run
//! use govee_toolkit::{Args, Config, Govee};
//!
//! # async fn example() -> Result<(), govee_toolkit::Error> {
//! let govee = Govee::start(Config::load()?).await?;
//! govee.scan().await?;
//!
//! for device in govee.devices() {
//!     let served = govee
//!         .device(&device.id)
//!         .send("power", &Args::new().int("on", 1))
//!         .await?;
//!     println!("{} served by {}", device.id, served.mode);
//! }
//! # Ok(())
//! # }
//! ```

pub mod codec;

#[cfg(feature = "ble")]
pub mod ble;
#[cfg(feature = "lan")]
pub mod lan;

// The facade needs a transport, but not a particular one. Every gate here names
// the modes that carry one, so `cloud` joins by widening the list.
#[cfg(any(feature = "lan", feature = "ble"))]
pub mod config;
#[cfg(any(feature = "lan", feature = "ble"))]
pub mod error;
#[cfg(any(feature = "lan", feature = "ble"))]
pub mod paths;
#[cfg(any(feature = "lan", feature = "ble"))]
pub mod stream;
#[cfg(any(feature = "lan", feature = "ble"))]
pub mod transport;

#[cfg(any(feature = "lan", feature = "ble"))]
mod device;
#[cfg(any(feature = "lan", feature = "ble"))]
mod event;
#[cfg(any(feature = "lan", feature = "ble"))]
mod govee;

pub use codec::{Args, Catalog, Mode};
#[cfg(any(feature = "lan", feature = "ble"))]
pub use config::{Config, DeviceConfig, LanConfig, Problem, StreamConfig};
#[cfg(any(feature = "lan", feature = "ble"))]
pub use device::DeviceHandle;
#[cfg(any(feature = "lan", feature = "ble"))]
pub use error::{Error, Result};
#[cfg(any(feature = "lan", feature = "ble"))]
pub use event::{Device, Event, Served};
#[cfg(any(feature = "lan", feature = "ble"))]
pub use govee::Govee;
#[cfg(any(feature = "lan", feature = "ble"))]
pub use stream::{Rate, SegmentStream, StreamOptions, Zones};
#[cfg(any(feature = "lan", feature = "ble"))]
pub use transport::{DeviceId, DeviceStatus, Health, Reply, State, Transport};
