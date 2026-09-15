//! The loopback rig both integration suites are built on.
//!
//! A transport and a simulator must agree on four addresses. One place holds
//! that agreement, so the two suites cannot drift when `Endpoints` changes
//! shape.

#![allow(dead_code, clippy::expect_used, clippy::format_collect)]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use govee_toolkit::lan::{DeviceId, Endpoints, Transport};
use govee_toolkit::{Catalog, Config, Govee};
use govee_toolkit_sim::Simulator;

pub(crate) const MAC: &str = "AA:BB:CC:DD:EE:FF";
pub(crate) const SKU: &str = "H61A0";

pub(crate) fn id() -> DeviceId {
    DeviceId::new(MAC)
}

/// One simulated device, on the loopback with ephemeral ports.
pub(crate) async fn simulator(sku: &str) -> Simulator {
    Simulator::start(govee_toolkit_sim::Options::loopback(MAC, sku))
        .await
        .expect("the simulator binds")
}

/// Point a transport at that simulator, with no multicast.
pub(crate) fn endpoints(simulator: &Simulator) -> Endpoints {
    Endpoints {
        scan_target: simulator.scan_addr().expect("scan address"),
        reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        control_port: simulator.control_addr().expect("control address").port(),
        multicast_group: None,
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Poll `check` until it yields, for at most a second.
///
/// UDP on the loopback is fast but not synchronous, and the work under test
/// runs on its own task. A fixed sleep is flaky or slow.
pub(crate) async fn wait_for<T>(mut check: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if let Some(value) = check() {
            return Some(value);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    check()
}

/// An SDK attached to one simulated device, already scanned.
pub(crate) struct Rig {
    pub(crate) govee: Govee,
    pub(crate) simulator: Simulator,
}

impl Rig {
    /// The timings belong to the suites: short enough that a test does not
    /// wait on a refresh, long enough for the loopback round trip.
    pub(crate) async fn start(mut config: Config, catalog: Catalog, sku: &str) -> Self {
        let simulator = simulator(sku).await;

        config.lan.cache_disabled = true;
        config.lan.refresh_interval_seconds = None;
        config.lan.status_timeout_ms = 150;
        config.lan.scan_window_ms = 200;

        let transport = Transport::start(govee_toolkit::lan::Options {
            endpoints: endpoints(&simulator),
            ..config.lan.transport_options().expect("transport options")
        })
        .await
        .expect("the socket binds");

        let govee = Govee::attach(config, catalog, [Arc::new(transport) as Arc<_>])
            .expect("the configuration applies");
        // A scan is one UDP request that nothing retries, and the window above
        // is short. A loaded host can starve the simulator's task for the whole
        // window, which leaves the device unknown and fails a later send with
        // `UnknownDevice`. Ask again rather than widen the window: the rig is
        // ready as soon as one reply lands.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let found = govee.scan().await.expect("the scan goes out");
            if found.iter().any(|device| device.id == id()) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the simulator answered no scan in 10s"
            );
        }
        Self { govee, simulator }
    }
}
