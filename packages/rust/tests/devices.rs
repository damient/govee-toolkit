//! `Govee::device` and `Govee::devices`, against a simulated device.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use govee_toolkit::{Catalog, Config, DeviceId, Devices, Filter, Mode, Verb};

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

async fn select<'a>(rig: &'a Rig, targets: &[&str], mode: Option<Mode>) -> Devices<'a> {
    rig.govee
        .devices(Filter::targets(targets.iter().copied()), mode)
        .await
        .expect("the targets resolve")
}

fn ids(devices: &Devices<'_>) -> Vec<DeviceId> {
    devices.iter().map(|handle| handle.id().clone()).collect()
}

#[tokio::test]
async fn a_group_name_resolves_to_its_members_from_the_configuration() {
    let rig = rig().await;
    assert_eq!(
        ids(&select(&rig, &["ambient"], None).await),
        [DeviceId::new(ABSENT), id()]
    );
    assert_eq!(ids(&select(&rig, &["desk"], None).await), [id()]);
    assert_eq!(
        rig.govee
            .device("ambient", None)
            .expect_err("a group is not one device")
            .code(),
        "target_not_understood"
    );
    assert_eq!(rig.govee.device("desk", None).expect("a name").id(), &id());
}

#[tokio::test]
async fn a_sku_resolves_against_what_the_first_scan_found() {
    let rig = rig().await;
    let devices = select(&rig, &[SKU], None).await;
    assert_eq!(ids(&devices), [id()]);
    assert_eq!(devices.list()[0].sku, SKU);
    assert_eq!(
        rig.govee
            .device(SKU, None)
            .expect_err("a SKU is not one device")
            .code(),
        "target_not_understood"
    );
}

#[tokio::test]
async fn every_device_is_what_a_scan_finds() {
    let rig = rig().await;
    let all = rig
        .govee
        .devices(Filter::all(), None)
        .await
        .expect("the scan runs");
    assert_eq!(ids(&all), [id()]);
    let none = rig
        .govee
        .devices(Filter::targets(Vec::<String>::new()), None)
        .await
        .expect("no target resolves");
    assert!(none.is_empty());
}

#[tokio::test]
async fn enables_keeps_the_devices_of_that_mode_and_refuses_an_empty_match() {
    let rig = rig().await;
    let refused = rig
        .govee
        .devices(Filter::targets(["desk"]).enables(Mode::Ble), None)
        .await
        .expect_err("desk does not enable ble");
    assert_eq!(refused.code(), "no_such_target");
    let kept = rig
        .govee
        .devices(Filter::targets(["desk"]).enables(Mode::Lan), None)
        .await
        .expect("desk enables lan");
    assert_eq!(ids(&kept), [id()]);
}

#[tokio::test]
async fn a_device_two_targets_name_appears_once() {
    let rig = rig().await;
    let devices = select(&rig, &["desk", "ambient", "desk"], None).await;
    assert_eq!(ids(&devices), [id(), DeviceId::new(ABSENT)]);
}

#[tokio::test]
async fn a_member_that_fails_stops_no_other_one() {
    let rig = rig().await;
    rig.simulator.clear();
    let outcomes = select(&rig, &["group:ambient"], None)
        .await
        .power(true)
        .await;

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
    let outcomes = select(&rig, &[MAC], Some(Mode::Ble))
        .await
        .power(true)
        .await;
    let refused = outcomes[0].result.as_ref().expect_err("ble is not enabled");
    assert_eq!(refused.code(), "mode_not_enabled");
}

#[tokio::test]
async fn ensure_known_reports_each_member() {
    let rig = rig().await;
    let known = select(&rig, &["ambient"], None).await.ensure_known().await;
    assert!(known[0].result.is_err());
    assert_eq!(known[1].result.as_ref().ok(), Some(&Mode::Lan));
}

#[tokio::test]
async fn a_pinned_ensure_known_fails_a_member_that_does_not_enable_the_mode() {
    let rig = rig().await;
    let known = select(&rig, &[MAC], Some(Mode::Ble))
        .await
        .ensure_known()
        .await;
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
    let known = rig
        .govee
        .devices(Filter::ids(absent), None)
        .await
        .expect("identities resolve")
        .ensure_known()
        .await;
    assert!(known.iter().all(|outcome| outcome.result.is_err()));
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(600),
        "{elapsed:?}"
    );
}

#[tokio::test]
async fn apply_sends_power_on_first_and_skips_a_member_the_scan_misses() {
    let rig = rig().await;
    let applied = select(&rig, &["ambient"], None)
        .await
        .apply(vec![Verb::Brightness(50), Verb::Power(true)])
        .await;

    assert!(applied.reached[0].result.is_err());
    let names: Vec<&str> = applied.steps.iter().map(|step| step.name).collect();
    assert_eq!(names, ["power", "brightness"]);
    for step in &applied.steps {
        assert_eq!(step.outcomes.len(), 1, "{}", step.name);
        assert_eq!(step.outcomes[0].id, id());
        assert!(step.outcomes[0].result.is_ok(), "{}", step.name);
    }
    assert_eq!(applied.failures().count(), 1);
    assert!(!applied.is_clean());
}

#[tokio::test]
async fn apply_sends_power_off_last() {
    let rig = rig().await;
    let applied = select(&rig, &[MAC], None)
        .await
        .apply(vec![Verb::Power(false), Verb::Color([255, 0, 0])])
        .await;
    let names: Vec<&str> = applied.steps.iter().map(|step| step.name).collect();
    assert_eq!(names, ["color", "power"]);
    assert!(applied.is_clean());
}

#[tokio::test]
async fn apply_sends_no_step_when_the_scan_reaches_no_member() {
    let rig = rig().await;
    let applied = select(&rig, &[ABSENT], None)
        .await
        .apply(vec![Verb::Power(true)])
        .await;
    assert!(applied.steps.is_empty());
    assert_eq!(applied.failures().count(), 1);
}
