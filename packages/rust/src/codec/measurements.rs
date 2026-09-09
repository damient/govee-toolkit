//! Numbers taken from one physical unit.
//!
//! Segment count, native resolution and sustainable frame rate depend on the
//! LENGTH of the unit, not only on its SKU, so each is an observation and not
//! a property of the model. Nothing here is derived: an absent number stays
//! absent. See `docs/protocol/lan.md` 2.3 and 2.7.

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
/// A bare list is the `lan` table; a mapping records one table per mode. A
/// rate measured over one mode says nothing about another.
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

/// The `measurements.ble` block: what one unit did over Bluetooth.
///
/// [`Ble::write_budget_hz`] and [`Ble::write_drain_ms`] are the fields the SDK
/// reads. The `ble` transport paces its writes to the first, and holds a link
/// open for the second before it drops it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Ble {
    /// Round trip of one read, in milliseconds.
    pub read_round_trip_ms: Option<f64>,
    /// Writes per second the unit held over seconds.
    pub sustained_writes_hz: Option<f64>,
    /// Writes per second the transport paces itself to, at or under
    /// `sustained_writes_hz`. [`crate::codec::validate`] checks that.
    pub write_budget_hz: Option<f64>,
    /// How long a link must stay open after a write, in milliseconds, for the
    /// frame to leave. The wire takes no acknowledgement, so a link dropped
    /// sooner loses the frame and reports nothing.
    pub write_drain_ms: Option<u64>,
    /// Frames in one burst that left the firmware unresponsive. The count
    /// that broke the unit, never a burst allowance.
    pub burst_frames_before_stall: Option<u32>,
    /// How long the firmware stayed unresponsive after such a burst, in
    /// seconds.
    pub burst_recovery_s: Option<f64>,
    /// Zones the unit addressed by mask over this mode.
    pub addressable_zones: Option<u32>,
    /// Everything else the block records.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Numbers taken from one physical unit.
///
/// The SDK reads [`Measurements::frame_rate`], [`Ble::write_budget_hz`] and
/// [`Ble::write_drain_ms`].
/// Everything else a device file records lands in [`Measurements::extra`],
/// untouched.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Measurements {
    /// Length of the unit the numbers were taken on, in metres.
    pub unit_length_m: Option<f64>,
    /// Addressable LEDs counted on that unit.
    pub native_pixels: Option<u32>,
    /// Sustainable segment frame rates, by mode and zone count.
    pub frame_rate: FrameRates,
    /// What one unit did over `ble`.
    pub ble: Ble,
    /// Everything else the file records.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Measurements {
    /// The measured rate for `zones` over `mode`, in hertz, or `None` if
    /// nothing was measured on this unit for that mode.
    ///
    /// The smallest row that covers `zones`, or the largest row past that.
    /// The ceiling only falls as frames grow, so the largest row is an
    /// observed floor and not an extrapolation.
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
  write_drain_ms: 50
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
    fn the_write_budget_is_read_off_the_unit_that_was_measured() {
        let m = measured();
        assert_eq!(m.ble.write_budget_hz, Some(100.0));
        assert_eq!(m.ble.write_drain_ms, Some(50));
        assert_eq!(m.ble.sustained_writes_hz, Some(130.0));
        assert_eq!(m.ble.burst_frames_before_stall, Some(100));
        assert!(m.ble.extra.is_empty());
    }

    #[test]
    fn a_unit_nobody_measured_over_ble_records_no_budget() {
        let m: Measurements = serde_norway::from_str("unit_length_m: 5").expect("parses");
        assert_eq!(m.ble.write_budget_hz, None);
        assert_eq!(m.ble.write_drain_ms, None);
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
