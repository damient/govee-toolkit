//! How one coarse zone covers the addressable LEDs behind it.
//!
//! A device file that declares `capabilities.segments.groups` states a zone
//! count under its LED count. The bridge lays out that many DMX zones and
//! paints each one over its own run of LEDs, so the rendering does not depend
//! on how the firmware groups the LEDs for a smaller frame. See `docs/dmx.md`
//! 1.2.

use crate::codec::capabilities::groups_fit;

/// Where the LEDs of one group start and stop.
///
/// Groups are contiguous runs in chain order, and two runs differ by one LED
/// at most: LED `i` falls in group `i * groups / pixels`. A chain that folds
/// back on itself therefore carries a group across the fold — see the
/// `segment_chain` measurement of the device file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    groups: u32,
    pixels: u32,
}

impl Spread {
    /// The layout of `groups` over `pixels`, and `None` where either counts
    /// nothing or where the groups outnumber the LEDs.
    #[must_use]
    pub const fn new(groups: u32, pixels: u32) -> Option<Self> {
        if !groups_fit(groups, pixels) {
            return None;
        }
        Some(Self { groups, pixels })
    }

    /// How many LEDs the frame carries.
    #[must_use]
    pub const fn pixels(self) -> u32 {
        self.pixels
    }

    /// One colour per LED, from one colour per group.
    ///
    /// A group the caller states no colour for renders black, which is what a
    /// desk that sends a short packet asks for.
    #[must_use]
    pub fn apply(self, groups: &[[u8; 3]]) -> Vec<[u8; 3]> {
        (0..u64::from(self.pixels))
            .map(|pixel| {
                let group = pixel * u64::from(self.groups) / u64::from(self.pixels);
                usize::try_from(group)
                    .ok()
                    .and_then(|group| groups.get(group))
                    .copied()
                    .unwrap_or([0, 0, 0])
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]

    use super::Spread;

    fn spread(groups: u32, pixels: u32) -> Spread {
        Spread::new(groups, pixels).expect("the counts lay out")
    }

    #[test]
    fn a_group_covers_its_own_run_of_leds() {
        let painted = spread(3, 6).apply(&[[1, 0, 0], [2, 0, 0], [3, 0, 0]]);
        assert_eq!(
            painted,
            [
                [1, 0, 0],
                [1, 0, 0],
                [2, 0, 0],
                [2, 0, 0],
                [3, 0, 0],
                [3, 0, 0]
            ]
        );
    }

    /// 132 LEDs over 15 groups is 8.8 LEDs a group, so no run is the same
    /// length as every other one.
    #[test]
    fn a_count_that_does_not_divide_spreads_the_remainder() {
        let groups: Vec<[u8; 3]> = (0..15).map(|index| [index, 0, 0]).collect();
        let painted = spread(15, 132).apply(&groups);
        assert_eq!(painted.len(), 132);
        let mut runs = [0u32; 15];
        for pixel in &painted {
            runs[usize::from(pixel[0])] += 1;
        }
        assert_eq!(runs.iter().sum::<u32>(), 132);
        assert_eq!(runs.iter().copied().max(), Some(9));
        assert_eq!(runs.iter().copied().min(), Some(8));
    }

    #[test]
    fn one_group_per_led_repaints_every_led_on_its_own() {
        let groups = [[1, 0, 0], [2, 0, 0], [3, 0, 0]];
        assert_eq!(spread(3, 3).apply(&groups), groups);
    }

    #[test]
    fn a_group_the_caller_states_no_colour_for_renders_black() {
        assert_eq!(spread(2, 4).apply(&[[9, 9, 9]])[3], [0, 0, 0]);
    }

    #[test]
    fn counts_that_lay_out_no_group_answer_nothing() {
        assert_eq!(Spread::new(0, 10), None);
        assert_eq!(Spread::new(10, 0), None);
        assert_eq!(Spread::new(11, 10), None);
    }
}
