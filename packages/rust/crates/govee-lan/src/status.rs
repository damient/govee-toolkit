//! What a device reports about itself.
//!
//! `devStatus` and the undocumented `status` of `docs/protocol/lan.md` §2.2
//! both land here. Parsing is generic: every field is optional, because which
//! ones a firmware fills in is not something this crate can know, and `raw`
//! keeps whatever was not recognized.

use crate::DeviceId;

/// A device's reported state.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceStatus {
    /// Which device answered.
    pub id: DeviceId,
    /// `onOff`.
    pub on: Option<bool>,
    /// `brightness`, as the device reports it — a percentage on every unit seen
    /// so far, but not normalized here.
    pub brightness: Option<i64>,
    /// `color`. Reset to `{0,0,0}` while the device is in white mode
    /// (`docs/protocol/lan.md` §2.1).
    pub color: Option<[u8; 3]>,
    /// `colorTemInKelvin`. `0` means the device is in color mode.
    pub color_temp_kelvin: Option<i64>,
    /// The whole `msg.data`, untouched. Undocumented fields — the frozen `pt`
    /// descriptor of §2.2 among them — stay reachable without this crate having
    /// to model them.
    pub raw: serde_json::Value,
}

impl DeviceStatus {
    /// Read one out of a reply's `msg.data`.
    #[must_use]
    pub fn from_data(id: DeviceId, data: &serde_json::Value) -> Self {
        let int = |key: &str| data.get(key).and_then(serde_json::Value::as_i64);
        let channel = |key: &str| {
            data.get("color")
                .and_then(|c| c.get(key))
                .and_then(serde_json::Value::as_i64)
                .and_then(|v| u8::try_from(v).ok())
        };

        Self {
            id,
            on: int("onOff").map(|v| v != 0),
            brightness: int("brightness"),
            color: match (channel("r"), channel("g"), channel("b")) {
                (Some(r), Some(g), Some(b)) => Some([r, g, b]),
                _ => None,
            },
            color_temp_kelvin: int("colorTemInKelvin"),
            raw: data.clone(),
        }
    }

    /// Whether the device is in white mode rather than color mode.
    ///
    /// The two are mutually exclusive: a non-zero temperature means the color
    /// in the same reply is not what is lit.
    #[must_use]
    pub fn is_white(&self) -> bool {
        self.color_temp_kelvin.is_some_and(|k| k > 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn data(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("valid json")
    }

    fn id() -> DeviceId {
        DeviceId::new("aa:bb:cc:dd:ee:ff")
    }

    #[test]
    fn reads_the_documented_reply() {
        let status = DeviceStatus::from_data(
            id(),
            &data(
                r#"{"onOff":1,"brightness":100,"color":{"r":255,"g":0,"b":0},
                    "colorTemInKelvin":0}"#,
            ),
        );
        assert_eq!(status.on, Some(true));
        assert_eq!(status.brightness, Some(100));
        assert_eq!(status.color, Some([255, 0, 0]));
        assert!(!status.is_white());
    }

    #[test]
    fn white_mode_is_visible_in_the_reply() {
        let status = DeviceStatus::from_data(
            id(),
            &data(
                r#"{"onOff":1,"brightness":50,"color":{"r":0,"g":0,"b":0},"colorTemInKelvin":7200}"#,
            ),
        );
        assert!(status.is_white());
        assert_eq!(status.color, Some([0, 0, 0]));
    }

    #[test]
    fn a_partial_reply_loses_nothing() {
        // The undocumented `status` command answers with a shape of its own.
        let status = DeviceStatus::from_data(
            id(),
            &data(r#"{"onOff":1,"brightness":75,"pt":"uwABsQEK"}"#),
        );
        assert_eq!(status.on, Some(true));
        assert_eq!(status.color, None);
        assert_eq!(status.color_temp_kelvin, None);
        assert_eq!(status.raw["pt"], "uwABsQEK");
    }
}
