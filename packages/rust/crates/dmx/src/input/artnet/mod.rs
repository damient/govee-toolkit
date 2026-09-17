//! Art-Net: the bytes a desk sends, and the frame they carry.
//!
//! This module holds no socket. It takes the datagram and the address it came
//! from, and answers what the packet says. The socket is `src/input/` at the
//! step that binds UDP 6454.
//!
//! The layout is in `docs/dmx.md`. A refusal drops the packet: the bridge
//! never truncates a packet to make it fit.

mod sequence;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;

use thiserror::Error;

pub use self::sequence::Sequence;
use super::UniverseFrame;
use crate::profile::UNIVERSE;

/// The UDP port every Art-Net node listens on.
pub const PORT: u16 = 6454;

/// The first 8 bytes of every packet.
const ID: &[u8; 8] = b"Art-Net\0";
/// The opcode of an `ArtDmx` packet, which carries the channel values.
const OP_DMX: u16 = 0x5000;
/// The oldest protocol version the bridge accepts.
const MIN_VERSION: u16 = 14;
/// How many bytes an `ArtDmx` packet carries before its data.
const HEADER: usize = 18;
/// The smallest data length an `ArtDmx` packet declares.
const MIN_LENGTH: u16 = 2;
/// The bits of the port-address. The top bit of the Net byte is reserved and
/// a sender writes 0 there.
const PORT_ADDRESS: u16 = 0x7FFF;

/// What a datagram on port 6454 turns out to be.
///
/// One variant carries a universe and the other carries an opcode. A box
/// around the wide one would allocate once per received packet, which the
/// receive path does not pay.
#[expect(
    clippy::large_enum_variant,
    reason = "the receive path allocates nothing"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// One universe of channel values.
    Dmx(Dmx),
    /// A packet the bridge reads and does nothing with. `ArtPoll` lands here
    /// until the node answers polls.
    Other {
        /// The opcode the packet carries.
        opcode: u16,
    },
}

/// One `ArtDmx` packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dmx {
    /// The channel values, and the port-address they are for.
    pub frame: UniverseFrame,
    /// What orders two packets of one sender. 0 disables the check.
    pub sequence: u8,
    /// The physical input the sender read the data on. The bridge reports it
    /// and drives nothing with it.
    pub physical: u8,
}

/// Why a packet is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Error {
    /// The datagram is shorter than an `ArtDmx` header.
    #[error("the packet carries {carried} bytes, under the {HEADER} a header takes")]
    TooShort {
        /// How many bytes the datagram carries.
        carried: usize,
    },
    /// The first 8 bytes are not `Art-Net\0`.
    #[error("the packet does not open with `Art-Net`")]
    NotArtNet,
    /// The sender speaks a protocol version the bridge does not.
    #[error("the packet declares protocol version {version}, under {MIN_VERSION}")]
    Version {
        /// What the packet declares.
        version: u16,
    },
    /// The data length is odd. Art-Net counts channel values in pairs.
    #[error("the packet declares an odd length of {length}")]
    OddLength {
        /// What the packet declares.
        length: u16,
    },
    /// The data length is outside what one universe holds.
    #[error("the packet declares a length of {length}, outside {MIN_LENGTH} to {UNIVERSE}")]
    Length {
        /// What the packet declares.
        length: u16,
    },
    /// The datagram stops before the data the header declares.
    #[error("the packet declares {length} bytes of data and carries {carried}")]
    Truncated {
        /// What the packet declares.
        length: u16,
        /// How many bytes of data it carries.
        carried: usize,
    },
}

/// What the datagram `bytes` from `source` says.
///
/// # Errors
///
/// [`enum@Error`], one per refusal. The caller drops the packet and keeps
/// listening: one bad datagram never stops a show.
pub fn parse(bytes: &[u8], source: SocketAddr) -> Result<Packet, Error> {
    let header = bytes.get(..HEADER).ok_or(Error::TooShort {
        carried: bytes.len(),
    })?;
    if header.get(..8) != Some(ID.as_slice()) {
        return Err(Error::NotArtNet);
    }
    let opcode = u16::from_le_bytes([at(header, 8), at(header, 9)]);
    if opcode != OP_DMX {
        return Ok(Packet::Other { opcode });
    }
    let version = u16::from_be_bytes([at(header, 10), at(header, 11)]);
    if version < MIN_VERSION {
        return Err(Error::Version { version });
    }
    let length = u16::from_be_bytes([at(header, 16), at(header, 17)]);
    if !length.is_multiple_of(2) {
        return Err(Error::OddLength { length });
    }
    if !(MIN_LENGTH..=UNIVERSE).contains(&length) {
        return Err(Error::Length { length });
    }
    let data = bytes
        .get(HEADER..HEADER + usize::from(length))
        .ok_or(Error::Truncated {
            length,
            carried: bytes.len().saturating_sub(HEADER),
        })?;
    let universe = u16::from_be_bytes([at(header, 15), at(header, 14)]) & PORT_ADDRESS;
    Ok(Packet::Dmx(Dmx {
        frame: UniverseFrame::new(universe, source, data),
        sequence: at(header, 12),
        physical: at(header, 13),
    }))
}

/// One byte of the header. Every caller has already checked the length, so a
/// missing byte cannot happen and reads 0.
fn at(header: &[u8], index: usize) -> u8 {
    header.get(index).copied().unwrap_or(0)
}
