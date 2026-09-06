//! Numbers taken from one physical unit.
//!
//! Segment count, native resolution and sustainable frame rate all depend on
//! the LENGTH of the unit measured, not only on its SKU — two ropes sharing a
//! model in different lengths share none of them. So these are records of an
//! observation, never a property of the model, and nothing here is derived from
//! anything else: an absent number stays absent.
//!
//! See `docs/protocol/lan.md` 2.3 and 2.7 for how they are measured.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::codec::catalog::Mode;

/// One row of `measurements.frame_rate`: how fast one physical unit accepts
/// segment frames at a given zone count.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FrameRate {
    /// Zones the frames carried.
    pub zones: u32,
    /// Size of the raw frame at that zone count, in bytes.
    pub payload_bytes: Option<u32>,
    /// Highest rate the unit sustained without visible stutter, in hertz.
    pub clean_hz: f64,
    /// Rate at which it began to break up, in hertz.
    pub breaks_at_hz: Option<f64>,
}

/// Sustainable segment frame rates, as a device file records them.
///
/// A bare list is the `lan` table, which is where the measurement started. A
/// mapping records one table per mode: the channels differ in frame size and
/// in what the firmware does between writes, so a rate measured over one says
/// nothing about another.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FrameRates {
    /// Rows measured over `lan`.
    Lan(Vec<FrameRate>),
    /// Rows measured per mode.
    ByMode(BTreeMap<Mode, Vec<FrameRate>>),
}

impl Default for FrameRates {
    fn default() -> Self {
        Self::Lan(Vec::new())
    }
}

impl FrameRates {
    /// The rows measured over `mode`, empty when nobody measured it there.
    #[must_use]
    pub fn rows(&self, mode: Mode) -> &[FrameRate] {
        match self {
            Self::Lan(rows) if mode == Mode::Lan => rows,
            Self::Lan(_) => &[],
            Self::ByMode(by_mode) => by_mode.get(&mode).map_or(&[], Vec::as_slice),
        }
    }
}

