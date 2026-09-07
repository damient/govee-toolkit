//! The `lan` transport: UDP on the local network.
//!
//! Carries the bytes [`crate::codec`] produces. It discovers devices,
//! remembers where they are, keeps one socket for everything and tracks each
//! device's health.
//!
//! It implements [`crate::transport::Transport`] under the rules in
//! `docs/modes.md`.
//!
//! What is specific to this mode:
//!
//! - **Devices are cached on disk.** A command reaches a device the last scan
//!   found, so no send waits for a scan.
//! - **One socket carries everything.** Discovery and every command share it.
//!
//! The identity, the breaker, the error and the reported status live in
//! [`crate::transport`], shared with every other mode. They are re-exported
//! here so that `lan` reads as one module.
//!
//! ```no_run
//! use govee_toolkit::codec::{self, Args, Catalog, Mode};
//! use govee_toolkit::lan::{Options, Transport};
//! use govee_toolkit::transport::{DeviceId, Transport as _, Verify};
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

pub mod cache;
pub mod discovery;
pub mod transport;

mod socket;

pub use cache::{Cache, CachedDevice};
pub use discovery::{DiscoveredDevice, Endpoints};
pub use transport::{Options, Transport};

pub use crate::transport::{
    Breaker, Change, DeviceId, DeviceStatus, Discovered, Error, Event, Health, KnownDevice, Policy,
    Result, Sent, State, Transition, Verify,
};
