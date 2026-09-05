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
    /// Sustainable segment frame rates, by zone count.
    pub frame_rate: Vec<FrameRate>,
    /// Everything else the file records.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Measurements {
    /// The measured rate for `zones`, in hertz, or `None` if nothing was
    /// measured on this unit.
    ///
    /// Answers with the smallest row that covers `zones`. Past the largest row
    /// it answers with that row, which is the slowest rate anybody measured:
    /// the ceiling only falls as frames grow, so that is a floor already
    /// observed rather than a value extrapolated from the trend.
    #[must_use]
    pub fn clean_hz(&self, zones: u32) -> Option<f64> {
        let mut rows: Vec<&FrameRate> = self.frame_rate.iter().collect();
        rows.sort_by_key(|row| row.zones);
        rows.iter()
            .find(|row| row.zones >= zones)
            .or_else(|| rows.last())
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
frame_rate:
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
        assert_eq!(m.frame_rate.len(), 3);
        assert!(m.extra.contains_key("latency_idle_ms"));
        assert!(m.extra.contains_key("resolution_changepoints"));
    }

    #[test]
    fn a_zone_count_is_answered_by_the_smallest_row_that_covers_it() {
        let m = measured();
        assert_eq!(m.clean_hz(20), Some(40.0));
        assert_eq!(m.clean_hz(1), Some(40.0));
        assert_eq!(m.clean_hz(21), Some(25.0));
        assert_eq!(m.clean_hz(60), Some(25.0));
        assert_eq!(m.clean_hz(119), Some(20.0));
    }

    #[test]
    fn past_the_last_row_the_slowest_measured_rate_stands() {
        // Not extrapolated: 20 Hz is a rate this unit was seen to hold, and the
        // ceiling only falls as frames grow.
        assert_eq!(measured().clean_hz(255), Some(20.0));
    }

    #[test]
    fn a_unit_nobody_measured_answers_nothing() {
        let m: Measurements = serde_norway::from_str("unit_length_m: 5").expect("parses");
        assert_eq!(m.clean_hz(10), None);
    }

    #[test]
    fn rows_out_of_order_answer_the_same() {
        let m: Measurements = serde_norway::from_str(
            "frame_rate:\n  - { zones: 120, clean_hz: 20 }\n  - { zones: 20, clean_hz: 40 }\n",
        )
        .expect("parses");
        assert_eq!(m.clean_hz(20), Some(40.0));
    }
}
