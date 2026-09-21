//! What a dimmer that reaches 0 does.
//!
//! The device goes black and stays on, and the power off waits for the dimmer
//! to stay at 0 — see `docs/dmx.md`, "The dimmer at 0".

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;
use govee_toolkit_dmx::apply::{Applier, Look};

use self::common::{REACHED, cmds, govee, lit, rig, simulator, timing, wait_for, wait_for_lit};

/// A look with the dimmer at 0. The color channels keep their values, the
/// way a desk that pulls one fader down leaves them.
fn dimmed_out() -> Look {
    Look {
        on: false,
        brightness: None,
        ..lit()
    }
}

/// A dimmer that dips through 0 and comes back must cost no power command:
/// the device stays on and goes black, so it lights again on one repaint.
#[tokio::test]
async fn a_dimmer_that_dips_through_zero_powers_nothing_off() {
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
    applier.push(&id, dimmed_out());
    assert!(
        wait_for(|| cmds(&simulator)
            .contains(&"colorwc".to_owned())
            .then_some(()))
        .await
        .is_some(),
        "the fixture went black: {:?}",
        cmds(&simulator)
    );
    applier.push(&id, lit());
    assert!(
        wait_for(|| (cmds(&simulator).len() >= 2).then_some(()))
            .await
            .is_some(),
        "the fixture lit again: {:?}",
        cmds(&simulator)
    );

    let taken = cmds(&simulator);
    assert!(
        !taken.contains(&"turn".to_owned()),
        "the dip powered the device: {taken:?}"
    );
    applier.close().await;
}

/// Black alone leaves a device drawing power, and glowing on some units.
#[tokio::test]
async fn a_dimmer_held_at_zero_powers_the_fixture_off() {
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
    applier.push(&id, dimmed_out());
    assert!(
        wait_for(|| cmds(&simulator).contains(&"turn".to_owned()).then_some(()))
            .await
            .is_some(),
        "the off delay powered the fixture off: {:?}",
        cmds(&simulator)
    );
    // Black first, then the power off.
    let taken = cmds(&simulator);
    assert_eq!(
        taken.first().map(String::as_str),
        Some("colorwc"),
        "{taken:?}"
    );
    applier.close().await;
}
