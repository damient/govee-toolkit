//! One line of the patch: which device answers which channels.

use std::fmt;

use govee_toolkit::DeviceId;
use serde::{Deserialize, Deserializer, de};

use super::address::Spelling;
use crate::profile::Personality;

/// What the bridge applies after the sender goes quiet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalLoss {
    /// Keep the last values. The default.
    #[default]
    Hold,
    /// Take every color to 0, and leave the device on.
    Black,
    /// Power the device off.
    Off,
}

impl fmt::Display for SignalLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hold => "hold",
            Self::Black => "black",
            Self::Off => "off",
        })
    }
}

/// One patch entry, as the file spells it.
///
/// The port-address it resolves to and the channels it takes are
/// [`super::Fixture`]: both need the device, and the file names it by
/// identity alone.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The identity `govee scan` reports.
    pub device: DeviceId,
    /// The 7-bit Net. Unset with `subnet` unset makes `universe` the whole
    /// port-address.
    pub net: Option<u16>,
    /// The 4-bit Sub-Net. See [`Entry::net`].
    pub subnet: Option<u16>,
    /// The port-address, or the 4-bit Universe where `net` or `subnet` is
    /// set.
    #[serde(default)]
    pub universe: u16,
    /// The DMX start address, which is the first channel the fixture answers
    /// to. 1 to 512, the way a desk counts.
    pub address: u16,
    /// The channel layout, which decides how many channels follow the start
    /// address.
    #[serde(deserialize_with = "personality")]
    pub personality: Personality,
    /// The rate to send at. Unset takes the measurement in the device file.
    pub max_hz: Option<f64>,
    /// What to apply after the sender goes quiet.
    #[serde(default)]
    pub on_signal_loss: SignalLoss,
}

impl Entry {
    /// The port-address this entry spells.
    ///
    /// # Errors
    ///
    /// [`super::Error::Address`] where a part is over the bits that hold it.
    pub fn port_address(&self) -> Result<super::PortAddress, super::Error> {
        Spelling {
            net: self.net,
            subnet: self.subnet,
            universe: self.universe,
        }
        .resolve(&self.device)
    }
}

/// The personality a name spells, refused at the line that carries it rather
/// than at the first frame.
fn personality<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Personality, D::Error> {
    let name = String::deserialize(deserializer)?;
    Personality::parse(&name).ok_or_else(|| {
        let spellings: Vec<&str> = Personality::ALL.iter().map(|p| p.as_str()).collect();
        de::Error::custom(format!(
            "unknown personality `{name}`; expected one of {}",
            spellings.join(", ")
        ))
    })
}
