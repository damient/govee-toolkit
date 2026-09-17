//! What one frame asks of one fixture.
//!
//! The channel table says which address drives what, and the scales say what
//! each slot means. A look is the result: the values a device must show, in
//! the units the device file declares.

use crate::input::UniverseFrame;
use crate::patch::Fixture;
use crate::profile::{Channel, Component, OFF, Slot};

/// The lowest control slot that forces a full resend. The channel holds 0 to 9
/// for no action, and every value between is reserved — see `docs/dmx.md`.
const RESEND: u8 = 250;

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
    /// One color over the whole device. `None` on a pixel personality.
    pub color: Option<[u8; 3]>,
    /// The white temperature, in kelvin. `None` at slot 0, which sends no
    /// white command, so the color stays.
    pub white_temp: Option<i64>,
    /// One color per zone, in zone order. Empty on `basic` and on `full`.
    pub zones: Vec<[u8; 3]>,
    /// Whether the control channel asks for a full resend.
    pub resend: bool,
}

impl Look {
    /// What `fixture` reads out of `frame`.
    ///
    /// A channel the frame stops short of reads 0, the way a desk that sends
    /// a short packet leaves the rest of the universe dark.
    #[must_use]
    pub fn read(fixture: &Fixture, frame: &UniverseFrame) -> Self {
        let mut look = Self::default();
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
                    put(&mut color, component, slot);
                }
                Slot::WhiteTemp => look.white_temp = channel.scale.and_then(|s| s.value(slot)),
                Slot::Control => look.resend = slot >= RESEND,
                Slot::Zone { index, component } => look.paint(index, component, slot),
            }
        }
        if painted {
            look.color = Some(color);
        }
        look
    }

    fn paint(&mut self, index: u32, component: Component, slot: u8) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        if index >= self.zones.len() {
            self.zones.resize(index + 1, [0, 0, 0]);
        }
        if let Some(zone) = self.zones.get_mut(index) {
            put(zone, component, slot);
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

fn put(color: &mut [u8; 3], component: Component, slot: u8) {
    let index = match component {
        Component::Red => 0,
        Component::Green => 1,
        Component::Blue => 2,
    };
    if let Some(value) = color.get_mut(index) {
        *value = slot;
    }
}
