//! Invariants every device file in the repository must hold.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use govee_toolkit::codec::{ArgSpec, Bounds, Catalog, Mode, Role, validate};

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

/// A device file and the shared table it includes, as the loader sees them.
const FAMILY: &str = "schema_version: 1\nfamily: shared\ncommands:\n  ble:\n    ping:\n      \
     frame: \"33 01 <pad:20> <xor>\"\n";

fn device_including(include: &str, own_commands: &str) -> String {
    format!(
        "schema_version: 1\nsku: HINC\nfamily: test\nname: Test\ncapabilities: {{}}\n\
         include: [{include}]\ncommands:\n  ble:\n{own_commands}"
    )
}

#[test]
fn an_included_table_reaches_the_device_that_includes_it() {
    let catalog = Catalog::from_sources_with(
        [("d.yaml", device_including("shared", "").as_str())],
        [("f.yaml", FAMILY)],
    )
    .expect("it loads");

    let device = catalog.device("HINC").expect("the device is there");
    // Nothing downstream can tell an included command from a local one.
    assert!(device.commands.get(Mode::Ble).contains_key("ping"));
}

#[test]
fn including_a_family_nothing_declares_is_an_error() {
    let error = Catalog::from_sources_with(
        [("d.yaml", device_including("absent", "").as_str())],
        [("f.yaml", FAMILY)],
    )
    .expect_err("the include names nothing");

    assert_eq!(error.code(), "unknown_family");
}

#[test]
fn a_command_declared_twice_is_an_error_rather_than_an_override() {
    let own = "    ping:\n      frame: \"33 02 <pad:20> <xor>\"\n      \
               notes: \"See docs/protocol/ble.md.\"\n";
    let error = Catalog::from_sources_with(
        [("d.yaml", device_including("shared", own).as_str())],
        [("f.yaml", FAMILY)],
    )
    .expect_err("the file and the family both declare `ping`");

    assert_eq!(error.code(), "duplicate_command");
}

/// A shared table that takes its bounds from the device that includes it.
const FAMILY_BOUNDS: &str = "schema_version: 1\nfamily: shared-bounds\ncommands:\n  ble:\n    \
     dim:\n      role: brightness\n      frame: \"33 04 ${level} <pad:20> <xor>\"\n      args:\n        \
     level: { type: int, range: brightness.range, role: brightness }\n";

fn device_with_capabilities(capabilities: &str) -> String {
    format!(
        "schema_version: 1\nsku: HCAP\nfamily: test\nname: Test\n\
         capabilities:\n{capabilities}include: [shared-bounds]\ncommands: {{}}\n"
    )
}

#[test]
fn capability_bounds_reach_a_command_from_the_device_that_includes_it() {
    let device = device_with_capabilities("  brightness: { range: [1, 80] }\n");
    let catalog =
        Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", FAMILY_BOUNDS)])
            .expect("it loads");

    let device = catalog.device("HCAP").expect("the device is there");
    let (_, command) = device
        .entry_for(Mode::Ble, Role::Brightness)
        .expect("the family declares it");
    let ArgSpec::Int { range, .. } = &command.args["level"] else {
        panic!("`level` is an integer");
    };
    assert_eq!(range.as_ref().and_then(Bounds::pair), Some([1, 80]));
}

#[test]
fn capability_bounds_the_device_does_not_declare_are_an_error() {
    let device = device_with_capabilities("  color:\n");
    let error =
        Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", FAMILY_BOUNDS)])
            .expect_err("the device declares no brightness range");

    assert_eq!(error.code(), "capability_bounds");
}

#[test]
fn capability_bounds_on_a_parameter_that_carries_no_pair_are_an_error() {
    let family = "schema_version: 1\nfamily: shared-bounds\ncommands:\n  ble:\n    dim:\n      \
                  frame: \"33 04 ${level} <pad:20> <xor>\"\n      args:\n        \
                  level: { type: int, range: brightness.count }\n";
    let device = device_with_capabilities("  brightness: { range: [1, 80] }\n");
    let error = Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", family)])
        .expect_err("`count` carries no pair");

    assert_eq!(error.code(), "capability_bounds");
}

#[test]
fn a_bound_that_names_no_parameter_is_refused() {
    let family = "schema_version: 1\nfamily: shared-bounds\ncommands:\n  ble:\n    dim:\n      \
                  frame: \"33 04 ${level} <pad:20> <xor>\"\n      args:\n        \
                  level: { type: int, range: brightness }\n";
    let device = device_with_capabilities("  brightness: { range: [1, 80] }\n");

    Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", family)])
        .expect_err("`brightness` alone names no parameter");
}

