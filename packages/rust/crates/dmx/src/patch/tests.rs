//! The patch, checked against a fixture rig and against every refusal.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeMap;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::{Catalog, Device};

use super::{Error, Patch, PortAddress, SignalLoss};
use crate::profile::Personality;

/// Three fixtures on one universe: two segment devices and one `full` device.
const RIG: &str = include_str!("../../tests/fixtures/patch.yaml");

fn catalog() -> Catalog {
    Catalog::embedded().expect("the embedded catalog parses")
}

fn parse(text: &str) -> Patch {
    Patch::parse(text, "patch.yaml").unwrap_or_else(|e| panic!("{e}"))
}

/// The devices the fixture rig names, by the SKU each one answers as.
fn rig_devices(catalog: &Catalog) -> BTreeMap<DeviceId, &Device> {
    [
        ("AA:BB:CC:DD:EE:01", "H61A0"),
        ("AA:BB:CC:DD:EE:02", "H61A0"),
        ("AA:BB:CC:DD:EE:03", "H6008"),
    ]
    .into_iter()
    .map(|(id, sku)| {
        (
            DeviceId::new(id),
            catalog.device(sku).expect("the SKU resolves"),
        )
    })
    .collect()
}

/// One device, patched however the test spells it.
fn one(sku: &str, entry: &str) -> Result<Vec<(u16, u16, u16)>, Vec<Error>> {
    let catalog = catalog();
    let device = catalog.device(sku).expect("the SKU resolves");
    let patch = parse(&format!("patch:\n{entry}"));
    patch
        .resolve(|_| Some(device), |_| Some(device))
        .map(|rig| spans(&rig))
}

fn spans(rig: &super::Rig) -> Vec<(u16, u16, u16)> {
    rig.fixtures()
        .iter()
        .map(|fixture| {
            (
                fixture.span.universe.get(),
                fixture.span.first,
                fixture.span.last,
            )
        })
        .collect()
}

#[test]
fn the_fixture_rig_loads() {
    let patch = parse(RIG);
    assert_eq!(patch.node.name, "govee-toolkit");
    assert_eq!(patch.node.refresh_secs, 10);
    assert_eq!(patch.patch.len(), 3);
    assert_eq!(patch.patch[0].personality, Personality::Segment);
    assert_eq!(patch.patch[0].max_hz, Some(20.0));
    assert_eq!(patch.patch[0].on_signal_loss, SignalLoss::Hold);
    assert_eq!(patch.patch[1].on_signal_loss, SignalLoss::Black);
    assert_eq!(patch.patch[2].on_signal_loss, SignalLoss::Off);
}

