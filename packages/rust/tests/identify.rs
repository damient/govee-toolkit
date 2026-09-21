//! The identify walk, against a simulated device.
//!
//! The walk is what a person reads a rig with, so the order the device is
//! sent matters: the rig goes off, one device lights, the rig goes off again.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Mutex;
use std::time::Duration;

use govee_toolkit::{Catalog, Config, DeviceId, Identify, Mode, Walk, WalkObserver};

mod common;

use common::{Rig, SKU, id, wait_for};

async fn rig(yaml: &str) -> Rig {
    let config: Config = serde_norway::from_str(yaml).expect("the configuration parses");
    Rig::start(config, Catalog::embedded().expect("catalog"), SKU).await
}

/// A walk short enough that the suite does not wait on it.
fn walk() -> Walk {
    Walk {
        pass: Identify::default(),
        wait: Duration::from_millis(1),
        hold: Duration::from_millis(1),
        keep: false,
        mode: Mode::Lan,
    }
}

/// What the walk announced, in order.
#[derive(Default)]
struct Seen {
    lit: Mutex<Vec<String>>,
    refused: Mutex<Vec<String>>,
}

impl WalkObserver for Seen {
    fn lighting(&self, id: &DeviceId) {
        if let Ok(mut lit) = self.lit.lock() {
            lit.push(id.to_string());
        }
    }

    fn refused(&self, id: &DeviceId, _reason: &str) {
        if let Ok(mut refused) = self.refused.lock() {
            refused.push(id.to_string());
        }
    }
}

/// The `cmd` of every write the device was sent, once `count` of them
/// landed. A write is one datagram that nothing acknowledges, so the walk
/// returns before the last one is recorded.
async fn writes(rig: &Rig, count: usize) -> Vec<String> {
    wait_for(|| {
        let sent: Vec<String> = rig
            .simulator
            .received()
            .into_iter()
            .filter(|packet| !packet.is_status_request())
            .map(|packet| packet.cmd)
            .collect();
        (sent.len() >= count).then_some(sent)
    })
    .await
    .unwrap_or_default()
}

#[tokio::test]
async fn a_walk_takes_the_rig_off_lights_it_then_takes_it_off_again() {
    let rig = rig("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();
    let seen = Seen::default();
    let targets = [id()];

    let report = rig
        .govee
        .identify_walk(&targets, &targets, &walk(), &seen)
        .await
        .expect("the configuration enables lan");

    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(report.stayed.is_empty(), "{:?}", report.stayed);
    assert_eq!(report.summary("device"), None);
    assert_eq!(
        seen.lit.lock().expect("no panic").as_slice(),
        [id().to_string()]
    );
    // The pass is power, brightness and color, between two blackouts.
    let sent = writes(&rig, 5).await;
    assert_eq!(sent.first().map(String::as_str), Some("turn"), "{sent:?}");
    assert_eq!(sent.last().map(String::as_str), Some("turn"), "{sent:?}");
    assert_eq!(sent.len(), 5, "{sent:?}");
}

/// `keep` leaves the room lit: the operator reads the last fixture for as
/// long as they need it.
#[tokio::test]
async fn a_walk_that_keeps_the_look_sends_no_closing_blackout() {
    let rig = rig("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();
    let targets = [id()];

    let report = rig
        .govee
        .identify_walk(
            &targets,
            &targets,
            &Walk {
                keep: true,
                ..walk()
            },
            &(),
        )
        .await
        .expect("the configuration enables lan");

    assert!(report.stayed.is_empty());
    let sent = writes(&rig, 4).await;
    assert_eq!(sent.len(), 4, "{sent:?}");
}

/// A caller that narrows the walk still darkens the rig it named.
#[tokio::test]
async fn a_device_in_the_blackout_alone_goes_off_and_never_lights() {
    let rig = rig("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();
    let seen = Seen::default();

    rig.govee
        .identify_walk(&[id()], &[], &walk(), &seen)
        .await
        .expect("the configuration enables lan");

    assert!(seen.lit.lock().expect("no panic").is_empty());
    assert_eq!(writes(&rig, 2).await, ["turn", "turn"]);
}

/// The walk drives what the user enabled and nothing else, and says so
/// before it sends anything.
#[tokio::test]
async fn a_walk_over_a_mode_the_configuration_does_not_enable_is_refused() {
    let rig = rig("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();
    let targets = [id()];

    let error = rig
        .govee
        .identify_walk(
            &targets,
            &targets,
            &Walk {
                mode: Mode::Cloud,
                ..walk()
            },
            &(),
        )
        .await
        .expect_err("the configuration enables lan alone");

    assert_eq!(error.code(), "mode_not_enabled");
    assert_eq!(rig.simulator.received_count(), 0);
}

/// A device that never went dark is dropped from the walk: it takes no pass,
/// and the closing blackout does not ask it again.
#[tokio::test]
async fn a_device_that_refuses_the_blackout_is_reported_once_and_never_lights() {
    let rig = rig("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();
    let seen = Seen::default();
    // Nothing ever answered for this one, so the blackout cannot reach it.
    let absent = DeviceId::new("11:22:33:44:55:66");
    let targets = [id(), absent.clone()];

    let report = rig
        .govee
        .identify_walk(&targets, &targets, &walk(), &seen)
        .await
        .expect("the configuration enables lan");

    assert_eq!(report.failed, std::slice::from_ref(&absent));
    assert!(report.stayed.is_empty(), "{:?}", report.stayed);
    assert_eq!(
        seen.lit.lock().expect("no panic").as_slice(),
        [id().to_string()]
    );
    assert_eq!(
        seen.refused.lock().expect("no panic").as_slice(),
        [absent.to_string()]
    );
    assert_eq!(
        report.summary("device").as_deref(),
        Some(format!("these devices did not take the pass: {absent}").as_str())
    );
}

/// Both faults reach one line: the room shows a device that took no pass and
/// a device that holds the color.
#[test]
fn a_summary_names_the_two_faults() {
    let report = govee_toolkit::WalkReport {
        failed: vec![DeviceId::new("AA:BB:CC:DD:EE:01")],
        stayed: vec![DeviceId::new("AA:BB:CC:DD:EE:02")],
    };
    assert_eq!(
        report.summary("fixture").as_deref(),
        Some(
            "these fixtures did not take the pass: AA:BB:CC:DD:EE:01; \
             these fixtures did not go off at the end: AA:BB:CC:DD:EE:02"
        )
    );
}
