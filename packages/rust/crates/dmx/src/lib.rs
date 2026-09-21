//! A DMX bridge that drives Govee devices over `lan`. Art-Net carries the
//! DMX in.
//!
//! Unofficial, and not affiliated with Govee. The design reference is
//! `docs/dmx.md`.
//!
//! The node is an **input**, not a mode. It maps each DMX slot to a device
//! parameter and writes the result through [`govee_toolkit`], the way any
//! other caller does. A device it drives must have `lan` enabled.
//!
//! # Layers
//!
//! - [`input`] — the protocols: the bytes a sender puts on the wire in, one
//!   universe of channel values out, over one socket.
//! - [`profile`] — the device file in, the channel table out. It lives in
//!   [`govee_toolkit`], so the catalog task and the site read the table this
//!   node drives.
//! - [`patch`] — which device answers which channels, on which universe.
//! - [`apply`] — the channel values in, the device commands out.
//! - [`node`] — the run loop that joins the four.
//! - [`report`] — a channel table, as text and as JSON.

pub mod apply;
pub mod input;
#[cfg(feature = "artnet")]
pub mod node;
pub mod patch;

pub use govee_toolkit::profile;
pub use govee_toolkit::profile::report;
