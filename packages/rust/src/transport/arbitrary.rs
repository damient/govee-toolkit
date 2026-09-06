//! Generators for the property tests that read what a device sent.
//!
//! Any host on the network, and any device in Bluetooth range, can send a
//! reply. The parsers must stay total against those bytes: they return
//! `Option` and they never index.

use proptest::prelude::*;

/// An arbitrary JSON value, shallow enough to stay fast.
pub(crate) fn json() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::from),
        any::<i64>().prop_map(serde_json::Value::from),
        any::<f64>().prop_filter_map("finite", |f| serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)),
        ".{0,24}".prop_map(serde_json::Value::from),
    ];
    leaf.prop_recursive(3, 24, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(serde_json::Value::from),
            prop::collection::btree_map(key(), inner, 0..6)
                .prop_map(|m| serde_json::Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Object keys, weighted towards the names the protocol uses so that a
/// generated value reaches past the first `get`.
fn key() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => prop::sample::select(vec![
            "msg", "cmd", "data", "device", "sku", "ip", "onOff", "brightness",
            "color", "r", "g", "b", "colorTemInKelvin", "pt",
            "bleVersionHard", "bleVersionSoft", "wifiVersionHard", "wifiVersionSoft",
        ])
        .prop_map(ToOwned::to_owned),
        1 => "[a-zA-Z]{1,8}".prop_map(|s| s),
    ]
}