/// A shared table with a bound and a note one model differs on.
const FAMILY_MUSIC: &str = "schema_version: 1\nfamily: shared-music\ncommands:\n  ble:\n    \
     music:\n      role: music\n      frame: \"33 05 13 ${effect} <pad:20> <xor>\"\n      \
     args:\n        effect: { type: int, range: [0, 7], role: effect }\n      \
     notes: \"Eight effects.\"\n    ping:\n      frame: \"33 01 <pad:20> <xor>\"\n";

fn device_overriding(overrides: &str) -> String {
    format!(
        "schema_version: 1\nsku: HOVR\nfamily: test\nname: Test\ncapabilities: {{}}\n\
         include: [shared-music]\ncommands: {{}}\noverrides:\n  ble:\n{overrides}"
    )
}

fn load_override(overrides: &str) -> govee_toolkit::codec::Result<Catalog> {
    let device = device_overriding(overrides);
    Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", FAMILY_MUSIC)])
}

#[test]
fn an_override_narrows_a_bound_an_included_table_declares() {
    let catalog = load_override(
        "    music:\n      args:\n        effect: { range: [0, 1] }\n      \
         notes: \"Two effects on this unit.\"\n",
    )
    .expect("it loads");

    let device = catalog.device("HOVR").expect("the device is there");
    let (_, command) = device
        .entry_for(Mode::Ble, Role::Music)
        .expect("the family declares it");
    let ArgSpec::Int { range, .. } = &command.args["effect"] else {
        panic!("`effect` is an integer");
    };
    assert_eq!(range.as_ref().and_then(Bounds::pair), Some([0, 1]));
    assert_eq!(command.notes, "Two effects on this unit.");
}

#[test]
fn an_override_drops_a_command_the_model_does_not_have() {
    let catalog = load_override("    ping:\n      drop: true\n").expect("it loads");

    let device = catalog.device("HOVR").expect("the device is there");
    assert!(!device.commands.get(Mode::Ble).contains_key("ping"));
    assert!(device.commands.get(Mode::Ble).contains_key("music"));
}

#[test]
fn an_override_that_changes_nothing_is_an_error() {
    let error = load_override("    music:\n      args:\n        effect: { range: [0, 7] }\n")
        .expect_err("the table already gives those bounds");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_override_naming_a_command_no_included_table_carries_is_an_error() {
    let error =
        load_override("    absent:\n      notes: \"x\"\n").expect_err("no table declares it");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_override_naming_an_argument_the_command_lacks_is_an_error() {
    let error = load_override("    music:\n      args:\n        absent: { range: [0, 1] }\n")
        .expect_err("the command declares no such argument");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_override_of_a_command_the_file_declares_itself_is_an_error() {
    let device = "schema_version: 1\nsku: HOVR\nfamily: test\nname: Test\ncapabilities: {}\n\
                  include: [shared-music]\ncommands:\n  ble:\n    local:\n      \
                  frame: \"33 02 <pad:20> <xor>\"\noverrides:\n  ble:\n    local:\n      \
                  notes: \"x\"\n";
    let error = Catalog::from_sources_with([("d.yaml", device)], [("f.yaml", FAMILY_MUSIC)])
        .expect_err("a local command is edited where it is written");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_override_appends_to_the_note_an_included_table_declares() {
    let catalog = load_override("    music:\n      notes_append: \"2 renders a fixed white.\"\n")
        .expect("it loads");

    let device = catalog.device("HOVR").expect("the device is there");
    let (_, command) = device
        .entry_for(Mode::Ble, Role::Music)
        .expect("the family declares it");
    assert_eq!(command.notes, "Eight effects. 2 renders a fixed white.");
}

#[test]
fn an_override_that_both_replaces_and_appends_a_note_is_an_error() {
    let error = load_override("    music:\n      notes: \"x\"\n      notes_append: \"y\"\n")
        .expect_err("a patch takes one or the other");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_append_to_a_command_the_table_gives_no_note_for_is_an_error() {
    let error = load_override("    ping:\n      notes_append: \"x\"\n")
        .expect_err("there is no note to append to");

    assert_eq!(error.code(), "override");
}

#[test]
fn an_append_the_table_already_says_is_an_error() {
    let error = load_override("    music:\n      notes_append: \"Eight effects.\"\n")
        .expect_err("the table already says it");

    assert_eq!(error.code(), "override");
}
