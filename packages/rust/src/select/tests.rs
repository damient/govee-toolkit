#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::*;
use crate::codec::Mode;

fn device(id: &str, sku: &str, name: Option<&str>) -> Device {
    on(id, sku, name, vec![Mode::Lan])
}

fn on(id: &str, sku: &str, name: Option<&str>, modes: Vec<Mode>) -> Device {
    Device {
        id: DeviceId::new(id),
        sku: sku.to_owned(),
        name: name.map(ToOwned::to_owned),
        modes,
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
    matches(selector, &rig(), None)
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
    assert_eq!(parse("h6008").ok(), Some(Selector::Sku("H6008".to_owned())));
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
        matches(&sku, &rig(), None).err(),
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

// A SKU and a name answer the devices the mode drives. An identity answers
// itself, and the command that follows reports the mode.
#[test]
fn a_mode_narrows_a_sku_and_a_name_and_leaves_an_identity_alone() {
    let known = vec![
        on(
            "1C:8B:C4:A2:C0:46:64:6E",
            "H6008",
            Some("kitchen"),
            vec![Mode::Lan],
        ),
        on(
            "AA:BB:CC:DD:EE:FF:00:11",
            "H6008",
            Some("hall"),
            vec![Mode::Ble],
        ),
    ];
    let sku = Selector::Sku("H6008".to_owned());
    let over_lan = matches(&sku, &known, Some(Mode::Lan)).expect("the model answers over `lan`");
    assert_eq!(
        over_lan.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["1C:8B:C4:A2:C0:46:64:6E"]
    );

    let hall = Selector::Name("hall".to_owned());
    assert_eq!(
        matches(&hall, &known, Some(Mode::Lan)).err(),
        Some(Error::NotOnMode {
            target: "name:hall".to_owned(),
            mode: Mode::Lan,
        })
    );

    let id = Selector::Id(DeviceId::new("AA:BB:CC:DD:EE:FF:00:11"));
    assert_eq!(
        matches(&id, &known, Some(Mode::Lan)).expect("an identity answers itself"),
        [DeviceId::new("AA:BB:CC:DD:EE:FF:00:11")]
    );
}

// A target that matches nothing at all is no mode fault.
#[test]
fn a_mode_never_turns_an_empty_match_into_a_mode_fault() {
    let sku = Selector::Sku("H9999".to_owned());
    assert_eq!(
        matches(&sku, &rig(), Some(Mode::Ble)).err(),
        Some(Error::NoMatch {
            target: "sku:H9999".to_owned()
        })
    );
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

fn named_config() -> crate::config::Config {
    serde_norway::from_str(
        r#"
devices:
  "1C:8B:C4:A2:C0:46:64:6E":
    name: kitchen
  "AA:BB:CC:DD:EE:FF:00:11":
    name: twin
  "11:22:33:44:55:66:77:88":
    name: TWIN
  "22:33:44:55:66:77:88:99":
    name: "33:44:55:66:77:88:99:AA"
"#,
    )
    .expect("the configuration parses")
}

fn one(target: &str) -> Result<DeviceId, Error> {
    Selector::one(target, &named_config())
}

#[test]
fn one_reads_a_name_the_configuration_gives() {
    let kitchen = DeviceId::new("1C:8B:C4:A2:C0:46:64:6E");
    assert_eq!(one("kitchen"), Ok(kitchen.clone()));
    assert_eq!(one("name:KITCHEN"), Ok(kitchen));
}

#[test]
fn one_reads_an_identity() {
    let id = DeviceId::new("AA:BB:CC:DD:EE:FF");
    assert_eq!(one("AA:BB:CC:DD:EE:FF"), Ok(id.clone()));
    assert_eq!(one("id:AA:BB:CC:DD:EE:FF"), Ok(id));
}

#[test]
fn one_refuses_what_is_not_one_device() {
    assert!(matches!(one("name:attic"), Err(Error::NoMatch { .. })));
    assert!(matches!(one("twin"), Err(Error::Several { .. })));
    assert!(matches!(one("sku:H6008"), Err(Error::NotOne { .. })));
    assert!(matches!(one("name:"), Err(Error::EmptyValue { .. })));
    assert!(matches!(
        one("33:44:55:66:77:88:99:AA"),
        Err(Error::Ambiguous { .. })
    ));
}
