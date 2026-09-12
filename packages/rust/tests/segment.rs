//! Painting zones one frame at a time, against a simulated device.
//!
//! The raw channel never answers, so a paint is only observable as the
//! datagrams it puts on the wire. `H61A0` paints over `lan` with one frame that
//! states every zone, which is the frame that reaches one LED at a time.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::format_collect
)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use govee_toolkit::{Catalog, Config, Paint, Resolution};
use govee_toolkit_sim::Simulator;

mod common;

use common::{Rig, SKU, hex, id, wait_for};

/// The arming frame every paint sends first, as `H61A0` declares it.
const ARM: &str = "bb0001b1010a";

async fn rig() -> Rig {
    Rig::start(
        Config::default(),
        Catalog::embedded().expect("catalog"),
        SKU,
    )
    .await
}

/// The frames the simulator has received once it holds `count` of them.
///
/// UDP on the loopback is fast but not synchronous: a send returns before the
/// datagram arrives.
async fn awaited(simulator: &Simulator, count: usize) -> Vec<String> {
    wait_for(|| {
        let frames = frames(simulator);
        (frames.len() >= count).then_some(frames)
    })
    .await
    .unwrap_or_default()
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

fn paint(colors: &[[u8; 3]], resolution: Resolution) -> Paint<'_> {
    Paint {
        zones: None,
        colors,
        resolution,
        gradient: false,
    }
}

#[tokio::test]
async fn one_color_fills_every_zone_the_resolution_names() {
    let rig = rig().await;
    rig.govee
        .device(&id())
        .segment(&paint(&[[255, 0, 0]], Resolution::Exact(2)))
        .await
        .expect("the paint goes out");

    assert_eq!(
        awaited(&rig.simulator, 2).await,
        vec![ARM.to_owned(), "bb0008b00002ff0000ff000001".to_owned()]
    );
}

/// `lan` carries the interpolation setting inside the frame that paints, so it
/// marks no `role: segment_gradient`. The call fails rather than repaint: the
/// colors the device shows are not held here.
#[tokio::test]
async fn setting_the_gradient_alone_is_refused_where_only_a_paint_carries_it() {
    let rig = rig().await;

    let refused = rig
        .govee
        .device(&id())
        .gradient(true)
        .await
        .expect_err("lan carries the setting in the painting frame");

    assert!(
        matches!(refused, govee_toolkit::Error::NoRoleCommand { .. }),
        "{refused:?}"
    );
    assert!(frames(&rig.simulator).is_empty(), "nothing was painted");
}

/// `measurements.arm_settle_ms` of `devices/H61A0.yaml`.
///
/// The firmware renders nothing when the paint follows the arming frame at
/// once, and answers nothing either way, so this wait is what makes the first
/// paint visible. See `docs/protocol/lan.md` 2.3.
#[tokio::test]
async fn the_first_paint_waits_for_the_channel_to_arm() {
    let rig = rig().await;
    let start = std::time::Instant::now();

    rig.govee
        .device(&id())
        .segment(&paint(&[[255, 0, 0]], Resolution::Exact(2)))
        .await
        .expect("the paint goes out");

    assert!(start.elapsed() >= std::time::Duration::from_millis(50));
}

#[tokio::test]
async fn a_color_list_states_one_zone_each() {
    let rig = rig().await;
    rig.govee
        .device(&id())
        .segment(&paint(&[[255, 0, 0], [0, 255, 0]], Resolution::Exact(2)))
        .await
        .expect("the paint goes out");

    assert_eq!(
        awaited(&rig.simulator, 2).await.last().map(String::as_str),
        Some("bb0008b00002ff000000ff0001")
    );
}

#[tokio::test]
async fn native_resolution_states_every_addressable_led() {
    let rig = rig().await;
    let mut colors = vec![[0, 0, 0]; 42];
    colors[0] = [255, 0, 0];

    rig.govee
        .device(&id())
        .segment(&paint(&colors, Resolution::Native))
        .await
        .expect("the paint goes out");

    // The `one lit zone at native resolution` vector of
    // tests/fixtures/golden/lan/H61A0.json.
    let frame = awaited(&rig.simulator, 2)
        .await
        .last()
        .cloned()
        .expect("a frame");
    assert!(frame.starts_with("bb0080b0002aff0000"));
    // bb, the 16-bit length, the op, the gradient, the count, then the xor.
    assert_eq!(frame.len(), 2 * (6 + 3 * 42 + 1));
}

#[tokio::test]
async fn a_list_that_states_another_count_than_the_resolution_is_refused() {
    let rig = rig().await;
    let error = rig
        .govee
        .device(&id())
        .segment(&paint(&[[255, 0, 0], [0, 255, 0]], Resolution::Native))
        .await
        .expect_err("42 zones, 2 colors");

    assert_eq!(error.code(), "color_count_mismatch");
    assert!(frames(&rig.simulator).is_empty(), "nothing went out");
}

#[tokio::test]
async fn a_resolution_the_unit_renders_as_a_smaller_one_is_refused() {
    let rig = rig().await;
    let error = rig
        .govee
        .device(&id())
        .segment(&paint(&[[255, 0, 0]], Resolution::Exact(30)))
        .await
        .expect_err("this unit refines at 21, then at 42");

    assert_eq!(error.code(), "resolution_not_distinct");
    assert!(frames(&rig.simulator).is_empty(), "nothing went out");
}

#[tokio::test]
async fn a_zone_list_takes_one_color() {
    let rig = rig().await;
    let error = rig
        .govee
        .device(&id())
        .segment(&Paint {
            zones: Some(&[0, 1]),
            colors: &[[255, 0, 0], [0, 255, 0]],
            resolution: Resolution::App,
            gradient: false,
        })
        .await
        .expect_err("a mask carries one color");

    assert_eq!(error.code(), "zone_list_color_count");
}
