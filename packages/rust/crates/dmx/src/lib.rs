//! An Art-Net node that drives Govee devices over `lan`.
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
//!   universe of channel values out.
//! - [`profile`] — the device file in, the channel table out. No I/O.
//! - [`patch`] — which device answers which channels, on which universe.
//! - [`report`] — a channel table, as text and as JSON.

pub mod input;
pub mod patch;
pub mod profile;
pub mod report;
