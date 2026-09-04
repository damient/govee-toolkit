//! Mode selection, against a simulated device.
//!
//! These are the rules of `docs/modes.md` written as assertions: one enabled
//! mode never becomes another, a mode is chosen from state already known, and
//! a command a mode does not carry fails instead of being approximated.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use govee::{Args, Catalog, Config, DeviceId, Govee, Mode, State};
use govee_lan::{Endpoints, Transport};
use govee_sim::Simulator;

const MAC: &str = "AA:BB:CC:DD:EE:FF";
const SKU: &str = "H61A0";

struct Rig {
    govee: Govee,
    simulator: Simulator,
}

impl Rig {
    async fn start(yaml: &str) -> Self {
        let simulator = Simulator::start(govee_sim::Options::loopback(MAC, SKU))
            .await
            .expect("the simulator binds");

        let mut config: Config = serde_norway::from_str(yaml).expect("the configuration parses");
        config.lan.cache_disabled = true;
        config.lan.refresh_interval_seconds = None;
        config.lan.status_timeout_ms = 150;
        config.lan.scan_window_ms = 200;

        let transport = Transport::start(govee_lan::Options {
            endpoints: Endpoints {
                scan_target: simulator.scan_addr().expect("scan address"),
                reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                control_port: simulator.control_addr().expect("control address").port(),
                multicast_group: None,
            },
            ..config.lan.transport_options().expect("transport options")
        })
        .await
        .expect("the socket binds");

        let govee = Govee::attach(config, Catalog::embedded().expect("catalog"), transport)
            .expect("the configuration applies");
        govee.scan().await.expect("the scan goes out");
        Self { govee, simulator }
    }
}

fn id() -> DeviceId {
    DeviceId::new(MAC)
}

#[tokio::test]
async fn a_command_reports_the_mode_that_served_it() {
    let rig = Rig::start("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();

    let served = rig
        .govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(served.mode, Mode::Lan);
    assert_eq!(served.command, "power");
    assert_eq!(served.cmd, "turn");
}

#[tokio::test]
async fn an_out_of_range_argument_is_refused_rather_than_clamped() {
    let rig = Rig::start("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();

    // The firmware would clamp 0 up to 1 in silence (docs/protocol/lan.md 2.1)
    // and report success for a value it did not apply.
    let error = rig
        .govee
        .device(&id())
        .send("brightness", &Args::new().int("level", 0))
        .await
        .expect_err("0 is outside the declared range");

    assert_eq!(error.code(), "out_of_range");
    assert_eq!(rig.simulator.received_count(), 0, "nothing was sent");
}

#[tokio::test]
async fn a_single_mode_fails_rather_than_switching() {
    let rig =
        Rig::start("defaults:\n  modes: [lan]\nlan:\n  degrade_after: 1\n  cooldown_seconds: 60\n")
            .await;
    rig.simulator.set_silent(true);

    // One unanswered status is enough to degrade this configuration.
    let device = rig.govee.device(&id());
    let _ = device.status().await;
    assert_eq!(
        device.health(Mode::Lan).expect("known").state,
        State::Degraded
    );

    rig.simulator.clear();
    let started = Instant::now();
    let error = device
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("the only enabled mode is unavailable");

    // It fails and says so. It does not reach for another mode, and it does not
    // wait for a timeout to find out.
    assert_eq!(error.code(), "no_mode_available");
    assert!(started.elapsed() < Duration::from_millis(50));
    assert_eq!(rig.simulator.received_count(), 0);
}

#[tokio::test]
async fn a_second_mode_is_reported_rather_than_silently_skipped() {
    let rig = Rig::start(
        "defaults:\n  modes: [lan, ble]\nlan:\n  degrade_after: 1\n  cooldown_seconds: 60\n",
    )
    .await;
    rig.simulator.set_silent(true);

    let device = rig.govee.device(&id());
    let _ = device.status().await;
    assert_eq!(
        device.health(Mode::Lan).expect("known").state,
        State::Degraded
    );

    let error = device
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("ble has no transport yet");
    assert_eq!(error.code(), "mode_not_implemented");
    assert!(error.to_string().contains("ble"), "{error}");
}

#[tokio::test]
async fn a_device_that_was_never_discovered_is_not_scanned_for() {
    let rig = Rig::start("defaults:\n  modes: [lan]\n").await;
    rig.simulator.clear();

    let error = rig
        .govee
        .device(&DeviceId::new("11:22:33:44:55:66"))
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("nothing is known about it");
    assert_eq!(error.code(), "unknown_device");
    assert_eq!(rig.simulator.received_count(), 0);
}

#[tokio::test]
async fn the_status_a_device_reports_reaches_the_caller() {
    let rig = Rig::start("defaults:\n  modes: [lan]\n").await;
    rig.simulator.set_status(serde_json::json!({
        "onOff": 1, "brightness": 75, "color": { "r": 0, "g": 0, "b": 0 },
        "colorTemInKelvin": 7200
    }));

    let status = rig
        .govee
        .device(&id())
        .status()
        .await
        .expect("the device answers");
    assert_eq!(status.brightness, Some(75));
    assert!(status.is_white(), "a non-zero temperature means white mode");
}

#[tokio::test]
async fn enabling_a_mode_the_hardware_lacks_is_reported() {
    // H61A0 declares `ble: support: none`.
    let rig = Rig::start("defaults:\n  modes: [ble]\n").await;
    let problems = rig.govee.problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].message.contains("ble") && problems[0].message.contains("does not support"),
        "{}",
        problems[0]
    );
}

#[tokio::test]
async fn a_configuration_that_could_never_work_is_refused_at_startup() {
    let config: Config = serde_norway::from_str("defaults:\n  modes: []\n").expect("it parses");
    let error = Govee::start_with(config, Catalog::embedded().expect("catalog"))
        .await
        .expect_err("no enabled mode is not a configuration");
    assert_eq!(error.code(), "configuration");
}

#[tokio::test]
async fn the_devices_listing_carries_the_configured_view() {
    let rig = Rig::start(
        "defaults:\n  modes: [lan]\ndevices:\n  \"aa:bb:cc:dd:ee:ff\":\n    name: \"desk\"\n",
    )
    .await;

    let devices = rig.govee.devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, id());
    assert_eq!(devices[0].sku, SKU);
    assert_eq!(devices[0].name.as_deref(), Some("desk"));
    assert_eq!(devices[0].modes, [Mode::Lan]);
    assert_eq!(
        devices[0].lan_health.expect("health is tracked").state,
        State::Ok
    );
}
