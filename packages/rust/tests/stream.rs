//! Segment streaming, against a simulated device.
//!
//! The raw channel never answers, so what a stream does is only observable as
//! the datagrams it puts on the wire. That is exactly what the simulator
//! records, and every assertion here reads the frames back out of it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::format_collect
)]

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use govee_toolkit::stream::{Rate, StreamOptions, Zones};
use govee_toolkit::{Args, Catalog, Config};
use govee_toolkit_sim::Simulator;

mod common;

use common::{Rig, SKU, hex, id, wait_for};

/// Fast enough that a test does not wait on the measured rate, slow enough that
/// several writes land inside one interval.
const TEST_HZ: f64 = 20.0;

/// The frame that closes the raw channel, as `H61A0` declares it.
const DISARM: &str = "bb0001b1000b";

async fn rig() -> Rig {
    rig_with(Catalog::embedded().expect("catalog"), SKU).await
}

async fn rig_with(catalog: Catalog, sku: &str) -> Rig {
    Rig::start(Config::default(), catalog, sku).await
}

/// Open a stream and forget the arming frame, so a test sees only what it asked
/// for. The frame has to arrive before it can be cleared: clearing straight
/// away would race the datagram and leave it in the next assertion.
async fn open(rig: &Rig, options: StreamOptions) -> govee_toolkit::SegmentStream {
    let stream = rig
        .govee
        .device(&id())
        .open_stream(options)
        .await
        .expect("the stream opens");
    wait_for(|| (!frames(&rig.simulator).is_empty()).then_some(())).await;
    rig.simulator.clear();
    stream
}

/// Every raw frame the simulator has received, as hex.
fn frames(simulator: &Simulator) -> Vec<String> {
    simulator
        .received()
        .into_iter()
        .filter_map(|received| received.data.get("pt")?.as_str().map(ToOwned::to_owned))
        .filter_map(|pt| BASE64.decode(pt).ok())
        .map(|bytes| hex(&bytes))
        .collect()
}

fn options(zones: Zones) -> StreamOptions {
    StreamOptions {
        zones,
        rate: Rate::Fixed(TEST_HZ),
        gradient: false,
    }
}

#[tokio::test]
async fn opening_arms_the_channel() {
    let rig = rig().await;
    let _stream = rig
        .govee
        .device(&id())
        .open_stream(options(Zones::App))
        .await
        .expect("the stream opens");

    assert_eq!(
        wait_for(|| frames(&rig.simulator).first().cloned()).await,
        Some("bb0001b1010a".to_owned())
    );
}

#[tokio::test]
async fn an_idle_stream_sends_nothing() {
    let rig = rig().await;
    let _stream = open(&rig, options(Zones::App)).await;

    tokio::time::sleep(Duration::from_secs_f64(4.0 / TEST_HZ)).await;
    assert!(frames(&rig.simulator).is_empty());
}

#[tokio::test]
async fn a_write_reaches_the_device_as_one_frame() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Exact(2))).await;
    stream.set_all(&[[255, 0, 0], [0, 255, 0]]).unwrap();

    let frame = wait_for(|| frames(&rig.simulator).first().cloned())
        .await
        .expect("a frame goes out");
    assert_eq!(frame, "bb0008b00002ff000000ff0001");
}

#[tokio::test]
async fn a_repainted_zone_keeps_the_others() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Exact(2))).await;

    stream.fill([255, 0, 0]).unwrap();
    wait_for(|| (!frames(&rig.simulator).is_empty()).then_some(())).await;
    stream.set_zone(1, [0, 255, 0]).unwrap();

    assert_eq!(
        wait_for(|| (stream.buffer() == [[255, 0, 0], [0, 255, 0]]).then_some(())).await,
        Some(())
    );
    assert_eq!(
        wait_for(|| frames(&rig.simulator)
            .into_iter()
            .find(|f| f == "bb0008b00002ff000000ff0001"))
        .await,
        Some("bb0008b00002ff000000ff0001".to_owned())
    );
}

#[tokio::test]
async fn only_the_latest_write_is_sent() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Exact(1))).await;

    // Five writes inside one interval: the source is not throttled, and four of
    // the frames it asked for never existed.
    for level in 1..=5u8 {
        stream.set_all(&[[level, 0, 0]]).unwrap();
    }

    wait_for(|| (stream.frames_sent() > 0).then_some(())).await;
    tokio::time::sleep(Duration::from_secs_f64(2.0 / TEST_HZ)).await;

    assert_eq!(stream.frames_sent(), 1);
    assert_eq!(stream.frames_superseded(), 4);
    assert_eq!(
        frames(&rig.simulator),
        vec!["bb0005b000010500000a".to_owned()]
    );
}

#[tokio::test]
async fn closing_disarms_the_channel() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::App)).await;
    stream.close().await.expect("the disarm goes out");

    // Exactly one: the handle asks the emitting task to disarm and reports what
    // it did, rather than sending a frame of its own on top of it.
    wait_for(|| frames(&rig.simulator).first().cloned()).await;
    assert_eq!(frames(&rig.simulator), vec![DISARM.to_owned()]);
}

#[tokio::test]
async fn dropping_disarms_the_channel() {
    let rig = rig().await;
    drop(open(&rig, options(Zones::App)).await);

    assert_eq!(
        wait_for(|| frames(&rig.simulator).first().cloned()).await,
        Some(DISARM.to_owned())
    );
}

