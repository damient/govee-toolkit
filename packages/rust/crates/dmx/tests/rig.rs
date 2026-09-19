//! What a rig does when the sender goes quiet, and when one device drops.
//!
//! One fixture is a simulated device on the loopback, and the other is a
//! device that never answered. The second one fails every write, which is what
//! a fixture that dropped off the network does.
//!
//! The simulator plays the wire and interprets nothing, so every assertion
//! here counts the datagrams it recorded rather than reading a command out of
//! them. The end-to-end test that reads them lands with the release — see
//! `docs/dmx-plan.md`.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use govee_toolkit::codec::Catalog;
use govee_toolkit::lan::{Endpoints, Transport};
use govee_toolkit::{Config, DeviceId, Govee};
use govee_toolkit_dmx::apply::{Applier, Look, Timing};
use govee_toolkit_dmx::patch::{Patch, Rig};
use govee_toolkit_sim::{Options, Simulator};

/// The device the simulator answers for.
const REACHED: &str = "AA:BB:CC:DD:EE:01";
/// A device that never answered a scan. Every write to it fails.
const GONE: &str = "AA:BB:CC:DD:EE:02";
const SKU: &str = "H61A0";

/// Short enough that a test does not wait on it, long enough that the
/// loopback round trip lands inside it.
const SILENCE: Duration = Duration::from_millis(300);
/// Far past what any test here waits, so a refresh drives no assertion.
const REFRESH: Duration = Duration::from_secs(60);

/// Two fixtures on one universe: the simulated one, then the one that is
/// gone. `loss` is what both show once the sender goes quiet.
fn patch(loss: &str) -> String {
    let entry = |device: &str, address: u16| {
        format!(
            "  - device: \"{device}\"\n    universe: 0\n    address: {address}\n    personality: full\n    on_signal_loss: {loss}\n"
        )
    };
    format!("patch:\n{}{}", entry(REACHED, 1), entry(GONE, 7))
}

fn rig(catalog: &Catalog, loss: &str) -> Rig {
    let patch = Patch::parse(&patch(loss), "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let device = catalog.device(SKU).expect("the SKU resolves");
    patch
        .resolve(|_| Some(device), |_| Some(device))
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

/// The SDK, attached to one simulated device and already scanned.
async fn govee(simulator: &Simulator) -> Govee {
    let mut config = Config::default();
    config.lan.cache_disabled = true;
    config.lan.refresh_interval_seconds = None;
    config.lan.status_timeout_ms = 150;
    config.lan.scan_window_ms = 200;
    let endpoints = Endpoints {
        scan_target: simulator.scan_addr().expect("the scan address"),
        reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        control_port: simulator
            .control_addr()
            .expect("the control address")
            .port(),
        multicast_group: None,
    };
    let options = govee_toolkit::lan::Options {
        endpoints,
        ..config.lan.transport_options().expect("transport options")
    };
    let transport = Transport::start(options).await.expect("the socket binds");
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let govee = Govee::attach(config, catalog, [Arc::new(transport) as Arc<_>])
        .expect("the configuration applies");
    // One scan is one datagram that nothing retries. Ask again rather than
    // widen the window: the rig is ready as soon as one reply lands.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let found = govee.scan().await.expect("the scan goes out");
        if found
            .iter()
            .any(|device| device.id == DeviceId::new(REACHED))
        {
            return govee;
        }
        assert!(
            Instant::now() < deadline,
            "the simulator answered no scan in 10s"
        );
    }
}

/// A look that powers the fixture on and paints it.
fn lit() -> Look {
    Look {
        on: true,
        brightness: Some(80),
        color: Some([10, 20, 30]),
        ..Look::default()
    }
}

/// Wait until the simulator has taken the whole of [`lit`]: the power, the
/// brightness and the color. Counting datagrams instead would clear the
/// recorder while the look is still on its way, because the commands of one
/// look do not go out back to back.
async fn wait_for_lit(simulator: &Simulator) {
    let took = wait_for(|| {
        let cmds: Vec<String> = simulator
            .received()
            .into_iter()
            .map(|received| received.cmd)
            .collect();
        ["turn", "brightness", "colorwc"]
            .iter()
            .all(|cmd| cmds.iter().any(|taken| taken == cmd))
            .then_some(())
    })
    .await;
    assert!(took.is_some(), "the whole look reached the device");
}

/// Poll `check` until it yields, for at most a second.
async fn wait_for<T>(mut check: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if let Some(value) = check() {
            return Some(value);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    check()
}

/// A show does not stop because one fixture dropped: the device that never
/// answered reports its failure, and the one that did takes the whole look.
#[tokio::test]
async fn a_device_that_fails_stops_no_other_fixture() {
    let simulator = Simulator::start(Options::loopback(REACHED, SKU))
        .await
        .expect("the simulator binds");
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, mut failures) = Applier::start(
        &govee,
        &rig,
        Timing {
            refresh: REFRESH,
            silence: SILENCE,
        },
    );
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
    let simulator = Simulator::start(Options::loopback(REACHED, SKU))
        .await
        .expect("the simulator binds");
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "hold");

    let (applier, _failures) = Applier::start(
        &govee,
        &rig,
        Timing {
            refresh: REFRESH,
            silence: SILENCE,
        },
    );
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
    let simulator = Simulator::start(Options::loopback(REACHED, SKU))
        .await
        .expect("the simulator binds");
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let rig = rig(&catalog, "off");

    let (applier, _failures) = Applier::start(
        &govee,
        &rig,
        Timing {
            refresh: REFRESH,
            silence: SILENCE,
        },
    );
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
