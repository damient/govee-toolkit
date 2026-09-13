//! The `ble` transport: GATT over Bluetooth Low Energy.
//!
//! Carries the frames [`crate::codec`] produces over one service, and
//! implements [`crate::transport::Transport`] under `docs/modes.md`.
//!
//! The UUIDs and the frame length below were observed on one H61A0 and on no
//! other unit; `docs/protocol/ble.md` says what was exercised.
//!
//! Four things are specific to this mode. A device takes one connection at a
//! time and stops advertising while it is up, so the transport keeps one link
//! per device. Writes are paced — see [`pace`]. An advertisement carries
//! the Bluetooth address, not the Wi-Fi MAC this crate identifies a device by:
//! nothing relates the two, see [`transport::Transport::bind`]. And a device
//! whose advertisement carries the encoding flag takes encoded frames only,
//! so the link runs the handshake of [`session`] before the first command —
//! see [`encode`].
//!
//! No adapter is claimed until something needs one.

pub mod encode;
pub mod link;
pub mod pace;
pub mod radio;
pub mod scan;
pub mod session;
pub mod transport;
pub mod wire;

pub use pace::{Budget, Budgets, Pacer};
pub use radio::Radio;
pub use scan::{Advertised, Beacon};
pub use transport::{Options, Transport};
use uuid::Uuid;

// Re-exported so that `ble` reads as one module.
pub use crate::transport::{
    Breaker, Change, DeviceId, DeviceStatus, Discovered, Error, Event, Health, KnownDevice, Policy,
    Result, Sent, State, Transition, Verify,
};

/// The service commands travel on.
///
/// Observed on one unit, and on no other family.
pub const SERVICE: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_1910);

/// The characteristic frames are written to, without a response. Same
/// provenance as [`SERVICE`].
pub const WRITE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b11);

/// The characteristic replies are notified on. Same provenance as
/// [`SERVICE`].
pub const NOTIFY_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b10);

/// The length of a frame on this wire, in bytes, checksum included.
///
/// This wire has no MTU negotiation. The codec builds a frame from the device
/// file's `frame:` layout, which ends `<pad:20> <xor>` for that reason.
/// [`HOST_COLOR_PROTYPE`] is the one exception.
pub const FRAME_LEN: usize = 20;

/// The first byte of a host colour frame, which is shorter than
/// [`FRAME_LEN`] and carries a sum where the others carry an XOR.
///
/// See `docs/protocol/ble.md` 8.
pub const HOST_COLOR_PROTYPE: u8 = 0xA5;
