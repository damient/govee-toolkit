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
//! async runtime, no `tokio`. That is the build a hand-written port checks
//! itself against.
//!
//! # What "decides" means here
//!
//! Narrow, and deliberately so (`docs/modes.md`):
//!
//! - The mode is picked from **breaker state already known**, before anything
//!   is encoded. It is never picked by trying one and waiting for a timeout.
//! - A device with one enabled mode is reached over that mode or not at all.
//!   Unreachable is an error, never a quiet switch to something else.
//! - A command the chosen mode does not carry **fails**. It is not approximated
//!   with a command that mode does have.
//! - Every command reports which mode served it, and every health transition is
//!   an event.
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
