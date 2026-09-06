//! What a device reports about itself.
//!
//! `devStatus` and the undocumented `status` of `docs/protocol/lan.md` §2.2
//! both land here. Every field is optional, because this crate cannot know
//! which ones a firmware fills in, and `raw` keeps what it did not recognize.

use std::collections::BTreeMap;

use crate::codec::args::ArgValue;
use crate::codec::{ArgRole, Captured};
use crate::transport::DeviceId;

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
    /// The whole reply: `msg.data` for a mode that answers JSON, every captured
    /// field for one that answers frames. Undocumented fields stay reachable
    /// here, the frozen `pt` descriptor of §2.2 among them.
    pub raw: serde_json::Value,
}

impl DeviceStatus {
    /// Read one out of a reply's `msg.data`.
    #[must_use]
    pub fn from_data(id: DeviceId, data: serde_json::Value) -> Self {
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
            raw: data,
        }
    }

    /// Read one out of what a command's `reply:` layouts captured.
    ///
    /// `roles` says which captured field is which, so no field name reaches
    /// this code. A field no role claims stays in `raw`.
    #[must_use]
    pub fn from_captured(
        id: DeviceId,
        captured: &Captured,
        roles: &BTreeMap<String, ArgRole>,
    ) -> Self {
        let int = |role: ArgRole| {
            roles
                .iter()
                .find(|(_, claimed)| **claimed == role)
                .and_then(|(name, _)| captured.get(name))
                .and_then(|value| match value {
                    ArgValue::Int(v) => Some(*v),
                    _ => None,
                })
        };

        Self {
            id,
            on: int(ArgRole::On).map(|v| v != 0),
            brightness: int(ArgRole::Brightness),
            color: None,
            color_temp_kelvin: None,
            raw: captured.to_json(),
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
    fn a_captured_reply_is_read_by_role_and_loses_nothing() {
        let mut captured = Captured::new();
        captured.insert("lit", ArgValue::Int(1));
        captured.insert("level", ArgValue::Int(64));
        captured.insert("segments", ArgValue::Int(15));
        let roles = BTreeMap::from([
            ("lit".to_owned(), ArgRole::On),
            ("level".to_owned(), ArgRole::Brightness),
        ]);

        let status = DeviceStatus::from_captured(id(), &captured, &roles);
        assert_eq!(status.on, Some(true));
        assert_eq!(status.brightness, Some(64));
        assert_eq!(status.raw["segments"], 15);
    }

    #[test]
    fn a_reply_no_role_claims_leaves_the_modelled_fields_empty() {
        let mut captured = Captured::new();
        captured.insert("version", ArgValue::Text("1.02.00".to_owned()));

        let status = DeviceStatus::from_captured(id(), &captured, &BTreeMap::new());
        assert_eq!(status.on, None);
        assert_eq!(status.brightness, None);
        assert_eq!(status.raw["version"], "1.02.00");
    }

    #[test]
    fn reads_the_documented_reply() {
        let status = DeviceStatus::from_data(
            id(),
            data(
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
            data(
                r#"{"onOff":1,"brightness":50,"color":{"r":0,"g":0,"b":0},"colorTemInKelvin":7200}"#,
            ),
        );
        assert!(status.is_white());
        assert_eq!(status.color, Some([0, 0, 0]));
    }

    #[test]
    fn a_partial_reply_loses_nothing() {
        // The undocumented `status` command answers with a shape of its own.
        let status =
            DeviceStatus::from_data(id(), data(r#"{"onOff":1,"brightness":75,"pt":"uwABsQEK"}"#));
        assert_eq!(status.on, Some(true));
        assert_eq!(status.color, None);
        assert_eq!(status.color_temp_kelvin, None);
        assert_eq!(status.raw["pt"], "uwABsQEK");
    }

    proptest::proptest! {
        #[test]
        fn any_reply_is_read_without_loss(value in crate::transport::arbitrary::json()) {
            let status = DeviceStatus::from_data(id(), value.clone());
            proptest::prop_assert_eq!(&status.raw, &value);
        }
    }
}
