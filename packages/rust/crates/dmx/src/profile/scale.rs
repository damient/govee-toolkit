//! A DMX slot, scaled into what a device parameter takes.
//!
//! A slot holds 0 to 255. A parameter holds what the device file declares for
//! it. The scale joins the two, so the operator gets a full travel on every
//! device — see `docs/dmx.md`.

/// The slot that carries no value. The dimmer powers the device off there,
/// and the white channel sends no command.
pub const OFF: u8 = 0;

/// How many slots carry a value, which is 1 to 255.
const CARRYING: i64 = 254;

/// The bounds one scaled channel writes into, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale {
    min: i64,
    max: i64,
}

impl Scale {
    /// The scale over `[min, max]`, as the device file declares the pair.
    /// `None` where `max` is below `min`.
    #[must_use]
    pub fn new([min, max]: [i64; 2]) -> Option<Self> {
        (min <= max).then_some(Self { min, max })
    }

    /// The pair this channel writes into.
    #[must_use]
    pub fn range(self) -> [i64; 2] {
        [self.min, self.max]
    }

    /// The value slot `slot` writes, `min + round((slot - 1) × span / 254)`.
    ///
    /// `None` at [`OFF`], which carries no value. Every other slot lands
    /// inside the pair: this scales, and never clamps.
    #[must_use]
    pub fn value(self, slot: u8) -> Option<i64> {
        if slot == OFF {
            return None;
        }
        let span = self.max - self.min;
        let carried = i64::from(slot) - 1;
        // Integer rounding to nearest. Both terms are positive, so the
        // division truncates toward zero and the half goes up.
        Some(self.min + (carried * span + CARRYING / 2) / CARRYING)
    }

    /// How many distinct values the 255 carrying slots reach.
    ///
    /// A pair narrower than the slots quantizes: `[1, 100]` puts about 2.5
    /// slots on one step, so a slow fade on the desk lands stepped on the
    /// device. That is the hardware's resolution, and the operator has to
    /// know it.
    #[must_use]
    pub fn steps(self) -> u32 {
        let values = self.max - self.min + 1;
        u32::try_from(values).unwrap_or(u32::MAX).min(255)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{OFF, Scale};

    fn scale(range: [i64; 2]) -> Scale {
        Scale::new(range).expect("the pair is ordered")
    }

    #[test]
    fn a_reversed_pair_is_no_scale() {
        assert_eq!(Scale::new([100, 1]), None);
    }

    #[test]
    fn the_carrying_slots_reach_both_ends() {
        let dimmer = scale([1, 100]);
        assert_eq!(dimmer.value(OFF), None);
        assert_eq!(dimmer.value(1), Some(1));
        assert_eq!(dimmer.value(255), Some(100));
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
