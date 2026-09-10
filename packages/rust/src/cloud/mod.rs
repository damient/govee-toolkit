//! The `cloud` transport: Govee's documented HTTPS API.
//!
//! Carries what [`crate::codec`] builds for [`Mode::Cloud`](crate::codec::Mode)
//! — one capability and its value — to a device the account owns, wherever
//! that device is. Every call is an internet round-trip, and requests are
//! throttled per device.
//!
//! The API key is never read from `config.yaml` and never logged.
//!
//! ```no_run
//! use govee_toolkit::cloud::{Options, Transport};
//! use govee_toolkit::codec::{self, Args, Catalog, Mode};
//! use govee_toolkit::transport::{DeviceId, Transport as _, Verify};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let catalog = Catalog::embedded()?;
//! let transport = Transport::start(Options {
//!     key: std::env::var("GOVEE_API_KEY")?,
//!     ..Options::default()
//! })?;
//! transport.scan().await?;
//!
//! let id = DeviceId::new("aa:bb:cc:dd:ee:ff");
//! let device = catalog.device(&transport.sku(&id).unwrap_or_default())?;
//! let on = codec::encode(device, Mode::Cloud, "power", &Args::new().int("on", 1))?;
//! transport.send(&id, &on, Verify::None).await?;
//! # Ok(())
//! # }
//! ```

pub mod transport;

mod api;
mod status;

pub use api::{ApiDevice, BASE_URL, CapabilityState};
pub use transport::{Options, Transport};

pub use crate::transport::{
    Breaker, Change, DeviceId, DeviceStatus, Discovered, Error, Event, Health, KnownDevice, Policy,
    Result, Sent, State, Transition, Verify,
};
