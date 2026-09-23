//! How many zones a stream carries, and what its frame states, against a
//! simulated device.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use govee_toolkit::stream::{Rate, Resolution, StreamOptions};
use govee_toolkit::{Catalog, Config};
use govee_toolkit_sim::Simulator;

mod common;

use common::{Rig, SKU, hex, id, wait_for};

const TEST_HZ: f64 = 20.0;

async fn rig() -> Rig {
    rig_with(Catalog::embedded().expect("catalog"), SKU).await
}

async fn rig_with(catalog: Catalog, sku: &str) -> Rig {
    Rig::start(Config::default(), catalog, sku).await
}

/// Open a stream and forget the arming frame, so a test sees only what it asked
/// for.
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

fn frames(simulator: &Simulator) -> Vec<String> {
    simulator
        .received()
        .into_iter()
        .filter_map(|received| received.data.get("pt")?.as_str().map(ToOwned::to_owned))
        .filter_map(|pt| BASE64.decode(pt).ok())
        .map(|bytes| hex(&bytes))
        .collect()
}

fn options(resolution: Resolution) -> StreamOptions {
    StreamOptions {
        resolution,
        rate: Rate::Fixed(TEST_HZ),
        gradient: false,
    }
}

/// 15 groups over 42 LEDs: the caller paints the groups, and the frame states
/// every LED. LEDs 0 to 2 fall in group 0.
#[tokio::test]
async fn a_group_stream_paints_each_group_over_its_own_leds() {
    let rig = rig().await;
    let stream = open(&rig, options(Resolution::Groups)).await;
    assert_eq!(stream.zones(), 15);

    let mut groups = [[0, 0, 0]; 15];
    groups[0] = [255, 0, 0];
    stream.set_all(&groups).unwrap();

    let frame = wait_for(|| frames(&rig.simulator).first().cloned())
        .await
        .expect("a frame goes out");
    assert_eq!(&frame[6..12], "b0002a", "42 colors");
    assert_eq!(&frame[12..36], "ff0000ff0000ff0000000000");
}

#[tokio::test]
async fn native_resolution_comes_from_the_measured_unit() {
    let rig = rig().await;
    let stream = open(&rig, options(Resolution::Native)).await;
    assert_eq!(stream.zones(), 42);
}

#[tokio::test]
async fn the_app_zone_count_is_not_the_native_one() {
    let rig = rig().await;
    let stream = open(&rig, options(Resolution::App)).await;
    assert_eq!(stream.zones(), 10);
}

#[tokio::test]
async fn a_measured_rate_is_read_off_the_device_file() {
    let rig = rig().await;
    // 42 zones falls in the 60-zone row of `devices/H61A0.yaml`.
    let stream = open(
        &rig,
        StreamOptions {
            resolution: Resolution::Native,
            ..StreamOptions::default()
        },
    )
    .await;
    assert!((stream.rate_hz() - 25.0).abs() < f64::EPSILON);
}

/// The frame and not the zone count sets the rate: 15 groups fall in the
/// 20-zone row, and the 42 LEDs the frame states in the 60-zone row.
#[tokio::test]
async fn a_group_stream_is_paced_by_the_leds_its_frame_states() {
    let rig = rig().await;
    let stream = open(
        &rig,
        StreamOptions {
            resolution: Resolution::Groups,
            ..StreamOptions::default()
        },
    )
    .await;
    assert!((stream.rate_hz() - 25.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn native_resolution_nobody_measured_is_refused() {
    const UNMEASURED: &str = include_str!("fixtures/unmeasured.yaml");

    let catalog = Catalog::from_sources([("unmeasured.yaml", UNMEASURED)]).expect("catalog");
    let rig = rig_with(catalog, "HTEST0").await;

    let error = rig
        .govee
        .device(&id())
        .open_stream(options(Resolution::Native))
        .await
        .expect_err("an unmeasured unit has no native resolution");
    assert_eq!(error.code(), "zone_count_unknown");
}
