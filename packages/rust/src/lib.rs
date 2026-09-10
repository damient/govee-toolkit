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

// The facade needs a transport, but not a particular one: each gate below
// names the modes that carry one.
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "lan", feature = "ble", feature = "cloud")))
)]
pub mod config;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "lan", feature = "ble", feature = "cloud")))
)]
pub mod error;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "lan", feature = "ble", feature = "cloud")))
)]
pub mod paths;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "lan", feature = "ble", feature = "cloud")))
)]
pub mod stream;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "lan", feature = "ble", feature = "cloud")))
)]
pub mod transport;

#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
mod device;

#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
mod event;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
mod govee;
#[cfg(feature = "ble")]
mod provision;

pub use codec::{Args, Catalog, Mode};
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use config::{CloudConfig, Config, DeviceConfig, LanConfig, Problem, StreamConfig};
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use device::DeviceHandle;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use error::{Error, Result};
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use event::{Device, Event, Served};
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use govee::Govee;
#[cfg(feature = "ble")]
pub use provision::WifiCredentials;
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use stream::{Rate, SegmentStream, StreamOptions, Zones};
#[cfg(any(feature = "lan", feature = "ble", feature = "cloud"))]
pub use transport::{DeviceId, DeviceStatus, Health, Reply, State, Transport};
