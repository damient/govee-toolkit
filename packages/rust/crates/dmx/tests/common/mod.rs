//! The rig every integration test drives: one simulated device on the
//! loopback, and one device that never answered.
//!
//! The simulator plays the wire and interprets nothing, so an assertion counts
//! the datagrams it recorded, or reads the command name off one, rather than
//! decoding a look out of them.

#![allow(
    dead_code,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use govee_toolkit::codec::Catalog;
use govee_toolkit::lan::{Endpoints, Transport};
use govee_toolkit::{Config, DeviceId, Govee};
use govee_toolkit_dmx::apply::{Look, Timing};
use govee_toolkit_dmx::patch::{Patch, Rig};
use govee_toolkit_sim::{Options, Simulator};

/// The device the simulator answers for.
pub(crate) const REACHED: &str = "AA:BB:CC:DD:EE:01";
/// A device that never answered a scan. Every write to it fails.
pub(crate) const GONE: &str = "AA:BB:CC:DD:EE:02";
pub(crate) const SKU: &str = "H61A0";

/// Short enough that a test does not wait on it, long enough that the
/// loopback round trip lands inside it.
pub(crate) const SILENCE: Duration = Duration::from_millis(300);
/// Far past what any test here waits, so a refresh drives no assertion.
pub(crate) const REFRESH: Duration = Duration::from_secs(60);
/// Short enough that a test waits it out, long enough that a dip through 0
/// lands inside it.
pub(crate) const OFF_DELAY: Duration = Duration::from_millis(300);

/// Two fixtures on one universe: the simulated one, then the one that is
/// gone. `loss` is what both show once the sender goes quiet.
pub(crate) fn patch(loss: &str) -> String {
    let entry = |device: &str, address: u16| {
        format!(
            "  - device: \"{device}\"\n    universe: 0\n    address: {address}\n    personality: full\n    on_signal_loss: {loss}\n"
        )
    };
    format!("patch:\n{}{}", entry(REACHED, 1), entry(GONE, 7))
}

pub(crate) fn rig(catalog: &Catalog, loss: &str) -> Rig {
    resolve(catalog, &patch(loss))
}

/// The simulated device alone, on the `segment` personality.
pub(crate) fn segment_rig(catalog: &Catalog) -> Rig {
    let entry = format!(
        "patch:\n  - device: \"{REACHED}\"\n    universe: 0\n    address: 1\n    personality: segment\n"
    );
    resolve(catalog, &entry)
}

fn resolve(catalog: &Catalog, text: &str) -> Rig {
    let patch = Patch::parse(text, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let device = catalog.device(SKU).expect("the SKU resolves");
    patch
        .resolve(|_| Some(device), |_| Some(device))
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

/// The SDK, attached to one simulated device and already scanned.
pub(crate) async fn govee(simulator: &Simulator) -> Govee {
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

pub(crate) fn lit() -> Look {
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
pub(crate) async fn wait_for_lit(simulator: &Simulator) {
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

pub(crate) async fn simulator() -> Simulator {
    Simulator::start(Options::loopback(REACHED, SKU))
        .await
        .expect("the simulator binds")
}

/// The three waits every test here runs with.
pub(crate) fn timing() -> Timing {
    Timing {
        refresh: REFRESH,
        silence: SILENCE,
        off_delay: OFF_DELAY,
    }
}

pub(crate) fn cmds(simulator: &Simulator) -> Vec<String> {
    simulator
        .received()
        .into_iter()
        .map(|received| received.cmd)
        .collect()
}

/// An `ArtDmx` packet, in the layout `docs/dmx.md` documents. The data length
/// is what `slots` carries, so a short packet leaves the rest of the universe
/// at 0.
pub(crate) fn artdmx(universe: u16, sequence: u8, slots: &[u8]) -> Vec<u8> {
    let mut bytes = b"Art-Net\0".to_vec();
    bytes.extend_from_slice(&0x5000u16.to_le_bytes());
    bytes.extend_from_slice(&14u16.to_be_bytes());
    bytes.push(sequence);
    bytes.push(0);
    let [net, sub_uni] = universe.to_be_bytes();
    bytes.push(sub_uni);
    bytes.push(net);
    let length = u16::try_from(slots.len()).unwrap_or(0);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(slots);
    bytes
}

pub(crate) fn payload(simulator: &Simulator, cmd: &str) -> Option<serde_json::Value> {
    simulator
        .received()
        .into_iter()
        .find(|received| received.cmd == cmd)
        .map(|received| received.data)
}
