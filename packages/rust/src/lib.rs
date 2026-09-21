//! Control Govee devices over the LAN, over Bluetooth or through the cloud,
//! including undocumented commands.
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
//! - [`cloud`] — the documented HTTPS API: any device the account owns,
//!   throttled.
//! - [`stream`] — the raw segment channel, armed once and fed frames.
//! - The facade, at the crate root — configuration, mode selection and events.
//!
//! # Features
//!
//! `lan` is on by default; `ble` and `cloud` are opt-in. With all three off,
//! what remains is the codec alone: no socket and no async runtime. Every
//! binding encodes through that build, so it is the one the conformance
//! vectors pin.
//!
//! # Choosing a mode
//!
//! The SDK picks among the modes the user enabled for that device, from
//! breaker state it already holds. It never sends a trial command and waits
//! for a timeout. A device no enabled mode reaches is an error, and a command
//! the chosen mode does not carry fails rather than being approximated.
//! Every command reports which mode served it — `docs/modes.md`.
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

// `doc(cfg(...))` labels each item with the feature that carries it. It is a
// nightly rustdoc feature, and `docsrs` is set by the docs.rs build alone.
#![cfg_attr(docsrs, feature(doc_cfg))]

/// The version of this crate. A binding reports it as the core it was built
/// from, so no binding carries a copy of the number.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod codec;

#[cfg(feature = "ble")]
#[cfg_attr(docsrs, doc(cfg(feature = "ble")))]
pub mod ble;
#[cfg(feature = "cloud")]
#[cfg_attr(docsrs, doc(cfg(feature = "cloud")))]
pub mod cloud;
#[cfg(feature = "lan")]
#[cfg_attr(docsrs, doc(cfg(feature = "lan")))]
pub mod lan;

#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod config;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod env;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod error;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod paths;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod stream;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod summary;
#[cfg(feature = "transport")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport")))]
pub mod transport;

#[cfg(feature = "transport")]
mod describe;
#[cfg(feature = "transport")]
mod device;

#[cfg(feature = "transport")]
mod event;
#[cfg(feature = "transport")]
mod govee;
#[cfg(feature = "ble")]
mod provision;
#[cfg(feature = "transport")]
mod resolved;
#[cfg(feature = "transport")]
pub mod select;
#[cfg(feature = "transport")]
mod verbs;

pub use codec::{Args, Catalog, Mode};
#[cfg(feature = "transport")]
pub use config::{CloudConfig, Config, DeviceConfig, LanConfig, Problem, StreamConfig};
#[cfg(feature = "transport")]
pub use describe::describe;
#[cfg(feature = "transport")]
pub use device::DeviceHandle;
#[cfg(feature = "transport")]
pub use env::Env;
#[cfg(feature = "transport")]
pub use error::{Category, Error, Result};
#[cfg(feature = "transport")]
pub use event::{Device, Event, Served};
#[cfg(feature = "transport")]
pub use govee::{Govee, Walk, WalkObserver, WalkReport};
#[cfg(feature = "ble")]
pub use provision::{Provisioned, WifiCredentials};
#[cfg(feature = "transport")]
pub use resolved::Resolved;
#[cfg(feature = "transport")]
pub use select::Selector;
#[cfg(feature = "transport")]
pub use stream::{ParseError, Rate, Reach, Resolution, SegmentStream, StreamOptions};
#[cfg(feature = "transport")]
pub use summary::{Style, Summary};
#[cfg(feature = "transport")]
pub use transport::{DeviceId, DeviceStatus, Health, Reply, State, Transport};
#[cfg(feature = "transport")]
pub use verbs::{IDENTIFY_COLOR, IDENTIFY_WAIT, Identify, Music, Paint};
