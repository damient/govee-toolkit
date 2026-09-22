//! The white channel of a zone personality.
//!
//! A white command ends the armed segment channel, so the white goes out over
//! the armed channel and the disarm follows it. The white channel back at 0
//! arms the channel again — see `docs/dmx.md`, "White temperature".

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;
use govee_toolkit_dmx::apply::{Applier, Look};
use govee_toolkit_sim::Simulator;

use self::common::{REACHED, govee, segment_rig, simulator, timing, wait_for};

/// `bb 00 01 b1 01 0a` and `bb 00 01 b1 00 0b`, in base64.
const ARM: &str = "uwABsQEK";
const DISARM: &str = "uwABsQAL";

/// What the device took, in order: the command name, with the segment channel
/// frames told apart.
fn taken(simulator: &Simulator) -> Vec<String> {
    simulator
        .received()
        .into_iter()
        .filter(|received| !received.is_status_request())
        .map(|received| {
            if received.cmd != "razer" {
                return received.cmd;
            }
            match received.data.get("pt").and_then(|pt| pt.as_str()) {
                Some(ARM) => "arm".to_owned(),
                Some(DISARM) => "disarm".to_owned(),
                _ => "paint".to_owned(),
            }
        })
        .collect()
}

async fn wait_for_taken(simulator: &Simulator, last: &str) -> Vec<String> {
    let took = wait_for(|| {
        let taken = taken(simulator);
        (taken.last().map(String::as_str) == Some(last)).then_some(taken)
    })
    .await;
    took.unwrap_or_else(|| panic!("no `{last}` reached the device: {:?}", taken(simulator)))
}

fn zones(white_temp: Option<i64>) -> Look {
    Look {
        on: true,
        brightness: Some(80),
        white_temp,
        zones: vec![[10, 20, 30]; 15],
        ..Look::default()
    }
}

#[tokio::test]
async fn the_white_goes_out_over_the_armed_channel_and_the_disarm_follows() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = segment_rig(&catalog);

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    let id = DeviceId::new(REACHED);
    simulator.clear();
    applier.push(&id, zones(None));
    let lit = wait_for_taken(&simulator, "paint").await;
    assert_eq!(lit, ["turn", "brightness", "arm", "paint"]);

    simulator.clear();
    applier.push(&id, zones(Some(4000)));
    let white = wait_for_taken(&simulator, "disarm").await;
    assert_eq!(white, ["colorwc", "disarm"]);

    simulator.clear();
    applier.push(&id, zones(None));
    let back = wait_for_taken(&simulator, "paint").await;
    assert_eq!(back, ["arm", "paint"], "no power command, no disarm");
    applier.close().await;
}

/// A white held on the desk sends nothing more, and a white that moves sends
/// the white alone: the channel is already disarmed.
#[tokio::test]
async fn a_white_that_moves_sends_the_white_alone() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = segment_rig(&catalog);

    let (applier, _failures) = Applier::start(&govee, &rig, timing());
    let id = DeviceId::new(REACHED);
    applier.push(&id, zones(None));
    wait_for_taken(&simulator, "paint").await;
    applier.push(&id, zones(Some(4000)));
    wait_for_taken(&simulator, "disarm").await;

    simulator.clear();
    applier.push(&id, zones(Some(4000)));
    applier.push(&id, zones(Some(5000)));
    let moved = wait_for_taken(&simulator, "colorwc").await;
    assert_eq!(moved, ["colorwc"]);
    applier.close().await;
}
