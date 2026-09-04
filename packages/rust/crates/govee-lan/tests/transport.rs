//! The transport against a simulated device.
//!
//! Everything here runs on the loopback with ephemeral ports, so it needs no
//! hardware, no multicast and no privileges — which is the point: the
//! behaviour that matters most (a breaker that refuses without waiting) cannot
//! be checked against a device that is answering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use govee_core::{Args, Catalog, Encoded, Mode};
use govee_lan::{Endpoints, Event, Options, Policy, State, Transport, Verify};
use govee_sim::Simulator;

const MAC: &str = "AA:BB:CC:DD:EE:FF";
const SKU: &str = "H61A0";

/// A transport wired to one simulated device, and the device itself.
struct Rig {
    transport: Transport,
    simulator: Simulator,
    catalog: Catalog,
}

impl Rig {
    async fn start(policy: Policy) -> Self {
        Self::with_options(policy, |options| options).await
    }

    async fn with_options(policy: Policy, tweak: impl FnOnce(Options) -> Options) -> Self {
        let simulator = Simulator::start(govee_sim::Options::loopback(MAC, SKU))
            .await
            .expect("the simulator binds");

        let endpoints = Endpoints {
            scan_target: simulator.scan_addr().expect("scan address"),
            reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            control_port: simulator.control_addr().expect("control address").port(),
            multicast_group: None,
        };

        let options = tweak(Options {
            endpoints,
            policy,
            // Every test drives its own scan: a background one would make the
            // breaker assertions race.
            refresh_interval: None,
            scan_window: Duration::from_millis(200),
            status_timeout: Duration::from_millis(150),
            ..Options::default()
        });

        let transport = Transport::start(options).await.expect("the socket binds");
        Self {
            transport,
            simulator,
            catalog: Catalog::embedded().expect("embedded catalog"),
        }
    }

    fn command(&self, name: &str, args: &Args) -> Encoded {
        let device = self.catalog.device(SKU).expect("the SKU is in the catalog");
        govee_core::encode(device, Mode::Lan, name, args).expect("the command encodes")
    }

    fn power_on(&self) -> Encoded {
        self.command("power", &Args::new().int("on", 1))
    }

    fn status_request(&self) -> Encoded {
        self.command("status", &Args::new())
    }

    async fn discover(&self) {
        let found = self
            .transport
            .scan(Duration::from_millis(300))
            .await
            .expect("the scan goes out");
        assert_eq!(found.len(), 1, "the simulated device answers");
    }
}

fn id() -> govee_lan::DeviceId {
    govee_lan::DeviceId::new(MAC)
}

#[tokio::test]
async fn a_scan_finds_the_device_and_records_where_it_is() {
    let rig = Rig::start(Policy::default()).await;
    rig.discover().await;

    let known = rig.transport.devices();
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].id, id());
    assert_eq!(known[0].sku, SKU);
    assert_eq!(known[0].health.state, State::Ok);
    assert_eq!(rig.transport.sku(&id()).as_deref(), Some(SKU));
}

#[tokio::test]
async fn a_command_reaches_the_device() {
    let rig = Rig::start(Policy::default()).await;
    rig.discover().await;
    rig.simulator.clear();

    let sent = rig
        .transport
        .send(&id(), &rig.power_on(), Verify::None)
        .await
        .expect("the command goes out");
    assert_eq!(sent.mode, Mode::Lan);

    let received = wait_for(|| {
        let received = rig.simulator.received();
        (!received.is_empty()).then_some(received)
    })
    .await
    .expect("the device receives it");

    assert_eq!(received[0].cmd, sent.cmd);
    assert_eq!(received[0].data["value"], 1);
}

#[tokio::test]
async fn an_undiscovered_device_fails_rather_than_triggering_a_scan() {
    let rig = Rig::start(Policy::default()).await;
    // No scan has run: nothing is known, and asking must not go looking.
    let unknown = govee_lan::DeviceId::new("11:22:33:44:55:66");

    let error = rig
        .transport
        .send(&unknown, &rig.power_on(), Verify::None)
        .await
        .expect_err("an unknown device cannot be sent to");
    assert_eq!(error.code(), "unknown_device");
    assert_eq!(rig.simulator.received_count(), 0);
}

#[tokio::test]
async fn a_status_reply_is_correlated_by_source_address() {
    let rig = Rig::start(Policy::default()).await;
    rig.discover().await;
    rig.simulator.set_status(serde_json::json!({
        "onOff": 1, "brightness": 42, "color": { "r": 255, "g": 0, "b": 0 }, "colorTemInKelvin": 0
    }));

    let status = rig
        .transport
        .status(&id(), &rig.status_request())
        .await
        .expect("the device answers");

    assert_eq!(status.id, id());
    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(42));
    assert_eq!(status.color, Some([255, 0, 0]));
    assert!(!status.is_white());
    assert_eq!(rig.transport.last_status(&id()), Some(status));
}

#[tokio::test]
async fn concurrent_waiters_share_one_request() {
    let rig = Rig::start(Policy::default()).await;
    rig.discover().await;
    rig.simulator.set_status(serde_json::json!({ "onOff": 1 }));
    rig.simulator.clear();

    let request = rig.status_request();
    let id = id();
    let (first, second, third) = tokio::join!(
        rig.transport.status(&id, &request),
        rig.transport.status(&id, &request),
        rig.transport.status(&id, &request),
    );
    assert!(first.is_ok() && second.is_ok() && third.is_ok());
    assert_eq!(
        rig.simulator.received_count(),
        1,
        "three waiters, one datagram"
    );
}

