//! Control Govee devices over the LAN or Bluetooth, including undocumented
//! commands.
//!
//! Unofficial, and not affiliated with Govee. The protocol is implemented once,
//! here; every other language binds to this crate rather than porting it — see
//! `docs/architecture.md`.
//!
//! # Layers
//!
//! - [`codec`] — `devices/*.yaml` in, exact bytes out. No I/O, no SKU name, no
//!   command name. Available with no features enabled.
//! - [`transport`] — what every mode has in common: the `Transport` trait, the
//!   device identity, the circuit breaker and the errors.
//! - [`lan`] — the UDP transport: discovery, a device cache, one shared socket
//!   and a per-device circuit breaker. Behind the `lan` feature, on by default.
//! - [`ble`] — the GATT transport: one connection per device, a paced write
//!   budget and the same per-device breaker. Behind the `ble` feature.
//! - [`stream`] — the raw segment channel. It arms once, then takes frames at
//!   the rate the device file records.
//! - The facade, at the crate root — configuration, mode selection and events.
//!
//! `cloud` is a declared mode with no transport yet. The SDK reports it as
//! such: it never skips the mode in silence, and never substitutes another one.
//!
//! # Features
//!
//! - `lan` *(default)* — the UDP transport and the facade above it.
//! - `ble` — the GATT transport, and the facade above it.
//!
//! With default features off, what remains is the codec alone: no socket, no
//! async runtime, no `tokio`. Every binding encodes through that build, so it
//! is the one the conformance vectors pin.
//!
//! # Choosing a mode
//!
//! The SDK chooses the mode from breaker state it already holds, before it
//! encodes anything — never by a trial send that waits for a timeout. It
//! chooses among the modes the user enabled for that device and nothing else:
//! modes are explicit, never a fallback chain. A device it cannot reach is an
//! error. A command the chosen mode does not carry fails, and the SDK never
//! approximates it. Every command reports which mode served it. The rules are
//! `docs/modes.md`.
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