/// The channels the operator patched on the desk are the channels the bridge
/// resolves: 1 to 32, 33 to 64, then 65 to 70.
#[test]
fn the_fixture_rig_resolves_to_the_channels_the_desk_shows() {
    let catalog = catalog();
    let devices = rig_devices(&catalog);
    let rig = parse(RIG)
        .resolve(
            |id| devices.get(id).copied(),
            |sku| catalog.device(sku).ok(),
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(spans(&rig), vec![(0, 1, 32), (0, 33, 64), (0, 65, 70)]);
    assert_eq!(rig.universes(), vec![PortAddress::new(0).expect("0 fits")]);
}

/// A channel a desk shows names the fixture that answers to it, and not only
/// the one that starts there.
#[test]
fn a_channel_names_the_fixture_that_answers_to_it() {
    let catalog = catalog();
    let devices = rig_devices(&catalog);
    let rig = parse(RIG)
        .resolve(
            |id| devices.get(id).copied(),
            |sku| catalog.device(sku).ok(),
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let universe = PortAddress::new(0).expect("0 fits");
    let inside = rig.at(universe, Some(40));
    assert_eq!(inside.len(), 1);
    assert_eq!(inside[0].span.first, 33);
    assert_eq!(rig.at(universe, None).len(), 3);
    assert!(rig.at(universe, Some(71)).is_empty());
    assert!(
        rig.at(PortAddress::new(1).expect("1 fits"), None)
            .is_empty()
    );
}

/// A desk shows one spelling or the other, and both name one address.
#[test]
fn both_spellings_of_one_address_resolve_to_one_port_address() {
    let whole = parse("patch:\n  - { device: A, universe: 275, address: 1, personality: full }");
    let parts = parse(
        "patch:\n  - { device: A, net: 1, subnet: 1, universe: 3, address: 1, personality: full }",
    );
    let address = whole.patch[0].port_address().expect("275 fits");
    assert_eq!(parts.patch[0].port_address(), Ok(address));
    assert_eq!(address.net(), 1);
    assert_eq!(address.sub_uni(), 0x13);
}

#[test]
fn a_port_address_over_15_bits_is_refused() {
    let patch = parse("patch:\n  - { device: A, universe: 32768, address: 1, personality: full }");
    assert!(matches!(
        patch.patch[0].port_address(),
        Err(Error::Address {
            field: "universe",
            value: 32768,
            ..
        })
    ));
}

#[test]
fn a_universe_over_4_bits_beside_a_net_is_refused() {
    let patch = parse(
        "patch:\n  - { device: A, net: 0, subnet: 0, universe: 16, address: 1, personality: full }",
    );
    assert!(matches!(
        patch.patch[0].port_address(),
        Err(Error::Address {
            field: "universe",
            value: 16,
            max: 15,
            ..
        })
    ));
}

#[test]
fn an_address_outside_a_universe_is_refused() {
    for address in ["0", "513"] {
        let entry =
            format!("  - {{ device: A, universe: 0, address: {address}, personality: full }}\n");
        let errors = one("H6008", &entry).expect_err("the address is outside a universe");
        assert!(
            matches!(errors.as_slice(), [Error::StartAddress { .. }]),
            "{errors:?}",
        );
    }
}

/// The patch is never truncated to fit, and it never spills into the next
/// universe. See the open question in `docs/dmx.md`.
#[test]
fn a_fixture_past_the_end_of_its_universe_is_refused() {
    let entry = "  - { device: A, universe: 0, address: 500, personality: segment }\n";
    let errors = one("H61A0", entry).expect_err("32 channels do not fit from 500");
    let [Error::PastUniverse { span, .. }] = errors.as_slice() else {
        panic!("{errors:?}");
    };
    assert_eq!((span.first, span.last), (500, 531));
}

#[test]
fn two_fixtures_on_one_channel_are_refused() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: segment }\n  \
                 - { device: B, universe: 0, address: 32, personality: full }\n";
    let errors = one("H61A0", entry).expect_err("channel 32 is taken twice");
    let [
        Error::Overlap {
            first_span,
            second_span,
            ..
        },
    ] = errors.as_slice()
    else {
        panic!("{errors:?}");
    };
    assert_eq!((first_span.last, second_span.first), (32, 32));
}

/// Two fixtures at one address on two universes drive two desks, and neither
/// touches the other.
#[test]
fn one_address_on_two_universes_is_no_overlap() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: segment }\n  \
                 - { device: B, universe: 1, address: 1, personality: segment }\n";
    let spans = one("H61A0", entry).expect("two universes do not overlap");
    assert_eq!(spans, vec![(0, 1, 32), (1, 1, 32)]);
}

#[test]
fn a_device_patched_twice_is_refused() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: full }\n  \
                 - { device: A, universe: 1, address: 1, personality: full }\n";
    let errors = one("H6008", entry).expect_err("one device answers one look");
    assert!(
        matches!(errors.as_slice(), [Error::Twice { .. }]),
        "{errors:?}",
    );
}

#[test]
fn a_personality_the_device_serves_through_nothing_is_refused() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: segment }\n";
    let errors = one("H6008", entry).expect_err("a bulb paints no zone");
    assert!(
        matches!(errors.as_slice(), [Error::Unserved { .. }]),
        "{errors:?}",
    );
}

#[test]
fn a_device_nothing_found_is_refused() {
    let patch = parse("patch:\n  - { device: A, universe: 0, address: 1, personality: full }");
    let errors = patch
        .resolve(|_| None, |_| None)
        .expect_err("nothing says what A is");
    assert!(
        matches!(errors.as_slice(), [Error::Unknown { .. }]),
        "{errors:?}",
    );
}

/// An operator corrects the whole patch once, not one line per run.
#[test]
fn every_fault_is_reported_at_once() {
    let entry = "  - { device: A, universe: 0, address: 0, personality: full }\n  \
                 - { device: B, universe: 0, address: 600, personality: full }\n";
    let errors = one("H6008", entry).expect_err("two addresses are outside a universe");
    assert_eq!(errors.len(), 2, "{errors:?}");
}

#[test]
fn an_unknown_key_is_refused() {
    let text = "patch:\n  - { device: A, universe: 0, address: 1, personality: full, colour: red }";
    let error = Patch::parse(text, "patch.yaml").expect_err("`colour` is not a key");
    let Error::Parse { reason, .. } = error else {
        panic!("{error:?}");
    };
    assert!(reason.contains("colour"), "{reason}");
}

