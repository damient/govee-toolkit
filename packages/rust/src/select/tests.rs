#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::*;
use crate::codec::Mode;

fn device(id: &str, sku: &str, name: Option<&str>) -> Device {
    Device {
        id: DeviceId::new(id),
        sku: sku.to_owned(),
        name: name.map(ToOwned::to_owned),
        modes: vec![Mode::Lan],
        health: BTreeMap::new(),
    }
}

fn rig() -> Vec<Device> {
    vec![
        device("1C:8B:C4:A2:C0:46:64:6E", "H6008", Some("kitchen")),
        device("AA:BB:CC:DD:EE:FF:00:11", "H6008", Some("hall")),
        device("11:22:33:44:55:66:77:88", "H61A0", None),
    ]
}

fn catalog() -> Catalog {
    Catalog::embedded().expect("the embedded catalog parses")
}

fn parse(target: &str) -> Result<Selector, Error> {
    Selector::parse(target, &catalog())
}

fn chosen(selector: &Selector) -> Vec<String> {
    matches(selector, &rig())
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_bare_target_takes_the_kind_it_reads_as() {
    assert_eq!(
        parse("1c:8b:c4:a2:c0:46:64:6e").ok(),
        Some(Selector::Id(DeviceId::new("1C:8B:C4:A2:C0:46:64:6E")))
    );
    assert_eq!(
        parse("h6008").ok(),
        Some(Selector::Sku("H6008".to_owned()))
    );
    assert_eq!(
        parse("kitchen").ok(),
        Some(Selector::Name("kitchen".to_owned()))
    );
}

// The handle a platform gives a Bluetooth peripheral carries dashes, and a
// name never reads as one.
#[test]
fn a_peripheral_handle_reads_as_an_identity() {
    let handle = "5A3F1B2C-4D6E-7F80-9A1B-2C3D4E5F6071";
    assert!(matches!(parse(handle), Ok(Selector::Id(_))));
    assert!(matches!(parse("living-room"), Ok(Selector::Name(_))));
}

#[test]
fn a_prefix_states_the_kind_the_shape_would_not_give() {
    assert_eq!(
        parse("name:H6008").ok(),
        Some(Selector::Name("H6008".to_owned()))
    );
    assert_eq!(
        parse("sku:h61a0").ok(),
        Some(Selector::Sku("H61A0".to_owned()))
    );
}

#[test]
fn a_target_with_nothing_in_it_is_refused() {
    assert_eq!(parse("   ").err(), Some(Error::Empty));
    assert_eq!(
        parse("sku:").err(),
        Some(Error::EmptyValue {
            prefix: "sku".to_owned()
        })
    );
}

#[test]
fn a_sku_matches_every_device_of_that_model() {
    assert_eq!(
        chosen(&Selector::Sku("H6008".to_owned())),
        ["1C:8B:C4:A2:C0:46:64:6E", "AA:BB:CC:DD:EE:FF:00:11"]
    );
}

// A name is exact: `hall` does not reach a device named `hallway`.
#[test]
fn a_name_matches_the_whole_name_and_ignores_case() {
    assert_eq!(
        chosen(&Selector::Name("KITCHEN".to_owned())),
        ["1C:8B:C4:A2:C0:46:64:6E"]
    );
    assert!(chosen(&Selector::Name("kit".to_owned())).is_empty());
}

#[test]
fn an_identity_selects_itself_whether_a_scan_found_it_or_not() {
    let unknown = Selector::Id(DeviceId::new("00:00:00:00:00:00:00:01"));
    assert_eq!(chosen(&unknown), ["00:00:00:00:00:00:00:01"]);
}

#[test]
fn a_sku_or_a_name_that_matches_nothing_is_a_failure() {
    let sku = Selector::Sku("H9999".to_owned());
    assert_eq!(
        matches(&sku, &rig()).err(),
        Some(Error::NoMatch {
            target: "sku:H9999".to_owned()
        })
    );
}

// The catalog says what a SKU is. A name that a model is not named after
// stays a name, whatever its shape.
#[test]
fn a_bare_sku_is_one_the_catalog_is_encoded_under() {
    assert!(matches!(parse("H0000"), Ok(Selector::Name(_))));
    assert!(matches!(parse("h6008"), Ok(Selector::Sku(_))));
}

// An alias is a second name for one model, and `matches` selects the SKU a
// device is encoded under. A bare alias is therefore a name.
#[test]
fn a_bare_alias_is_no_sku() {
    let aliases: Vec<String> = catalog()
        .devices()
        .flat_map(|device| device.aliases.clone())
        .collect();
    for alias in aliases {
        assert!(
            matches!(parse(&alias), Ok(Selector::Name(_))),
            "`{alias}` reads as a SKU"
        );
    }
}

#[test]
fn a_bare_sku_that_a_device_carries_as_a_name_is_refused() {
    let known = vec![device("1C:8B:C4:A2:C0:46:64:6E", "H61A0", Some("H6008"))];
    let selector = Selector::Sku("H6008".to_owned());
    assert_eq!(
        check_ambiguity("H6008", &selector, &known).err(),
        Some(Error::Ambiguous {
            target: "H6008".to_owned(),
            kind: "sku".to_owned(),
        })
    );
    // The prefixed form says which kind is meant, so it is never ambiguous.
    assert_eq!(
        parse("sku:H6008").ok(),
        Some(Selector::Sku("H6008".to_owned()))
    );
}
