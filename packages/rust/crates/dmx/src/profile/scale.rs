//! A DMX slot, scaled into what a device parameter takes.
//!
//! A slot holds 0 to 255. A parameter holds what the device file declares for
//! it. The scale joins the two, so the operator gets a full travel on every
//! device — see `docs/dmx.md`.

/// The slot that carries no value. The dimmer powers the device off there,
/// and the white channel sends no command.
pub const OFF: u8 = 0;

/// How many slots carry a value above [`Zero::Off`], which is 1 to 255.
const ABOVE_OFF: i64 = 254;

/// How many slots carry a value above [`Zero::Min`], which is 0 to 255.
const ABOVE_MIN: i64 = 255;

/// What slot 0 carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zero {
    /// No value. The dimmer powers the device off, and the white channel
    /// sends no command.
    Off,
    /// The bottom of the pair. Every color component travels the whole pair,
    /// because a component at 0 is a color the device shows.
    Min,
}

/// The bounds one scaled channel writes into, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale {
    min: i64,
    max: i64,
    zero: Zero,
}

impl Scale {
    /// The scale over `[min, max]`, as the device file declares the pair.
    /// `None` where `max` is below `min`.
    #[must_use]
    pub fn new([min, max]: [i64; 2], zero: Zero) -> Option<Self> {
        (min <= max).then_some(Self { min, max, zero })
    }

    /// The scale a byte channel writes into, at [`Zero::Min`].
    ///
    /// `None` where `max` is below `min`, or where the pair leaves a byte. A
    /// color component carries a byte to the wire, so a pair it cannot hold
    /// is a pair this channel cannot drive.
    #[must_use]
    pub fn bytes(range: [i64; 2]) -> Option<Self> {
        let scale = Self::new(range, Zero::Min)?;
        (scale.min >= 0 && scale.max <= 255).then_some(scale)
    }

    /// The pair this channel writes into.
    #[must_use]
    pub fn range(self) -> [i64; 2] {
        [self.min, self.max]
    }

    /// The value slot `slot` writes, `min + round(carried × span / carrying)`.
    ///
    /// `None` at [`OFF`] on a [`Zero::Off`] scale, which carries no value.
    /// Every other slot lands inside the pair: this scales, and never clamps.
    #[must_use]
    pub fn value(self, slot: u8) -> Option<i64> {
        let carried = match self.zero {
            Zero::Off if slot == OFF => return None,
            Zero::Off => i64::from(slot) - 1,
            Zero::Min => i64::from(slot),
        };
        let carrying = self.carrying();
        let span = self.max - self.min;
        // Integer rounding to nearest. Both terms are positive, so the
        // division truncates toward zero and the half goes up.
        Some(self.min + (carried * span + carrying / 2) / carrying)
    }

    /// The byte slot `slot` writes.
    ///
    /// Every pair [`Self::bytes`] accepts fits in a byte, so the value does
    /// too. A wider pair from [`Self::new`] answers with the slot itself.
    #[must_use]
    pub fn byte(self, slot: u8) -> u8 {
        self.value(slot)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(slot)
    }

    /// How many distinct values the carrying slots reach.
    ///
    /// A pair narrower than the slots quantizes: `[1, 100]` puts about 2.5
    /// slots on one step, so a slow fade on the desk lands stepped on the
    /// device. That is the hardware's resolution, and the operator has to
    /// know it.
    #[must_use]
    pub fn steps(self) -> u32 {
        let values = self.max - self.min + 1;
        let slots = u32::try_from(self.carrying() + 1).unwrap_or(u32::MAX);
        u32::try_from(values).unwrap_or(u32::MAX).min(slots)
    }

    /// How far the carrying slots travel, which is one below their count.
    const fn carrying(self) -> i64 {
        match self.zero {
            Zero::Off => ABOVE_OFF,
            Zero::Min => ABOVE_MIN,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{OFF, Scale, Zero};

    fn scale(range: [i64; 2]) -> Scale {
        Scale::new(range, Zero::Off).expect("the pair is ordered")
    }

    fn bytes(range: [i64; 2]) -> Scale {
        Scale::bytes(range).expect("the pair is an ordered byte pair")
    }

    #[test]
    fn a_reversed_pair_is_no_scale() {
        assert_eq!(Scale::new([100, 1], Zero::Off), None);
        assert_eq!(Scale::bytes([100, 1]), None);
    }

    /// A color component carries a byte to the wire. A pair a byte cannot
    /// hold is refused here, so the channel reports it rather than sending a
    /// value the frame cannot carry.
    #[test]
    fn a_pair_outside_a_byte_is_no_byte_scale() {
        assert_eq!(Scale::bytes([0, 1000]), None);
        assert_eq!(Scale::bytes([-1, 255]), None);
        assert!(Scale::bytes([0, 255]).is_some());
    }

    #[test]
    fn the_carrying_slots_reach_both_ends() {
        let dimmer = scale([1, 100]);
        assert_eq!(dimmer.value(OFF), None);
        assert_eq!(dimmer.value(1), Some(1));
        assert_eq!(dimmer.value(255), Some(100));
    }

    /// Slot 0 is a color the device shows, so a component travels the whole
    /// pair and answers at every slot.
    #[test]
    fn a_byte_scale_carries_slot_zero() {
        let red = bytes([0, 255]);
        assert_eq!(red.value(OFF), Some(0));
        assert_eq!(red.byte(OFF), 0);
        assert_eq!(red.byte(128), 128);
        assert_eq!(red.byte(255), 255);
    }

    /// The whole point of the byte scale: a narrower pair still takes the
    /// full travel of the fader.
    #[test]
    fn a_narrow_byte_pair_still_takes_the_whole_fader() {
        let red = bytes([0, 100]);
        assert_eq!(red.byte(OFF), 0);
        assert_eq!(red.byte(255), 100);
        for slot in 0..=255u8 {
            assert!(red.byte(slot) <= 100, "slot {slot} left the pair");
        }
    }

    #[test]
    fn the_middle_slot_rounds_to_the_middle_value() {
        assert_eq!(scale([0, 254]).value(128), Some(127));
        assert_eq!(scale([2000, 9000]).value(128), Some(5500));
    }

    /// The count the operator patches against has to be the count the scale
    /// produces, on a pair narrower than the slots and on one wider.
    #[test]
    fn the_step_count_is_what_the_slots_reach() {
        for range in [[1, 100], [0, 255], [2000, 9000], [50, 50]] {
            let scale = scale(range);
            let mut values: Vec<i64> = (1..=255).filter_map(|slot| scale.value(slot)).collect();
            values.dedup();
            assert_eq!(
                u32::try_from(values.len()),
                Ok(scale.steps()),
                "{range:?} reaches a different count",
            );
        }
        for range in [[0, 255], [0, 100], [10, 20]] {
            let scale = bytes(range);
            let mut values: Vec<i64> = (0..=255).filter_map(|slot| scale.value(slot)).collect();
            values.dedup();
            assert_eq!(
                u32::try_from(values.len()),
                Ok(scale.steps()),
                "{range:?} reaches a different count",
            );
        }
    }

    #[test]
    fn every_slot_lands_inside_the_pair() {
        let scale = scale([1, 100]);
        for slot in 1..=255u8 {
            let value = scale.value(slot).expect("a carrying slot");
            assert!((1..=100).contains(&value), "slot {slot} left the pair");
        }
    }
}
