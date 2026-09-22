//! What one frame asks of one fixture, over the fixture rig of the patch
//! tests.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::net::SocketAddr;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;

use super::Look;
use crate::input::UniverseFrame;
use crate::patch::{Fixture, Patch, Rig, SignalLoss};

/// The rig the patch tests load: two `segment` fixtures at 1 and 48, and one
/// `full` fixture at 95.
const RIG: &str = include_str!("../../tests/fixtures/patch.yaml");

fn source() -> SocketAddr {
    "192.0.2.2:6454".parse().expect("a socket address")
}

fn rig(catalog: &Catalog) -> Rig {
    let patch = Patch::parse(RIG, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let segment = catalog.device("H61A0").expect("the SKU resolves");
    let full = catalog.device("H6008").expect("the SKU resolves");
    patch
        .resolve(
            |id| {
                if id == &DeviceId::new("AA:BB:CC:DD:EE:03") {
                    Some(full)
                } else {
                    Some(segment)
                }
            },
            |sku| catalog.device(sku).ok(),
        )
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
        &universe(95, &[0, 0, 255, 255, 255, 255]),
    );
    assert!(!look.on);
    assert_eq!(look.brightness, None);
}

#[test]
fn a_full_fixture_reads_its_six_channels() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let fixture = &rig.fixtures()[2];
    let look = look(fixture, &universe(95, &[255, 0, 10, 20, 30, 255]));
    assert!(look.on);
    assert_eq!(look.brightness, Some(100));
    assert_eq!(look.color, Some([10, 20, 30]));
    assert!(look.white_temp.is_some());
    assert!(!look.resend);
    assert!(look.zones.is_empty());
}

/// A blackout does not turn the rig white.
#[test]
fn the_white_channel_at_zero_carries_no_value() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(&rig.fixtures()[2], &universe(95, &[255, 0, 0, 0, 0, 0]));
    assert_eq!(look.white_temp, None);
}

#[test]
fn the_mode_channel_asks_for_a_resend_at_250() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let fixture = &rig.fixtures()[2];
    assert!(look(fixture, &universe(95, &[255, 250, 0, 0, 0, 0])).resend);
    assert!(!look(fixture, &universe(95, &[255, 9, 0, 0, 0, 0])).resend);
}

#[test]
fn a_segment_fixture_reads_the_zones_at_its_own_address() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let mut slots = universe(1, &[255]);
    slots[47] = 255;
    slots[49] = 1;
    slots[50] = 2;
    slots[51] = 3;
    let frame = UniverseFrame::new(0, source(), &slots);

    let first = Look::read(&rig.fixtures()[0], &frame);
    assert_eq!(first.zones.len(), 15);
    assert_eq!(first.zones[0], [0, 0, 0]);

    let second = Look::read(&rig.fixtures()[1], &frame);
    assert_eq!(second.zones.len(), 15);
    assert_eq!(second.zones[0], [1, 2, 3]);
    assert_eq!(second.color, None, "a segment personality carries no color");
}

/// A desk that sends a short packet leaves the rest of the universe dark.
#[test]
fn a_channel_the_packet_stops_short_of_reads_zero() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(&rig.fixtures()[0], &[255, 0, 10, 20]);
    assert!(look.on);
    assert_eq!(look.zones.len(), 15);
    assert_eq!(look.zones[0], [10, 20, 0]);
    assert_eq!(look.zones[14], [0, 0, 0]);
}

#[test]
fn a_held_fixture_takes_no_look_after_the_signal_goes() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(
        &rig.fixtures()[2],
        &universe(95, &[255, 0, 10, 20, 30, 255]),
    );
    assert_eq!(look.quiet(SignalLoss::Hold), None);
}

#[test]
fn a_blacked_fixture_keeps_its_brightness_and_loses_its_color() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(
        &rig.fixtures()[2],
        &universe(95, &[255, 0, 10, 20, 30, 255]),
    );
    let quiet = look
        .quiet(SignalLoss::Black)
        .expect("`black` asks for a look");
    assert!(quiet.on);
    assert_eq!(quiet.brightness, Some(100));
    assert_eq!(quiet.color, Some([0, 0, 0]));
    assert_eq!(quiet.white_temp, None);
}

#[test]
fn a_blacked_segment_fixture_takes_every_zone_to_zero() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(&rig.fixtures()[0], &universe(1, &[255, 0, 10, 20, 30]));
    let quiet = look
        .quiet(SignalLoss::Black)
        .expect("`black` asks for a look");
    assert_eq!(quiet.zones.len(), 15);
    assert!(quiet.zones.iter().all(|zone| *zone == [0, 0, 0]));
    assert_eq!(quiet.color, None, "a segment personality carries no color");
}

#[test]
fn an_off_fixture_powers_down_after_the_signal_goes() {
    let catalog = catalog();
    let rig = rig(&catalog);
    let look = look(
        &rig.fixtures()[2],
        &universe(95, &[255, 0, 10, 20, 30, 255]),
    );
    let quiet = look.quiet(SignalLoss::Off).expect("`off` asks for a look");
    assert!(!quiet.on);
    assert_eq!(quiet.brightness, None);
}
