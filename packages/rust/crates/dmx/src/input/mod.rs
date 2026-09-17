//! The input layer: one socket, one parser, one frame.
//!
//! Every protocol produces one [`UniverseFrame`]. A second protocol therefore
//! adds a module and no new type — see `docs/dmx.md`.

#[cfg(feature = "artnet")]
pub mod artnet;

use std::net::SocketAddr;

use crate::profile::UNIVERSE;

/// One universe of channel values, from whichever protocol carried it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniverseFrame {
    /// The Art-Net port-address, or the sACN universe number.
    pub universe: u16,
    /// Where the frame came from. Two senders on one universe are two
    /// addresses, and the bridge names both.
    pub source: SocketAddr,
    /// What sACN carries and Art-Net does not.
    pub priority: Option<u8>,
    slots: [u8; UNIVERSE as usize],
    len: u16,
}

impl UniverseFrame {
    /// The frame `values` carries on `universe`, from `source`.
    ///
    /// A value past the 512 a universe holds is dropped: a caller that holds
    /// more has a packet the parser should have refused.
    #[must_use]
    pub fn new(universe: u16, source: SocketAddr, values: &[u8]) -> Self {
        let mut slots = [0u8; UNIVERSE as usize];
        let len = values.len().min(UNIVERSE as usize);
        if let (Some(target), Some(source)) = (slots.get_mut(..len), values.get(..len)) {
            target.copy_from_slice(source);
        }
        Self {
            universe,
            source,
            priority: None,
            slots,
            len: u16::try_from(len).unwrap_or(UNIVERSE),
        }
    }

    /// How many slots the packet carried. Every slot above it reads 0.
    #[must_use]
    pub fn len(&self) -> u16 {
        self.len
    }

    /// Whether the packet carried no slot.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The slots the packet carried.
    #[must_use]
    pub fn values(&self) -> &[u8] {
        self.slots.get(..usize::from(self.len)).unwrap_or(&[])
    }

    /// The value at DMX address `address`, which counts from 1 the way a desk
    /// counts. `None` outside a universe.
    #[must_use]
    pub fn slot(&self, address: u16) -> Option<u8> {
        if address == 0 {
            return None;
        }
        self.slots.get(usize::from(address - 1)).copied()
    }
}