#[test]
fn an_unknown_personality_names_the_ones_that_exist() {
    let text = "patch:\n  - { device: A, universe: 0, address: 1, personality: pixels }";
    let error = Patch::parse(text, "patch.yaml").expect_err("`pixels` is no personality");
    let Error::Parse { reason, .. } = error else {
        panic!("{error:?}");
    };
    assert!(reason.contains("segment"), "{reason}");
}

/// An empty patch is a node that drives nothing, which is what the defaults
/// have to produce.
#[test]
fn an_empty_patch_takes_every_default() {
    let patch = parse("");
    assert_eq!(patch.node.name, "govee-toolkit");
    assert_eq!(patch.node.refresh_secs, 10);
    assert!(patch.node.bind.is_unspecified());
    assert!(patch.patch.is_empty());
}

/// A disabled fixture takes no frame, and the channels it holds stay its
/// own: the addresses the desk carries do not move because one fixture left
/// the rig.
#[test]
fn a_disabled_entry_is_reserved_and_never_driven() {
    let catalog = catalog();
    let device = catalog.device("H6008").expect("the SKU resolves");
    let text = "patch:\n  \
        - { device: A, enabled: false, universe: 0, address: 1, personality: full }\n  \
        - { device: B, universe: 0, address: 7, personality: full }\n";
    let rig = parse(text)
        .resolve(|_| Some(device), |_| Some(device))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(rig.fixtures().len(), 1);
    assert_eq!(rig.fixtures()[0].entry.device, DeviceId::new("B"));
    assert_eq!(rig.reserved().len(), 1);
    assert_eq!(rig.reserved()[0].span.first, 1);
}

/// A disabled fixture takes no frame, so a driven fixture can cover the
/// channels it holds. The patch command is what keeps a new entry off them.
#[test]
fn a_fixture_over_a_disabled_one_is_allowed() {
    let catalog = catalog();
    let device = catalog.device("H6008").expect("the SKU resolves");
    let text = "patch:\n  \
        - { device: A, enabled: false, universe: 0, address: 1, personality: full }\n  \
        - { device: B, universe: 0, address: 3, personality: full }\n";
    let rig = parse(text)
        .resolve(|_| Some(device), |_| Some(device))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(rig.fixtures().len(), 1);
    assert_eq!(rig.reserved().len(), 1);
}

/// Two fixtures on one span under one personality answer to one channel
/// table. An operator patches a pair that way to drive it from one set of
/// channels.
#[test]
fn a_clone_on_one_address_is_allowed() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: full }\n  \
                 - { device: B, universe: 0, address: 1, personality: full }\n";
    let spans = one("H6008", entry).expect("one table on one span drives both");
    assert_eq!(spans, vec![(0, 1, 6), (0, 1, 6)]);
}

/// A clone shares the whole span. Two fixtures that share a part of one
/// answer to one channel under two tables, and that stays a fault.
#[test]
fn a_part_of_a_span_is_no_clone() {
    let entry = "  - { device: A, universe: 0, address: 1, personality: full }\n  \
                 - { device: B, universe: 0, address: 4, personality: full }\n";
    let errors = one("H6008", entry).expect_err("channels 4 to 6 mean two things");
    assert!(
        matches!(errors.as_slice(), [Error::Overlap { .. }]),
        "{errors:?}"
    );
}

/// A disabled entry carries the SKU that sizes it, so its channels are held
/// while the device is off the network.
#[test]
fn a_disabled_entry_is_sized_by_its_sku() {
    let catalog = catalog();
    let text = "patch:\n  \
        - { device: A, sku: H6008, enabled: false, universe: 0, address: 1, personality: full }\n";
    let rig = parse(text)
        .resolve(|_| None, |sku| catalog.device(sku).ok())
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(rig.fixtures().is_empty());
    assert_eq!(rig.reserved()[0].span.last, 6);
}

/// `hold:` is what keeps a fixture out of a rig while its device answers: the
/// key is on the entry, and the scan reads it before it writes a state.
#[test]
fn an_entry_can_hold_its_own_state() {
    let text = "patch:\n  \
        - { device: A, hold: true, enabled: false, universe: 0, address: 1, personality: full }\n";
    let patch = parse(text);
    assert!(patch.patch[0].hold);
    assert!(!patch.patch[0].enabled);
}
