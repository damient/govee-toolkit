//! Groups, against a simulated device.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use govee_toolkit::{Catalog, Config, DeviceId, Mode};

mod common;

use common::{MAC, Rig, SKU, id, wait_for};

/// Sorts before the simulated device, so the absent member comes first.
const ABSENT: &str = "11:22:33:44:55:66";

async fn rig() -> Rig {
    let yaml = format!(
        "devices:\n  \"{MAC}\":\n    name: desk\n    groups: [ambient]\n  \"{ABSENT}\":\n    groups: [Ambient]\n"
    );
    let config: Config = serde_norway::from_str(&yaml).expect("the configuration parses");
    Rig::start(config, Catalog::embedded().expect("catalog"), SKU).await
}

#[tokio::test]
async fn a_group_name_resolves_to_its_members_from_the_configuration() {
    let rig = rig().await;
    let members = rig.govee.targets("ambient").expect("a group");
    assert_eq!(members, [DeviceId::new(ABSENT), id()]);
    assert_eq!(rig.govee.targets("desk").expect("a name"), [id()]);
    assert_eq!(
        rig.govee
            .target("ambient")
            .expect_err("a group is not one device")
            .code(),
        "target_not_understood"
    );
}

#[tokio::test]
async fn a_member_that_fails_stops_no_other_one() {
    let rig = rig().await;
    rig.simulator.clear();
    let members = rig.govee.targets("group:ambient").expect("a group");

    let outcomes = rig.govee.group(&members).power(true).await;

    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].id, DeviceId::new(ABSENT));
    let absent = outcomes[0]
        .result
        .as_ref()
        .expect_err("nothing answers there");
    assert_eq!(absent.code(), "unknown_device");
    let served = outcomes[1]
        .result
        .as_ref()
        .expect("the simulated device is served");
    assert_eq!(served.mode, Mode::Lan);
    let received = wait_for(|| (rig.simulator.received_count() > 0).then_some(())).await;
    assert!(received.is_some(), "the simulated device got the command");
}

#[tokio::test]
async fn a_pinned_mode_that_a_member_does_not_enable_fails_that_member() {
    let rig = rig().await;
    let outcomes = rig.govee.group_on(&[id()], Mode::Ble).power(true).await;
    let refused = outcomes[0].result.as_ref().expect_err("ble is not enabled");
    assert_eq!(refused.code(), "mode_not_enabled");
}

#[tokio::test]
async fn ensure_known_reports_each_member() {
    let rig = rig().await;
    let members = rig.govee.targets("ambient").expect("a group");
    let known = rig.govee.group(&members).ensure_known().await;
    assert!(known[0].result.is_err());
    assert_eq!(known[1].result.as_ref().ok(), Some(&Mode::Lan));
}

#[tokio::test]
async fn a_pinned_ensure_known_fails_a_member_that_does_not_enable_the_mode() {
    let rig = rig().await;
    let known = rig.govee.group_on(&[id()], Mode::Ble).ensure_known().await;
    let refused = known[0].result.as_ref().expect_err("ble is not enabled");
    assert_eq!(refused.code(), "mode_not_enabled");
}

/// One after the other, five absent members would cost five scan windows.
#[tokio::test]
async fn absent_members_cost_one_scan_window_between_them() {
    let rig = rig().await;
    let absent: Vec<DeviceId> = (0..5)
        .map(|n| DeviceId::new(format!("11:22:33:44:55:7{n}")))
        .collect();
    let started = std::time::Instant::now();
    let known = rig.govee.group(&absent).ensure_known().await;
    assert!(known.iter().all(|outcome| outcome.result.is_err()));
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(600),
        "{elapsed:?}"
    );
}
