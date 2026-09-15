//! The local-devices escape hatch: what an overlay replaces, and what it may
//! not.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use govee_toolkit::codec::Catalog;

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

#[test]
fn a_clone_taken_before_an_overlay_keeps_what_it_was_built_with() {
    let mut catalog =
        Catalog::from_sources([("shipped.yaml", yaml("H0007", &[]).as_str())]).expect("catalog");
    let before = catalog.clone();
    let local = yaml("H0007", &[]).replace("name: Test", "name: Local");

    catalog
        .overlay([("local.yaml", local.as_str())])
        .expect("overlay");

    assert_eq!(before.device("H0007").unwrap().name, "Test");
    assert_eq!(catalog.device("H0007").unwrap().name, "Local");
}

#[test]
fn the_embedded_catalog_is_untouched_without_an_overlay() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    let names: Vec<&str> = catalog.devices().map(|d| d.name.as_str()).collect();
    assert!(!names.contains(&"Test"));
}

#[test]
fn a_schema_revision_this_build_does_not_know_is_refused() {
    let mut catalog = Catalog::embedded().expect("embedded catalog");
    let sku = catalog.devices().next().expect("a device").sku.clone();
    let future = yaml(&sku, &[]).replace("schema_version: 1", "schema_version: 2");

    let error = catalog
        .overlay([("from-a-later-build.yaml", future.as_str())])
        .expect_err("a newer revision cannot be read by these rules");

    assert_eq!(error.code(), "schema_version");
}
