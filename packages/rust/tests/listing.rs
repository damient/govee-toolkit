//! The models of `devices/listed/`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use govee_toolkit::codec::{Catalog, Error, Mode, Role, Support, validate};

#[test]
fn every_listed_model_is_well_formed() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let problems: Vec<String> = catalog
        .listed()
        .flat_map(validate::device)
        .map(|p| p.to_string())
        .collect();
    assert!(
        problems.is_empty(),
        "listed models:\n  {}",
        problems.join("\n  ")
    );
}

fn first_of<'a>(catalog: &'a Catalog, listing: &str) -> &'a govee_toolkit::codec::Device {
    catalog
        .listed()
        .find(|device| device.listing.as_deref() == Some(listing))
        .expect("a listed model")
}

#[test]
fn a_dimmable_model_carries_power_and_brightness_and_nothing_else() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let device = first_of(&catalog, "govee-lan-dimmable");
    assert_eq!(
        device.capabilities.names().collect::<Vec<_>>(),
        ["brightness", "power"]
    );
    for mode in [Mode::Lan, Mode::Cloud] {
        for role in [Role::Power, Role::Brightness] {
            assert!(device.command_for(mode, role).is_some(), "{mode} {role:?}");
        }
        for role in [Role::Color, Role::ColorTemp] {
            assert!(device.command_for(mode, role).is_none(), "{mode} {role:?}");
        }
    }
}

#[test]
fn a_listed_model_carries_power_brightness_and_color_and_nothing_else() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let device = first_of(&catalog, "govee-lan");
    assert_eq!(device.verified.by, "");
    assert_eq!(
        device.capabilities.names().collect::<Vec<_>>(),
        ["brightness", "color", "power"]
    );
    for mode in [Mode::Lan, Mode::Cloud] {
        for role in [Role::Power, Role::Brightness, Role::Color] {
            assert!(device.command_for(mode, role).is_some(), "{mode} {role:?}");
        }
        assert!(
            device.command_for(mode, Role::ColorTemp).is_none(),
            "{mode}"
        );
        assert_eq!(device.modes.get(mode).support, Support::Unknown);
    }
    assert_eq!(device.commands.get(Mode::Ble).len(), 0);
}

#[test]
fn a_device_file_wins_over_the_listing() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let listed: Vec<&str> = catalog
        .listings()
        .iter()
        .flat_map(|listing| listing.models.iter().map(|model| model.sku.as_str()))
        .collect();
    for device in catalog.devices() {
        let keys = std::iter::once(&device.sku).chain(&device.aliases);
        for key in keys.filter(|key| listed.contains(&key.as_str())) {
            let found = catalog.device(key).unwrap();
            assert!(found.listing.is_none(), "{key} resolves to the listing");
        }
    }
}

const FAMILY: &str = "schema_version: 1\nfamily: shared\ncommands:\n  lan:\n    power:\n      \
     cmd: turn\n      role: power\n      payload: { value: \"${on}\" }\n      args:\n        \
     on: { type: int, range: [0, 1], role: \"on\" }\n";

fn listing(models: &str) -> String {
    format!(
        "schema_version: 1\nlisting: test\nsource: here\ndevice:\n  family: listed\n  \
         include: [shared]\n  capabilities:\n    power:\nmodels:\n{models}"
    )
}

const FILE: &str = "schema_version: 1\nsku: H0001\nfamily: own\nname: Own\naliases: [H0003]\n\
                    capabilities:\n  power:\n";

#[test]
fn a_model_resolves_with_its_own_sku_and_name() {
    let listing = listing("  - { sku: H0002, name: Two }\n");
    let catalog = Catalog::from_parts([], [("f.yaml", FAMILY)], [("l.yaml", listing.as_str())])
        .expect("catalog");
    let device = catalog.device("h0002").expect("a listed model resolves");
    assert_eq!(
        (device.sku.as_str(), device.name.as_str()),
        ("H0002", "Two")
    );
    assert_eq!(device.listing.as_deref(), Some("test"));
    assert_eq!(catalog.devices().count(), 0);
    assert_eq!(catalog.listed().count(), 1);
}

#[test]
fn a_sku_or_an_alias_of_a_device_file_stays_with_the_file() {
    let listing = listing("  - { sku: H0001, name: One }\n  - { sku: H0003, name: Three }\n");
    let catalog = Catalog::from_parts(
        [("H0001.yaml", FILE)],
        [("f.yaml", FAMILY)],
        [("l.yaml", listing.as_str())],
    )
    .expect("catalog");
    for sku in ["H0001", "H0003"] {
        assert!(catalog.device(sku).unwrap().listing.is_none(), "{sku}");
    }
    assert_eq!(catalog.listed().count(), 0);
}

#[test]
fn a_model_listed_twice_is_an_error() {
    let listing = listing("  - { sku: H0002, name: Two }\n  - { sku: H0002, name: Two }\n");
    let error = Catalog::from_parts([], [("f.yaml", FAMILY)], [("l.yaml", listing.as_str())])
        .expect_err("a duplicate");
    assert!(matches!(error, Error::DuplicateSku { .. }), "{error}");
}

#[test]
fn an_overlay_takes_a_listed_sku_back() {
    let listing = listing("  - { sku: H0001, name: One }\n  - { sku: H0003, name: Three }\n");
    let mut catalog = Catalog::from_parts([], [("f.yaml", FAMILY)], [("l.yaml", listing.as_str())])
        .expect("catalog");
    let replaced = catalog.overlay([("H0001.yaml", FILE)]).expect("overlay");
    assert_eq!(replaced.len(), 1);
    for sku in ["H0001", "H0003"] {
        assert!(catalog.device(sku).unwrap().listing.is_none(), "{sku}");
    }
    assert_eq!(catalog.listed().count(), 0);
}
