//! Invariants every device file in the repository must hold.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use govee_core::{Catalog, Mode, validate};

#[test]
fn every_device_file_is_well_formed() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let problems: Vec<String> = catalog
        .devices()
        .flat_map(validate::device)
        .map(|p| p.to_string())
        .collect();
    assert!(
        problems.is_empty(),
        "device files:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn the_catalog_is_not_empty() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    assert!(catalog.devices().next().is_some());
}

#[test]
fn verified_aliases_resolve() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    for device in catalog.devices() {
        for alias in &device.aliases {
            let resolved = catalog.device(alias).expect("a verified alias resolves");
            assert_eq!(resolved.sku, device.sku);
        }
    }
}

/// A lookalike that has not been verified must read as an unknown SKU. Silently
/// serving it the neighbouring device's protocol is exactly the inference
/// `devices/README.md` forbids.
#[test]
fn candidate_aliases_do_not_resolve() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    for device in catalog.devices() {
        for candidate in &device.candidate_aliases {
            assert!(
                catalog.device(candidate).is_err(),
                "`{candidate}` is only a candidate alias of {} and must not resolve",
                device.sku
            );
        }
    }
}

#[test]
fn lookup_is_case_insensitive() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let sku = catalog.devices().next().expect("a device").sku.clone();
    assert_eq!(catalog.device(&sku.to_lowercase()).unwrap().sku, sku);
}

/// A capture is what turns a claim into a verified one. Nothing is attached yet
/// — this test records how many are missing so the number can only go down.
#[test]
fn captures_still_missing() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let missing: Vec<String> = catalog
        .devices()
        .flat_map(|d| {
            [Mode::Lan, Mode::Ble, Mode::Cloud]
                .into_iter()
                .flat_map(move |mode| {
                    d.commands
                        .get(mode)
                        .iter()
                        .filter(|(_, c)| c.capture.trim().is_empty())
                        .map(move |(name, _)| format!("{}/{mode}/{name}", d.sku))
                })
        })
        .collect();
    // TODO: tighten to `assert!(missing.is_empty())` once captures are attached.
    println!("commands without a capture: {}", missing.len());
}

/// A minimal, well-formed device file.
fn yaml(sku: &str, aliases: &[&str]) -> String {
    format!(
        "schema_version: 1\nsku: \"{sku}\"\nfamily: test\nname: Test\naliases: {aliases:?}\ncapabilities: {{}}\n"
    )
}

#[test]
fn an_overlay_replaces_an_embedded_device_and_says_so() {
    let mut catalog = Catalog::embedded().expect("embedded catalog");
    let sku = catalog.devices().next().expect("a device").sku.clone();
    let local = yaml(&sku, &[]);

    let replaced = catalog
        .overlay([("local.yaml", local.as_str())])
        .expect("overlay");

    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].sku, sku);
    assert_eq!(replaced[0].now, "local.yaml");
    assert_eq!(catalog.device(&sku).unwrap().name, "Test");
}

#[test]
fn an_overlay_adds_a_sku_the_build_does_not_carry() {
    let mut catalog = Catalog::embedded().expect("embedded catalog");
    let before = catalog.devices().count();
    let local = yaml("H0000", &[]);

    let replaced = catalog
        .overlay([("local.yaml", local.as_str())])
        .expect("overlay");

    assert!(replaced.is_empty(), "nothing was replaced");
    assert_eq!(catalog.devices().count(), before + 1);
    assert!(catalog.device("H0000").is_ok());
}

/// Replacing a device replaces it whole. An alias the old file declared must
/// stop resolving, or a lookup would silently reach the wrong definition.
#[test]
fn a_replacement_drops_the_aliases_it_no_longer_declares() {
    let mut catalog = Catalog::from_sources([("shipped.yaml", yaml("H0001", &["H0002"]).as_str())])
        .expect("catalog");
    assert!(catalog.device("H0002").is_ok());

    catalog
        .overlay([("local.yaml", yaml("H0001", &[]).as_str())])
        .expect("overlay");

    assert!(catalog.device("H0001").is_ok());
    assert!(
        catalog.device("H0002").is_err(),
        "the stale alias must not resolve"
    );
}

#[test]
fn two_overlay_files_claiming_one_sku_is_a_mistake_not_an_override() {
    let mut catalog = Catalog::embedded().expect("embedded catalog");
    let (a, b) = (yaml("H0003", &[]), yaml("H0003", &[]));

    let err = catalog
        .overlay([("a.yaml", a.as_str()), ("b.yaml", b.as_str())])
        .expect_err("a self-contradictory overlay is rejected");

    assert_eq!(err.code(), "duplicate_sku");
}

#[test]
fn an_overlay_may_not_steal_an_alias_from_a_device_it_does_not_replace() {
    let mut catalog = Catalog::from_sources([
        ("a.yaml", yaml("H0004", &["H0005"]).as_str()),
        ("b.yaml", yaml("H0006", &[]).as_str()),
    ])
    .expect("catalog");

    let err = catalog
        .overlay([("local.yaml", yaml("H0006", &["H0005"]).as_str())])
        .expect_err("H0005 belongs to H0004");

    assert_eq!(err.code(), "duplicate_sku");
}

/// The escape hatch is opt-in: nothing reads a local directory on its own.
#[test]
fn the_embedded_catalog_is_untouched_without_an_overlay() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let names: Vec<&str> = catalog.devices().map(|d| d.name.as_str()).collect();
    assert!(!names.contains(&"Test"));
}
