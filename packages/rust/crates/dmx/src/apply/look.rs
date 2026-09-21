//! What one frame asks of one fixture.
//!
//! The channel table says which address drives what, and the scales say what
//! each slot means. A look is the result: the values a device must show, in
//! the units the device file declares.

use crate::input::UniverseFrame;
use crate::patch::{Fixture, SignalLoss};
use crate::profile::{Channel, Component, MODE_RESEND, OFF, Slot};

/// What one fixture reads out of one frame.
///
/// A field is `None` where the personality carries no such channel, and where
/// the slot carries no value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Look {
    /// Whether the dimmer carries a value. `false` powers the device off.
    pub on: bool,
    /// The brightness, in the unit the device file declares. `None` where the
    /// dimmer is at 0.
    pub brightness: Option<i64>,
    /// One color over the whole device. `None` on a `segment` or a `pixel`
    /// personality.
    pub color: Option<[u8; 3]>,
    /// The white temperature, in kelvin. `None` at slot 0, which sends no
    /// white command. The send path paints the color again where the white
    /// channel comes back to 0, which is what takes the device out of white.
    pub white_temp: Option<i64>,
    /// One color per zone, in zone order. Empty on `basic` and on `full`.
    pub zones: Vec<[u8; 3]>,
    /// Whether the mode channel asks for a full resend.
    pub resend: bool,
}

impl Look {
    /// What `fixture` reads out of `frame`.
    ///
    /// A channel the frame stops short of reads 0, the way a desk that sends
    /// a short packet leaves the rest of the universe dark.
    #[must_use]
    pub fn read(fixture: &Fixture, frame: &UniverseFrame) -> Self {
        let mut look = Self {
            zones: vec![[0, 0, 0]; fixture.profile.zones()],
            ..Self::default()
        };
        let mut color = [0u8; 3];
        let mut painted = false;
        for channel in fixture.profile.channels() {
            let slot = slot_of(fixture, frame, channel);
            match channel.slot {
                Slot::Dimmer => {
                    look.on = slot != OFF;
                    look.brightness = channel.scale.and_then(|scale| scale.value(slot));
                }
                Slot::Color(component) => {
                    painted = true;
                    put(&mut color, component, byte(channel, slot));
                }
                Slot::WhiteTemp => look.white_temp = channel.scale.and_then(|s| s.value(slot)),
                Slot::Mode => look.resend = slot >= MODE_RESEND,
                Slot::Zone { index, component } => {
                    look.paint(index, component, byte(channel, slot));
                }
            }
        }
        if painted {
            look.color = Some(color);
        }
        look
    }

    /// What the fixture shows once the sender has gone quiet, and `None`
    /// where the patch holds the last look.
    ///
    /// `black` keeps the device on and takes every color to 0. It sends no
    /// white command: the color it sends is what the device shows, and a
    /// white command would light the rig at a blackout.
    #[must_use]
    pub fn quiet(&self, loss: SignalLoss) -> Option<Self> {
        match loss {
            SignalLoss::Hold => None,
            SignalLoss::Off => Some(Self::default()),
            SignalLoss::Black => Some(self.black()),
        }
    }

    /// The same look, dark: every color at 0, the device on, and the
    /// brightness untouched.
    ///
    /// It sends no white command, because a white command would light the
    /// fixture. The send path writes this where a dimmer reaches 0, so the
    /// device stays on and a dimmer that comes back up costs one repaint
    /// instead of a power cycle.
    #[must_use]
    pub fn black(&self) -> Self {
        Self {
            on: true,
            color: self.color.map(|_| [0, 0, 0]),
            white_temp: None,
            zones: vec![[0, 0, 0]; self.zones.len()],
            resend: false,
            brightness: self.brightness,
        }
    }

    fn paint(&mut self, index: u32, component: Component, value: u8) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        if index >= self.zones.len() {
            self.zones.resize(index + 1, [0, 0, 0]);
        }
        if let Some(zone) = self.zones.get_mut(index) {
            put(zone, component, value);
        }
    }
}

/// The value the frame carries for one channel of one fixture. The fixture's
/// start address is where its offsets count from.
fn slot_of(fixture: &Fixture, frame: &UniverseFrame, channel: &Channel) -> u8 {
    let address = fixture
        .entry
        .address
        .saturating_add(channel.offset.saturating_sub(1));
    frame.slot(address).unwrap_or(OFF)
}

/// The value one color channel writes. The scale is what carries a device
/// whose component takes less than a whole byte.
fn byte(channel: &Channel, slot: u8) -> u8 {
    channel.scale.map_or(slot, |scale| scale.byte(slot))
}

fn put(color: &mut [u8; 3], component: Component, value: u8) {
    let index = match component {
        Component::Red => 0,
        Component::Green => 1,
        Component::Blue => 2,
    };
    if let Some(slot) = color.get_mut(index) {
        *slot = value;
    }
}
