//! What the configuration hands a binding, and what it must never hand one.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use govee_toolkit::codec::Mode;
use govee_toolkit::{Config, Env};

#[test]
fn the_serialized_configuration_carries_what_a_binding_reads() {
    let config = Config::default();

    let json = serde_json::to_value(&config).unwrap();

    assert_eq!(json["defaults"]["modes"][0], Mode::Lan.as_str());
    assert!(json["devices"].as_object().unwrap().is_empty());
    assert!(json["stream"]["fallback_hz"].is_number());
}

#[test]
fn the_serialized_configuration_carries_no_credential() {
    let config = Config {
        env: Env::from_pairs([("GOVEE_API_KEY", "a-secret")]),
        ..Config::default()
    };

    let json = serde_json::to_string(&config).unwrap();

    assert!(!json.contains("a-secret"));
    assert!(!json.contains("env"));
}