/// Numbers taken from one physical unit.
///
/// Only [`Measurements::frame_rate`] is read by the SDK; everything else a
/// device file records lands in [`Measurements::extra`] and is carried through
/// untouched. These are properties of the unit measured, not of the SKU — the
/// same model in another length does not share them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Measurements {
    /// Length of the unit the numbers were taken on, in metres.
    pub unit_length_m: Option<f64>,
    /// Addressable LEDs counted on that unit.
    pub native_pixels: Option<u32>,
    /// Sustainable segment frame rates, by mode and zone count.
    pub frame_rate: FrameRates,
    /// Everything else the file records.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Measurements {
    /// The measured rate for `zones` over `mode`, in hertz, or `None` if
    /// nothing was measured on this unit for that mode.
    ///
    /// Answers with the smallest row that covers `zones`. Past the largest row
    /// it answers with that row, which is the slowest rate anybody measured:
    /// the ceiling only falls as frames grow, so that is a floor already
    /// observed rather than a value extrapolated from the trend. A rate is
    /// never carried from one mode to another.
    #[must_use]
    pub fn clean_hz(&self, mode: Mode, zones: u32) -> Option<f64> {
        let rows = self.frame_rate.rows(mode);
        rows.iter()
            .filter(|row| row.zones >= zones)
            .min_by_key(|row| row.zones)
            .or_else(|| rows.iter().max_by_key(|row| row.zones))
            .map(|row| row.clean_hz)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// The `measurements:` block of `devices/H61A0.yaml`, verbatim.
    const H61A0: &str = "
unit_length_m: 3
native_pixels: 42
segment_count_app: 10
resolution_changepoints: [1, 2, 3, 4, 5, 6, 7, 9, 11, 14, 21, 42]
latency_idle_ms:
  median: 16
  p95: 28
  max: 39
  loss: \"0/30 requests\"
ble:
  read_round_trip_ms: 63
  sustained_writes_hz: 130
  write_budget_hz: 100
  burst_frames_before_stall: 100
  burst_recovery_s: 20
  addressable_zones: 15
frame_rate:
  lan:
    - { zones: 20,  payload_bytes: 62,  clean_hz: 40, breaks_at_hz: 45 }
    - { zones: 60,  payload_bytes: 182, clean_hz: 25, breaks_at_hz: 30 }
    - { zones: 120, payload_bytes: 362, clean_hz: 20, breaks_at_hz: 25 }
";

    fn measured() -> Measurements {
        serde_norway::from_str(H61A0).expect("the H61A0 measurements parse")
    }

    #[test]
    fn what_the_sdk_does_not_read_is_carried_through_untouched() {
        let m = measured();
        assert_eq!(m.unit_length_m, Some(3.0));
        assert_eq!(m.native_pixels, Some(42));
        assert_eq!(m.frame_rate.rows(Mode::Lan).len(), 3);
        assert!(m.extra.contains_key("latency_idle_ms"));
        assert!(m.extra.contains_key("resolution_changepoints"));
    }

    #[test]
    fn a_mode_nobody_ran_the_stutter_test_on_answers_nothing() {
        // The unit's `measurements.ble` block records a write budget, which is
        // not a frame rate: no row is derived from it.
        let m = measured();
        assert!(m.frame_rate.rows(Mode::Ble).is_empty());
        assert_eq!(m.clean_hz(Mode::Ble, 15), None);
    }

    #[test]
    fn a_zone_count_is_answered_by_the_smallest_row_that_covers_it() {
        let m = measured();
        assert_eq!(m.clean_hz(Mode::Lan, 20), Some(40.0));
        assert_eq!(m.clean_hz(Mode::Lan, 1), Some(40.0));
        assert_eq!(m.clean_hz(Mode::Lan, 21), Some(25.0));
        assert_eq!(m.clean_hz(Mode::Lan, 60), Some(25.0));
        assert_eq!(m.clean_hz(Mode::Lan, 119), Some(20.0));
    }

    #[test]
    fn past_the_last_row_the_slowest_measured_rate_stands() {
        // Not extrapolated: 20 Hz is a rate this unit was seen to hold, and the
        // ceiling only falls as frames grow.
        assert_eq!(measured().clean_hz(Mode::Lan, 255), Some(20.0));
    }

    #[test]
    fn a_unit_nobody_measured_answers_nothing() {
        let m: Measurements = serde_norway::from_str("unit_length_m: 5").expect("parses");
        assert_eq!(m.clean_hz(Mode::Lan, 10), None);
    }

    #[test]
    fn a_bare_table_is_the_lan_one_and_answers_for_no_other_mode() {
        let m: Measurements =
            serde_norway::from_str("frame_rate:\n  - { zones: 20, clean_hz: 40 }\n")
                .expect("parses");
        assert_eq!(m.clean_hz(Mode::Lan, 10), Some(40.0));
        assert_eq!(m.clean_hz(Mode::Ble, 10), None);
        assert!(m.frame_rate.rows(Mode::Ble).is_empty());
    }

    #[test]
    fn a_table_keyed_by_mode_answers_per_mode() {
        let m: Measurements = serde_norway::from_str(
            "frame_rate:\n  lan:\n    - { zones: 20, clean_hz: 40 }\n  \
             ble:\n    - { zones: 15, clean_hz: 5 }\n",
        )
        .expect("parses");
        assert_eq!(m.clean_hz(Mode::Lan, 10), Some(40.0));
        assert_eq!(m.clean_hz(Mode::Ble, 10), Some(5.0));
        assert_eq!(m.clean_hz(Mode::Cloud, 10), None);
    }

    #[test]
    fn rows_out_of_order_answer_the_same() {
        let m: Measurements = serde_norway::from_str(
            "frame_rate:\n  - { zones: 120, clean_hz: 20 }\n  - { zones: 20, clean_hz: 40 }\n",
        )
        .expect("parses");
        assert_eq!(m.clean_hz(Mode::Lan, 20), Some(40.0));
    }
}
