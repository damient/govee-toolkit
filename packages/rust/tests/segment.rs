//! Painting zones, against a simulated device. Neither the raw channel nor
//! `ptReal` answers, so a paint is only observable as the datagrams it puts
//! on the wire.

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

use common::{One, Rig, SKU, hex, wait_for};

async fn rig() -> Rig {
    Rig::start(
        Config::default(),
        Catalog::embedded().expect("catalog"),
        SKU,
    )
    .await
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

/// The frames of every `ptReal` datagram, in the order they arrived.
fn ptreal(simulator: &Simulator) -> Vec<Vec<String>> {
    simulator
        .received()
        .into_iter()
        .filter(|received| received.cmd == "ptReal")
        .map(|received| {
            received.data["command"]
                .as_array()
                .expect("a list of frames")
                .iter()
                .map(|frame| {
                    hex(&BASE64
                        .decode(frame.as_str().expect("a string"))
                        .expect("base64"))
                })
                .collect()
        })
        .collect()
}

async fn painted(simulator: &Simulator) -> Vec<String> {
    wait_for(|| ptreal(simulator).pop())
        .await
        .unwrap_or_default()
}

fn paint(colors: &[[u8; 3]], resolution: Resolution) -> Paint<'_> {
    Paint {
        zones: None,
        colors,
        resolution,
        gradient: false,
    }
}

fn gradient(colors: &[[u8; 3]], resolution: Resolution) -> Paint<'_> {
    Paint {
        gradient: true,
        ..paint(colors, resolution)
    }
}

/// The `interpolation off, in ptReal` vector of
/// tests/fixtures/golden/lan/H61A0.json.
const GRADIENT_OFF: &str = "33a3000000000000000000000000000000000090";

/// The zones that the mask field of a masked frame names.
fn mask(frame: &str) -> u16 {
    let field = u16::from_str_radix(&frame[24..28], 16).expect("hex");
    field.swap_bytes()
}

/// One datagram carries the setting and one frame per distinct color. Three
/// zones spread over the 15 groups of the mask, so no group keeps an old
/// color.
#[tokio::test]
async fn a_paint_of_every_zone_goes_in_one_datagram() {
    let rig = rig().await;
    rig.govee
        .one()
        .segment(&paint(
            &[[255, 0, 0], [0, 255, 0], [0, 0, 255]],
            Resolution::Exact(3),
        ))
        .await
        .expect("the paint goes out");

    let datagram = painted(&rig.simulator).await;
    assert_eq!(ptreal(&rig.simulator).len(), 1, "one datagram");
    assert_eq!(datagram.len(), 4, "{datagram:?}");
    assert_eq!(datagram[0], GRADIENT_OFF);
    for (frame, color) in datagram[1..].iter().zip(["ff0000", "00ff00", "0000ff"]) {
        assert!(frame.starts_with(&format!("33051501{color}")), "{frame}");
    }
    let masks: Vec<u16> = datagram[1..].iter().map(|frame| mask(frame)).collect();
    assert_eq!(masks, [0x001f, 0x03e0, 0x7c00]);
    assert!(
        frames(&rig.simulator).is_empty(),
        "nothing went out on razer"
    );
}

/// The app count of the device file is 10 zones: one color spreads over the
/// 15 groups of the mask.
#[tokio::test]
async fn one_color_paints_every_group_of_the_mask() {
    let rig = rig().await;
    rig.govee
        .one()
        .segment(&paint(&[[255, 0, 0]], Resolution::App))
        .await
        .expect("the paint goes out");

    let datagram = painted(&rig.simulator).await;
    assert_eq!(datagram.len(), 2, "{datagram:?}");
    assert_eq!(mask(&datagram[1]), 0x7fff);
}

#[tokio::test]
async fn a_paint_of_every_led_is_refused() {
    let rig = rig().await;
    let error = rig
        .govee
        .one()
        .segment(&paint(&[[255, 0, 0]], Resolution::Native))
        .await
        .expect_err("the mask names 15 groups");

    assert_eq!(error.code(), "native_zones_unreachable");
}

/// The `interpolation on, in ptReal` vector of
/// tests/fixtures/golden/lan/H61A0.json.
const GRADIENT_ON: &str = "33a3010000000000000000000000000000000091";

/// The setting travels alone, as the `ble` frame inside `ptReal`.
#[tokio::test]
async fn the_gradient_alone_goes_out_in_ptreal() {
    let rig = rig().await;
    rig.govee
        .one()
        .gradient(true)
        .await
        .expect("lan declares the setting in ptReal");

    assert_eq!(painted(&rig.simulator).await, vec![GRADIENT_ON.to_owned()]);
    assert!(
        frames(&rig.simulator).is_empty(),
        "nothing went out on razer"
    );
}

/// The setting rides in the datagram of the mask.
#[tokio::test]
async fn a_gradient_paint_carries_the_setting_first() {
    let rig = rig().await;
    rig.govee
        .one()
        .segment(&gradient(&[[255, 0, 0]], Resolution::App))
        .await
        .expect("the paint goes out");

    let datagram = painted(&rig.simulator).await;
    assert_eq!(ptreal(&rig.simulator).len(), 1, "one datagram");
    assert_eq!(datagram[0], GRADIENT_ON);
    assert!(datagram[1].starts_with("33051501ff0000"), "{datagram:?}");
}

#[tokio::test]
async fn a_gradient_paint_reaches_the_groups_of_the_mask() {
    let rig = rig().await;
    let error = rig
        .govee
        .one()
        .segment(&gradient(&[[255, 0, 0]], Resolution::Exact(30)))
        .await
        .expect_err("the mask names 15 groups");

    assert_eq!(error.code(), "zone_count_unsupported");
    assert!(ptreal(&rig.simulator).is_empty(), "nothing went out");
}

#[tokio::test]
async fn a_list_that_states_another_count_than_the_resolution_is_refused() {
    let rig = rig().await;
    let error = rig
        .govee
        .one()
        .segment(&paint(&[[255, 0, 0], [0, 255, 0]], Resolution::App))
        .await
        .expect_err("10 zones, 2 colors");

    assert_eq!(error.code(), "color_count_mismatch");
    assert!(frames(&rig.simulator).is_empty(), "nothing went out");
}

#[tokio::test]
async fn a_zone_list_takes_one_color() {
    let rig = rig().await;
    let error = rig
        .govee
        .one()
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

/// The mask frame is the `ble` frame inside `ptReal`. It paints the normal
/// mode, not the raw channel, so nothing arms that channel first. The
/// `interpolation off, in ptReal` and `red over zones 0 to 4, in ptReal`
/// vectors of tests/fixtures/golden/lan/H61A0.json, in one datagram.
#[tokio::test]
async fn a_zone_list_goes_out_in_ptreal_and_arms_nothing() {
    let rig = rig().await;
    rig.govee
        .one()
        .segment(&Paint {
            zones: Some(&[0, 1, 2, 3, 4]),
            colors: &[[255, 0, 0]],
            resolution: Resolution::App,
            gradient: false,
        })
        .await
        .expect("the paint goes out");

    assert_eq!(
        painted(&rig.simulator).await,
        [GRADIENT_OFF, "33051501ff000000000000001f000000000000c2"]
    );
    assert_eq!(ptreal(&rig.simulator).len(), 1, "one datagram");
    assert!(
        frames(&rig.simulator).is_empty(),
        "nothing went out on razer"
    );
}
