//! The allocator, checked against the catalog.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use govee_toolkit::Catalog;

use super::*;
use crate::patch::Patch;

/// A zone device, and a device with no zone.
const ZONES: &str = "H61A0";
const PLAIN: &str = "H6008";

fn catalog() -> Catalog {
    Catalog::embedded().expect("the embedded catalog parses")
}

fn id(last: &str) -> DeviceId {
    DeviceId::new(format!("AA:BB:CC:DD:EE:{last}"))
}

fn plan(text: &str, layout: Layout, scanned: &[(DeviceId, &str)]) -> Plan {
    let catalog = catalog();
    let devices: Vec<(DeviceId, &Device)> = scanned
        .iter()
        .map(|(id, sku)| (id.clone(), catalog.device(sku).expect("the SKU resolves")))
        .collect();
    let candidates: Vec<Candidate<'_>> = devices
        .iter()
        .map(|(id, device)| Candidate {
            id,
            device,
            name: None,
            groups: &[],
        })
        .collect();
    let patch = Patch::parse(text, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    patch
        .plan(
            &candidates,
            layout,
            PortAddress::new(0).expect("0 fits"),
            |sku| catalog.device(sku).ok(),
        )
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

fn placed(plan: &Plan) -> Vec<(String, u16, u16, u16)> {
    plan.added
        .iter()
        .map(|p| {
            (
                p.device.as_str().to_owned(),
                p.universe.get(),
                p.address,
                p.width,
            )
        })
        .collect()
}

/// A first scan writes the rig from address 1 up, in the order the scan
/// answered.
#[test]
fn a_first_scan_packs_the_devices_from_address_one() {
    let plan = plan(
        "patch: []",
        Layout::Fixed(Personality::Full),
        &[(id("01"), PLAIN), (id("02"), ZONES)],
    );
    assert_eq!(
        placed(&plan),
        vec![
            ("AA:BB:CC:DD:EE:01".to_owned(), 0, 1, 6),
            ("AA:BB:CC:DD:EE:02".to_owned(), 0, 7, 6),
        ]
    );
    assert_eq!(plan.kept, 0);
}

/// The address an operator patched on a desk is the one thing a second
/// scan must not move, and a disabled fixture keeps its channels.
#[test]
fn a_second_scan_keeps_every_address_and_adds_below_the_gap() {
    let text = format!(
        "patch:\n  - device: \"{}\"\n    sku: {ZONES}\n    enabled: false\n    universe: 0\n    address: 33\n    personality: segment\n",
        id("02")
    );
    let plan = plan(
        &text,
        Layout::Fixed(Personality::Full),
        &[(id("01"), PLAIN), (id("02"), ZONES)],
    );
    assert_eq!(
        placed(&plan),
        vec![("AA:BB:CC:DD:EE:01".to_owned(), 0, 1, 6)]
    );
    assert_eq!(plan.kept, 1);
}

/// A device the file carries is left alone, whatever the scan reports.
#[test]
fn a_patched_device_is_never_added_twice() {
    let text = format!(
        "patch:\n  - device: \"{}\"\n    sku: {PLAIN}\n    universe: 0\n    address: 1\n    personality: full\n",
        id("01")
    );
    let plan = plan(
        &text,
        Layout::Fixed(Personality::Full),
        &[(id("01"), PLAIN)],
    );
    assert!(plan.added.is_empty());
}

/// `widest` takes the zone table where the device serves one.
#[test]
fn the_widest_layout_takes_the_zone_table() {
    let plan = plan("patch: []", Layout::Widest, &[(id("02"), ZONES)]);
    let [placement] = plan.added.as_slice() else {
        panic!("one entry")
    };
    assert!(placement.width > 6, "{} channels", placement.width);
    assert_ne!(placement.personality, Personality::Full);
}

/// A device that serves the layout through nothing is reported, and the
/// scan places every other device.
#[test]
fn a_device_that_serves_no_such_table_is_skipped() {
    let plan = plan(
        "patch: []",
        Layout::Fixed(Personality::Pixel),
        &[(id("01"), PLAIN)],
    );
    assert!(plan.added.is_empty());
    assert_eq!(plan.skipped.len(), 1);
}

/// An entry nothing sizes would hand its channels to the next fixture.
#[test]
fn an_entry_with_no_sku_and_no_device_is_refused() {
    let catalog = catalog();
    let text = format!(
        "patch:\n  - device: \"{}\"\n    enabled: false\n    universe: 0\n    address: 1\n    personality: full\n",
        id("09")
    );
    let patch = Patch::parse(&text, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let errors = patch
        .plan(
            &[],
            Layout::Widest,
            PortAddress::new(0).expect("0 fits"),
            |_| None,
        )
        .expect_err("nothing states its width");
    assert!(matches!(errors.as_slice(), [Error::Unsized { .. }]));
    let _ = catalog;
}