/// A handle can outlive the runtime it was opened on — a binding holding one
/// past `block_on`, or a test that shuts the runtime down first. Disarming from
/// `Drop` means signalling the emitting task, never spawning one: nothing goes
/// out here, because nothing is left to send it, and the drop stays silent.
#[test]
fn dropping_after_the_runtime_is_gone_does_not_panic() {
    let runtime = tokio::runtime::Runtime::new().expect("the runtime starts");
    let (rig, stream) = runtime.block_on(async {
        let rig = rig().await;
        let stream = open(&rig, options(Zones::App)).await;
        (rig, stream)
    });
    runtime.shutdown_timeout(Duration::from_millis(100));

    drop(stream);
    drop(rig);
}

#[tokio::test]
async fn native_resolution_comes_from_the_measured_unit() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Native)).await;
    assert_eq!(stream.zones(), 42);
}

#[tokio::test]
async fn the_app_zone_count_is_not_the_native_one() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::App)).await;
    assert_eq!(stream.zones(), 10);
}

#[tokio::test]
async fn a_measured_rate_is_read_off_the_device_file() {
    let rig = rig().await;
    // 42 zones falls in the 60-zone row of `devices/H61A0.yaml`.
    let stream = open(
        &rig,
        StreamOptions {
            zones: Zones::Native,
            ..StreamOptions::default()
        },
    )
    .await;
    assert!((stream.rate_hz() - 25.0).abs() < f64::EPSILON);
}

/// The device file names its arguments as well as its commands, and the SDK
/// reaches both through `role:` only.
#[tokio::test]
async fn the_argument_names_come_from_the_device_file() {
    const RENAMED: &str = include_str!("fixtures/renamed-args.yaml");

    let catalog = Catalog::from_sources([("renamed-args.yaml", RENAMED)]).expect("catalog");
    let rig = rig_with(catalog, "HTEST2").await;
    let stream = open(&rig, options(Zones::App)).await;
    stream.set_all(&[[255, 0, 0], [0, 255, 0]]).unwrap();

    assert_eq!(
        wait_for(|| frames(&rig.simulator).first().cloned()).await,
        Some("bb0008b00002ff000000ff0001".to_owned())
    );
}

#[tokio::test]
async fn native_resolution_nobody_measured_is_refused() {
    const UNMEASURED: &str = include_str!("fixtures/unmeasured.yaml");

    let catalog = Catalog::from_sources([("unmeasured.yaml", UNMEASURED)]).expect("catalog");
    let rig = rig_with(catalog, "HTEST0").await;

    let error = rig
        .govee
        .device(&id())
        .open_stream(options(Zones::Native))
        .await
        .expect_err("an unmeasured unit has no native resolution");
    assert_eq!(error.code(), "zone_count_unknown");
}

#[tokio::test]
async fn a_file_naming_no_segment_command_is_refused() {
    const NO_SEGMENTS: &str = include_str!("fixtures/no-segments.yaml");

    let catalog = Catalog::from_sources([("no-segments.yaml", NO_SEGMENTS)]).expect("catalog");
    let rig = rig_with(catalog, "HTEST1").await;

    let error = rig
        .govee
        .device(&id())
        .open_stream(options(Zones::App))
        .await
        .expect_err("nothing claims the role");
    assert_eq!(error.code(), "no_segment_command");
}

#[tokio::test]
async fn a_frame_of_the_wrong_length_is_refused_rather_than_padded() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Exact(3))).await;

    let error = stream
        .set_all(&[[255, 0, 0]])
        .expect_err("one colour is not three zones");
    assert_eq!(error.code(), "zone_count_mismatch");
}

#[tokio::test]
async fn a_stream_survives_a_device_that_stops_answering() {
    let rig = rig().await;
    let stream = open(&rig, options(Zones::Exact(1))).await;

    // Verification is what learns silence, and a stream asks for none: the
    // breaker keeps letting frames through, and nothing stops the task.
    rig.simulator.set_silent(true);
    stream.fill([255, 0, 0]).unwrap();
    wait_for(|| (stream.frames_sent() > 0).then_some(())).await;

    assert!(stream.error().is_none());
    assert!(stream.frames_sent() > 0);
}

#[tokio::test]
async fn a_gradient_stream_says_so_in_every_frame() {
    let rig = rig().await;
    let stream = open(
        &rig,
        StreamOptions {
            zones: Zones::Exact(1),
            rate: Rate::Fixed(TEST_HZ),
            gradient: true,
        },
    )
    .await;
    stream.fill([255, 0, 0]).unwrap();

    let frame = wait_for(|| frames(&rig.simulator).first().cloned())
        .await
        .expect("a frame goes out");
    assert_eq!(frame, "bb0005b00101ff0000f1");
}

#[tokio::test]
async fn a_stream_is_only_opened_for_a_device_a_mode_can_reach() {
    let rig = rig().await;
    let error = rig
        .govee
        .device(&govee_toolkit::DeviceId::new("11:22:33:44:55:66"))
        .open_stream(options(Zones::App))
        .await
        .expect_err("nothing was ever discovered under that identity");
    assert_eq!(error.code(), "unknown_device");
}

#[tokio::test]
async fn a_stream_does_not_power_the_device_on() {
    let rig = rig().await;
    let _stream = rig
        .govee
        .device(&id())
        .open_stream(options(Zones::App))
        .await
        .expect("the stream opens");

    // `turn(1)` precedes arming, and it is the caller's to send: this crate
    // names no command of its own. See docs/protocol/lan.md 2.3.
    let commands: Vec<String> = rig
        .simulator
        .received()
        .into_iter()
        .map(|received| received.cmd)
        .collect();
    assert!(!commands.contains(&"turn".to_owned()), "{commands:?}");

    // The caller's own power command still goes out as usual.
    rig.govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("power reaches the device");
}
