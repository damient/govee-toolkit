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
//! - [`profile`] — the device file in, the channel table out. No I/O.

pub mod profile;
