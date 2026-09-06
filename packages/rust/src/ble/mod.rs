//! The `ble` transport: GATT over Bluetooth Low Energy.
//!
//! Carries the frames [`crate::codec`] produces. Everything it needs is on the
//! wire itself — one vendor service, one characteristic to write to, one to be
//! notified on — and the bytes come from `devices/*.yaml`, as everywhere else.
//!
//! **Nothing here is verified against a device.** `docs/protocol/ble.md`
//! records nothing probed and no device file declares a `ble` command, so the
//! UUIDs, the frame length and the advertised-name prefixes below are the shape
//! the transport is written to, not facts this repository has established. Each
//! carries the same note, and each is a TODO until a capture backs it.
//!
//! It is one implementation of [`crate::transport::Transport`], under the same
//! rules as every other mode (`docs/modes.md`): it never chooses a mode, its
//! health is state already recorded, and nothing is approximated.
//!
//! What is specific to this mode:
//!
//! - **One connection at a time.** A connected device stops advertising, so a
//!   scan run while something else holds the link returns nothing. The
//!   transport keeps one link per device and reuses it.
//! - **Writes are paced.** The transport spends a write budget rather than
//!   trusting a caller. See [`pace`].
//! - **Identity is not observable from an advertisement.** A device is
//!   identified by its Wi-Fi MAC everywhere in this crate, and an advertisement
//!   carries the Bluetooth address instead. Nothing here relates the two — see
//!   [`transport::Transport::bind`].
//!
//! No adapter is claimed until something needs one: starting the transport on
//! a machine with no radio succeeds, and the first command is what fails.

pub mod link;
pub mod pace;
pub mod scan;
pub mod transport;

pub use pace::{Budget, Pacer};
pub use scan::Advertised;
pub use transport::{Options, Transport};
use uuid::Uuid;

// Shared with every other mode; re-exported so that `ble` reads as one module.
pub use crate::transport::{
    Breaker, Change, DeviceId, DeviceStatus, Discovered, Error, Event, Health, KnownDevice, Policy,
    Result, Sent, State, Transition, Verify,
};

/// The vendor service commands travel on.
///
/// Unverified in this repository: `docs/protocol/ble.md` records nothing
/// probed, and there is no capture under `tests/fixtures/ble-captures/` behind
/// this value. TODO: confirm it against a capture, and write it down there with
/// its provenance.
pub const SERVICE: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_1910);

/// The characteristic frames are written to, without a response. Unverified:
/// see [`SERVICE`].
pub const WRITE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b11);

/// The characteristic replies are notified on. Unverified: see [`SERVICE`].
pub const NOTIFY_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b10);

/// The length of every frame on this wire, in bytes.
///
/// The codec builds a frame from the device file's `frame:` layout, and this is
/// the one length the transport writes. Unverified: see [`SERVICE`].
pub const FRAME_LEN: usize = 20;
