//! What one frame asks of one fixture, over the fixture rig of the patch
//! tests.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::net::SocketAddr;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;

use super::Look;
use crate::input::UniverseFrame;
use crate::patch::{Fixture, Patch, Rig};

/// The rig the patch tests load: two `pixel` fixtures at 1 and 32, and one
/// `full` fixture at 63.
const RIG: &str = include_str!("../../tests/fixtures/patch.yaml");

fn source() -> SocketAddr {
    "192.0.2.2:6454".parse().expect("a socket address")
}

fn rig(catalog: &Catalog) -> Rig {
    let patch = Patch::parse(RIG, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let pixel = catalog.device("H61A0").expect("the SKU resolves");
    let full = catalog.device("H6008").expect("the SKU resolves");
    patch
        .resolve(|id| {
            if id == &DeviceId::new("AA:BB:CC:DD:EE:03") {
                Some(full)
            } else {
                Some(pixel)
            }
        })
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

fn catalog() -> Catalog {
    Catalog::embedded().expect("the embedded catalog parses")
}

/// One look, read from a universe the test fills by hand.
fn look(fixture: &Fixture, values: &[u8]) -> Look {
    Look::read(fixture, &UniverseFrame::new(0, source(), values))
}

/// A universe of 512 slots, with `values` written from address `first`.
fn universe(first: u16, values: &[u8]) -> Vec<u8> {
    let mut slots = vec![0u8; 512];
    let start = usize::from(first) - 1;
    slots[start..start + values.len()].copy_from_slice(values);
    slots
}

#[test]
fn the_dimmer_at_zero_powers_the_device_off() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(
        &rig.fixtures()[2],
        &universe(63, &[0, 255, 255, 255, 255, 0]),
    );
    assert!(!look.on);
    assert_eq!(look.brightness, None);
}

/// The dimmer scales into the pair the device file declares, and every other
/// slot goes out as it is.
#[test]
fn a_full_fixture_reads_its_six_channels() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let fixture = &rig.fixtures()[2];
    let look = look(fixture, &universe(63, &[255, 10, 20, 30, 255, 0]));
    assert!(look.on);
    assert_eq!(look.brightness, Some(100));
    assert_eq!(look.color, Some([10, 20, 30]));
    assert!(look.white_temp.is_some());
    assert!(!look.resend);
    assert!(look.zones.is_empty());
}

/// Slot 0 on the white channel sends no command, so a blackout does not turn
/// the rig white.
#[test]
fn the_white_channel_at_zero_carries_no_value() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(&rig.fixtures()[2], &universe(63, &[255, 0, 0, 0, 0, 0]));
    assert_eq!(look.white_temp, None);
}

#[test]
fn the_control_channel_asks_for_a_resend_at_250() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let fixture = &rig.fixtures()[2];
    assert!(look(fixture, &universe(63, &[255, 0, 0, 0, 0, 250])).resend);
    assert!(!look(fixture, &universe(63, &[255, 0, 0, 0, 0, 9])).resend);
}

/// The second fixture starts at address 32, so its zone 0 is channels 33 to
/// 35 and it reads nothing of the first fixture's look.
#[test]
fn a_pixel_fixture_reads_the_zones_at_its_own_address() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let mut slots = universe(1, &[255]);
    slots[32] = 1;
    slots[33] = 2;
    slots[34] = 3;
    slots[31] = 255;
    let frame = UniverseFrame::new(0, source(), &slots);

    let first = Look::read(&rig.fixtures()[0], &frame);
    assert_eq!(first.zones.len(), 10);
    assert_eq!(first.zones[0], [0, 0, 0]);

    let second = Look::read(&rig.fixtures()[1], &frame);
    assert_eq!(second.zones.len(), 10);
    assert_eq!(second.zones[0], [1, 2, 3]);
    assert_eq!(second.color, None, "a pixel personality carries no color");
}

/// A desk that sends a short packet leaves the rest of the universe dark.
#[test]
fn a_channel_the_packet_stops_short_of_reads_zero() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(&rig.fixtures()[0], &[255, 10, 20]);
    assert!(look.on);
    assert_eq!(look.zones.len(), 10);
    assert_eq!(look.zones[0], [10, 20, 0]);
    assert_eq!(look.zones[9], [0, 0, 0]);
}
