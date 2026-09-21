//! What a rig does when the sender goes quiet, and when one device drops.
//!
//! One fixture is a simulated device on the loopback, and the other is a
//! device that never answered. The second one fails every write, which is what
//! a fixture that dropped off the network does.
//!
//! `wire.rs` drives the same rig from the socket and reads the bytes back.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;
use govee_toolkit_dmx::apply::{Applier, Look};

use self::common::{
    GONE, REACHED, SILENCE, govee, lit, rig, simulator, timing, wait_for, wait_for_lit,
};

/// The device that never answered reports its failure, and the one that did
/// takes the whole look.
#[tokio::test]
async fn a_device_that_fails_stops_no_other_fixture() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, mut failures) = Applier::start(&govee, &rig, timing());
    simulator.clear();
    applier.push(&DeviceId::new(REACHED), lit());
    applier.push(&DeviceId::new(GONE), lit());

    let failure = failures.recv().await.expect("the failure of the gone one");
    assert_eq!(failure.id, DeviceId::new(GONE));
    // Power, brightness and color: the whole look reached the device that
    // answered, while the other one was failing.
    assert!(
        wait_for(|| (simulator.received_count() >= 3).then_some(()))
            .await
            .is_some(),
        "the reached fixture took {} datagrams",
        simulator.received_count()
    );
    applier.close().await;
}

/// `hold` keeps the last look, so a sender that goes away puts nothing more
/// on the wire.
#[tokio::test]
async fn a_silent_sender_holds_the_last_look() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    simulator.clear();
    applier.push(&DeviceId::new(REACHED), lit());
    wait_for_lit(&simulator).await;

    simulator.clear();
    tokio::time::sleep(SILENCE * 3).await;
    assert_eq!(simulator.received_count(), 0, "`hold` writes nothing");
    applier.close().await;
}

/// `off` powers the device off after the timeout, and the write goes out
/// without the desk asking for it.
#[tokio::test]
async fn a_silent_sender_powers_an_off_fixture_down() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "off");

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    simulator.clear();
    applier.push(&DeviceId::new(REACHED), lit());
    wait_for_lit(&simulator).await;

    simulator.clear();
    assert!(
        wait_for(|| (simulator.received_count() >= 1).then_some(()))
            .await
            .is_some(),
        "`off` powers the device down"
    );
    applier.close().await;
}

/// A white command replaces the color on the device, so the white channel
/// back at 0 sends the color again. Nothing else takes the device out of
/// white.
#[tokio::test]
async fn the_white_channel_back_at_zero_paints_the_color_again() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    let id = DeviceId::new(REACHED);
    simulator.clear();
    applier.push(&id, lit());
    wait_for_lit(&simulator).await;

    simulator.clear();
    applier.push(
        &id,
        Look {
            white_temp: Some(4000),
            ..lit()
        },
    );
    assert!(
        wait_for(|| (simulator.received_count() >= 1).then_some(()))
            .await
            .is_some(),
        "the white temperature reached the device"
    );

    simulator.clear();
    applier.push(&id, lit());
    assert!(
        wait_for(|| (simulator.received_count() >= 1).then_some(()))
            .await
            .is_some(),
        "the color goes out again, so the fixture leaves white"
    );
    applier.close().await;
}

/// The white temperature one `colorwc` carries. A color write carries the
/// same command, with the temperature at 0.
fn kelvin(received: &govee_toolkit_sim::Received) -> Option<i64> {
    received.data.get("colorTemInKelvin")?.as_i64()
}

/// The look the desk holds on the white channel, with every color channel at
/// 0.
fn white() -> Look {
    Look {
        color: Some([0, 0, 0]),
        white_temp: Some(4000),
        ..lit()
    }
}

/// A pass that carries both writes the white one alone: the color would show
/// for the few milliseconds before it, which reads as a blink.
#[tokio::test]
async fn a_pass_that_carries_a_white_temperature_writes_no_color() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    let id = DeviceId::new(REACHED);
    applier.push(&id, white());
    assert!(
        wait_for(|| simulator
            .received()
            .iter()
            .any(|received| kelvin(received) == Some(4000))
            .then_some(()))
        .await
        .is_some(),
        "the white temperature reached the device"
    );
    let colored: Vec<String> = simulator
        .received()
        .into_iter()
        .filter(|received| kelvin(received) == Some(0))
        .map(|received| received.data.to_string())
        .collect();
    assert!(
        colored.is_empty(),
        "the pass wrote a color too: {colored:?}"
    );
    applier.close().await;
}
