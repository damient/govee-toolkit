//! The govee-toolkit facade: configuration, mode selection, events.
//!
//! The layer an application talks to. It reads the user's configuration, holds
//! the catalog and the transports, and decides — for each command — which of
//! the modes the user enabled serves it.
//!
//! What "decides" means here is narrow, and deliberately so
//! (`docs/modes.md`):
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
//! use govee::{Config, Govee};
//! use govee_core::Args;
//!
//! # async fn example() -> Result<(), govee::Error> {
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

pub mod config;
pub mod error;
pub mod paths;

mod device;
mod event;
mod govee;

pub use config::{Config, DeviceConfig, LanConfig, Problem};
pub use device::DeviceHandle;
pub use error::{Error, Result};
pub use event::{Device, Event, Served};
pub use govee::Govee;
pub use govee_core::{Args, Catalog, Mode};
pub use govee_lan::{DeviceId, DeviceStatus, Health, State};
