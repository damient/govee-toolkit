//! One channel of a personality: where it sits, and what it drives.

use std::fmt;

use super::scale::Scale;

/// One component of an RGB triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    /// Red.
    Red,
    /// Green.
    Green,
    /// Blue.
    Blue,
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
        })
    }
}

/// What one channel drives on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Power and brightness. Slot 0 powers the device off, and every other
    /// slot powers it on and sets the brightness.
    Dimmer,
    /// One component of the color over the whole device.
    Color(Component),
    /// The white temperature, in kelvin. Slot 0 sends no white command, so
    /// the color stays.
    WhiteTemp,
    /// 250 to 255 forces a full resend, and 0 to 9 asks for no action. Every
    /// other value is reserved.
    Control,
    /// One component of one zone's color, in zone order.
    Zone {
        /// The zone, counted from 0.
        index: u32,
        /// Which component of its triple.
        component: Component,
    },
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimmer => f.write_str("dimmer"),
            Self::Color(component) => write!(f, "{component}"),
            Self::WhiteTemp => f.write_str("white temperature"),
            Self::Control => f.write_str("control"),
            Self::Zone { index, component } => write!(f, "zone {index} {component}"),
        }
    }
}

/// One channel of a channel table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// Where it sits inside the personality, counted from 1, the way a desk
    /// counts. The operator adds the patch entry's start address to it.
    pub offset: u16,
    /// What it drives.
    pub slot: Slot,
    /// The pair it scales into. `None` where the slot goes out as it is,
    /// which is every color component and the control channel.
    pub scale: Option<Scale>,
}

impl Channel {
    /// A channel that sends the slot as it is.
    pub(super) fn plain(offset: u16, slot: Slot) -> Self {
        Self {
            offset,
            slot,
            scale: None,
        }
    }

    /// A channel that scales the slot into `scale`.
    pub(super) fn scaled(offset: u16, slot: Slot, scale: Scale) -> Self {
        Self {
            offset,
            slot,
            scale: Some(scale),
        }
    }
}