#[tokio::test]
async fn silence_degrades_the_mode_and_is_then_refused_without_waiting() {
    let policy = Policy {
        degrade_after: 2,
        cooldown: Duration::from_secs(60),
        ..Policy::default()
    };
    let rig = Rig::start(policy).await;
    rig.discover().await;

    let mut events = rig.transport.events();
    rig.simulator.set_silent(true);

    let request = rig.status_request();
    for _ in 0..2 {
        let error = rig
            .transport
            .status(&id(), &request)
            .await
            .expect_err("a silent device answers nothing");
        assert_eq!(error.code(), "unreachable");
    }

    let health = rig.transport.health(&id()).expect("the device is known");
    assert_eq!(health.state, State::Degraded);
    assert!(!health.available);

    // The transition is observable, as docs/modes.md requires.
    let transition = loop {
        if let Event::HealthChanged { transition, .. } =
            events.recv().await.expect("the event stream is alive")
        {
            break transition;
        }
    };
    assert_eq!(
        (transition.from, transition.to),
        (State::Ok, State::Degraded)
    );

    // And the next command is refused from state already known: no round-trip,
    // no timeout, nothing on the wire.
    rig.simulator.clear();
    let started = Instant::now();
    let error = rig
        .transport
        .send(&id(), &rig.power_on(), Verify::None)
        .await
        .expect_err("a degraded mode refuses");
    assert_eq!(error.code(), "mode_unavailable");
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "refusing cost {:?}; it must not wait for the network",
        started.elapsed()
    );
    assert_eq!(rig.simulator.received_count(), 0);
}

#[tokio::test]
async fn answering_again_brings_the_mode_back() {
    let policy = Policy {
        degrade_after: 2,
        recover_after: 2,
        // Short enough that the test does not sleep for the real cooldown.
        cooldown: Duration::from_millis(50),
        ..Policy::default()
    };
    let rig = Rig::start(policy).await;
    rig.discover().await;
    rig.simulator.set_status(serde_json::json!({ "onOff": 1 }));

    let request = rig.status_request();
    rig.simulator.set_silent(true);
    for _ in 0..2 {
        let _ = rig.transport.status(&id(), &request).await;
    }
    assert_eq!(
        rig.transport.health(&id()).expect("known").state,
        State::Degraded
    );

    rig.simulator.set_silent(false);
    tokio::time::sleep(Duration::from_millis(60)).await;
    for _ in 0..2 {
        rig.transport
            .status(&id(), &request)
            .await
            .expect("the device answers again");
    }
    assert_eq!(rig.transport.health(&id()).expect("known").state, State::Ok);
}

#[tokio::test]
async fn fire_and_verify_records_the_silence_without_delaying_the_command() {
    let policy = Policy {
        degrade_after: 1,
        cooldown: Duration::from_secs(60),
        ..Policy::default()
    };
    let rig = Rig::with_options(policy, |options| Options {
        verify_interval: Some(Duration::ZERO),
        ..options
    })
    .await;
    rig.discover().await;
    rig.simulator.set_silent(true);

    let request = rig.status_request();
    let started = Instant::now();
    rig.transport
        .send(&id(), &rig.power_on(), Verify::With(&request))
        .await
        .expect("the command goes out");
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "the send path waited for the verification"
    );

    let state = wait_for(|| {
        let health = rig.transport.health(&id())?;
        (health.state != State::Ok).then_some(health.state)
    })
    .await
    .expect("the unanswered verification reaches the breaker");
    assert_eq!(state, State::Degraded);
}

#[tokio::test]
async fn the_cache_makes_a_restart_free_of_scanning() {
    let directory = std::env::temp_dir().join(format!("govee-lan-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let path = directory.join("devices.json");

    let rig = Rig::with_options(Policy::default(), |options| Options {
        cache: govee_lan::Cache::load(&path).expect("an empty cache"),
        ..options
    })
    .await;
    rig.discover().await;
    rig.transport.save_cache().expect("the cache is written");

    // A second transport on the same cache, wired to the same device, with no
    // scan of its own.
    let endpoints = Endpoints {
        scan_target: rig.simulator.scan_addr().expect("scan address"),
        reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        control_port: rig
            .simulator
            .control_addr()
            .expect("control address")
            .port(),
        multicast_group: None,
    };
    let restarted = Transport::start(Options {
        endpoints,
        refresh_interval: None,
        cache: govee_lan::Cache::load(&path).expect("the cache reloads"),
        ..Options::default()
    })
    .await
    .expect("the socket binds");

    assert_eq!(restarted.devices().len(), 1);
    rig.simulator.clear();
    restarted
        .send(&id(), &rig.power_on(), Verify::None)
        .await
        .expect("the first command needs no scan");

    let received = wait_for(|| {
        let received = rig.simulator.received();
        (!received.is_empty()).then_some(received)
    })
    .await;
    assert!(received.is_some());

    let _ = std::fs::remove_dir_all(&directory);
}

/// Poll `check` until it yields, for at most a second.
///
/// UDP on the loopback is fast but not synchronous, and the verification runs
/// on its own task: a fixed sleep would either be flaky or slow.
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
