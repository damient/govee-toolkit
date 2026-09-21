//! The Art-Net port-address, in both spellings a desk shows.
//!
//! The bridge holds one `u16` either way — see `docs/dmx.md` 5.1.

use std::fmt;

use govee_toolkit::DeviceId;

use super::error::Error;

/// The largest port-address, which is 15 bits.
pub const MAX: u16 = 0x7FFF;
/// The largest Net.
const MAX_NET: u16 = 0x7F;
/// The largest Sub-Net, and the largest Universe inside one Sub-Net.
const MAX_NIBBLE: u16 = 0x0F;

/// One Art-Net port-address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortAddress(u16);

impl PortAddress {
    /// The address `value` numbers. `None` above [`MAX`].
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        if value > MAX { None } else { Some(Self(value)) }
    }

    /// The address the three parts number. `None` where a part is over the
    /// bits that hold it.
    #[must_use]
    pub const fn from_parts(net: u16, subnet: u16, universe: u16) -> Option<Self> {
        if net > MAX_NET || subnet > MAX_NIBBLE || universe > MAX_NIBBLE {
            return None;
        }
        Some(Self((net << 8) | (subnet << 4) | universe))
    }

    /// The whole 15-bit number, which is what an `ArtDmx` packet carries.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// The 7-bit Net, which `ArtDmx` carries in its own byte.
    #[must_use]
    pub const fn net(self) -> u8 {
        let [net, _] = self.0.to_be_bytes();
        net
    }

    /// The low byte: the Sub-Net in the high nibble, the Universe in the low
    /// one. `ArtDmx` calls it `SubUni`.
    #[must_use]
    pub const fn sub_uni(self) -> u8 {
        let [_, sub_uni] = self.0.to_be_bytes();
        sub_uni
    }
}

impl fmt::Display for PortAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The address as a patch entry spells it.
///
/// `universe` alone is the whole port-address. With `net` or `subnet` beside
/// it, the three parts number the address, and `universe` is then the 4-bit
/// part.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Spelling {
    pub(super) net: Option<u16>,
    pub(super) subnet: Option<u16>,
    pub(super) universe: u16,
}

impl Spelling {
    /// The address the three fields number.
    ///
    /// # Errors
    ///
    /// [`Error::Address`] where a field is over the bits that hold it. The
    /// message names `device`, because the operator reads it next to a desk.
    pub(super) fn resolve(self, device: &DeviceId) -> Result<PortAddress, Error> {
        let too_large = |field: &'static str, value: u16, max: u16| Error::Address {
            device: device.clone(),
            field,
            value,
            max,
        };
        match (self.net, self.subnet) {
            (None, None) => PortAddress::new(self.universe)
                .ok_or_else(|| too_large("universe", self.universe, MAX)),
            (net, subnet) => self.parts(net.unwrap_or(0), subnet.unwrap_or(0), &too_large),
        }
    }

    /// The three parts, each checked on its own so that the message names the
    /// field the operator has to correct.
    fn parts(
        self,
        net: u16,
        subnet: u16,
        too_large: &impl Fn(&'static str, u16, u16) -> Error,
    ) -> Result<PortAddress, Error> {
        if net > MAX_NET {
            return Err(too_large("net", net, MAX_NET));
        }
        if subnet > MAX_NIBBLE {
            return Err(too_large("subnet", subnet, MAX_NIBBLE));
        }
        PortAddress::from_parts(net, subnet, self.universe)
            .ok_or_else(|| too_large("universe", self.universe, MAX_NIBBLE))
    }
}
