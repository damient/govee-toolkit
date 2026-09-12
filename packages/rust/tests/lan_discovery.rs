//! What a `lan` scan finds, against a simulated device. On the loopback with
//! ephemeral ports: no hardware, no multicast and no privileges.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::time::{Duration, Instant};

use govee_toolkit::lan::{DeviceId, Options, State, Transport};
use govee_toolkit_sim::Simulator;

mod common;

use common::{SKU, id};

struct Rig {
    transport: Transport,
    _simulator: Simulator,
}

impl Rig {
    async fn start(window: Duration) -> Self {
        let simulator = common::simulator(SKU).await;
        let transport = Transport::start(Options {
            endpoints: common::endpoints(&simulator),
            // Every test drives its own scan: a background one would race the
            // assertions.
            refresh_interval: None,
            scan_window: window,
            ..Options::default()
        })
        .await
        .expect("the socket binds");
        Self {
            transport,
            _simulator: simulator,
        }
    }
}

#[tokio::test]
async fn a_transport_reports_the_window_its_own_mode_needs() {
    let rig = Rig::start(Duration::from_millis(200)).await;
    assert_eq!(rig.transport.scan_window(), Duration::from_millis(200));
}

#[tokio::test]
async fn a_scan_finds_the_device_and_records_where_it_is() {
    let rig = Rig::start(Duration::from_millis(200)).await;

    let found = rig
        .transport
        .scan(Duration::from_millis(300))
        .await
        .expect("the scan goes out");

    assert_eq!(found.len(), 1, "the simulated device answers");
    let known = rig.transport.devices();
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].id, id());
    assert_eq!(known[0].sku, SKU);
    assert_eq!(known[0].health.state, State::Ok);
    assert_eq!(rig.transport.sku(&id()).as_deref(), Some(SKU));
}

/// The window is a ceiling and not a cost: the reply ends the wait.
#[tokio::test]
async fn a_scan_for_one_device_returns_at_its_reply() {
    let window = Duration::from_secs(30);
    let rig = Rig::start(window).await;

    let start = Instant::now();
    let found = rig
        .transport
        .scan_for(&id(), window)
        .await
        .expect("the scan goes out")
        .expect("the simulated device answers");

    assert_eq!(found.id, id());
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "waited {:?} of a {window:?} window",
        start.elapsed()
    );
    assert_eq!(rig.transport.sku(&id()).as_deref(), Some(SKU));
}

#[tokio::test]
async fn a_scan_for_a_device_that_is_not_there_spends_the_window() {
    let rig = Rig::start(Duration::from_millis(200)).await;

    let found = rig
        .transport
        .scan_for(
            &DeviceId::new("11:22:33:44:55:66"),
            Duration::from_millis(200),
        )
        .await
        .expect("the scan goes out");

    assert!(found.is_none());
    // The device that did answer is recorded, whoever the scan was for.
    assert_eq!(rig.transport.devices().len(), 1);
}
