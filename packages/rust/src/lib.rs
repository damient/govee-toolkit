//! Control Govee devices over the LAN, including undocumented commands.
//!
//! Unofficial, and not affiliated with Govee. The protocol is implemented once,
//! here; every other language binds to this crate rather than porting it — see
//! `docs/architecture.md`.
//!
//! # Layers
//!
//! - [`codec`] — `devices/*.yaml` in, exact bytes out. No I/O, no SKU name, no
//!   command name. Available with no features enabled at all.
//! - [`lan`] — the UDP transport: discovery, a device cache, one shared socket
//!   and a per-device circuit breaker. Behind the `lan` feature, on by default.
//! - [`stream`] — the raw segment channel, armed once and fed frames at a rate
//!   taken from what was measured on the device.
//! - The facade, at the crate root — configuration, mode selection and events.
//!
//! `ble` and `cloud` are declared modes with no transport yet. Enabling one is
//! reported as such; it is never silently skipped, and never substituted with
//! another mode.
//!
//! # Features
//!
//! - `lan` *(default)* — the UDP transport and the facade above it.
//!
//! With default features off, what remains is the codec alone: no socket, no
//! async runtime, no `tokio`. Every binding encodes through that build, so it
//! is the one the conformance vectors pin.
//!
//! # Choosing a mode
//!
//! The mode is picked from breaker state already known, before anything is
//! encoded — never by trying one and waiting for a timeout. It is picked from
//! the modes the user enabled for that device and from nothing else: a device
//! that cannot be reached is an error, and a command the chosen mode does not
//! carry fails rather than being approximated. Every command reports which
//! mode served it. The rules are `docs/modes.md`.
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

#[cfg(feature = "lan")]
pub mod lan;

// The facade below needs a transport to be worth compiling. When `ble` and
// `cloud` land, every `feature = "lan"` here becomes `any(feature = "lan", …)`.
#[cfg(feature = "lan")]
pub mod config;
#[cfg(feature = "lan")]
pub mod error;
#[cfg(feature = "lan")]
pub mod paths;
#[cfg(feature = "lan")]
pub mod stream;

#[cfg(feature = "lan")]
mod device;
#[cfg(feature = "lan")]
mod event;
#[cfg(feature = "lan")]
mod govee;

pub use codec::{Args, Catalog, Mode};
#[cfg(feature = "lan")]
pub use config::{Config, DeviceConfig, LanConfig, Problem};
#[cfg(feature = "lan")]
pub use device::DeviceHandle;
#[cfg(feature = "lan")]
pub use error::{Error, Result};
#[cfg(feature = "lan")]
pub use event::{Device, Event, Served};
#[cfg(feature = "lan")]
pub use govee::Govee;
#[cfg(feature = "lan")]
pub use lan::{DeviceId, DeviceStatus, Health, State};
#[cfg(feature = "lan")]
pub use stream::{Rate, SegmentStream, StreamOptions, Zones};
