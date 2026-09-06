//! The `lan` transport: UDP on the local network.
//!
//! Carries the bytes [`crate::codec`] produces. It discovers devices,
//! remembers where they are, keeps one socket for everything and tracks each
//! device's health.
//!
//! It is one implementation of [`crate::transport::Transport`]. The rules below
//! are that trait's, and come from `docs/modes.md`:
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
