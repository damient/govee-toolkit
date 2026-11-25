//! Storing a drawing of the LEDs, against a simulated device. `ptReal`
//! answers nothing, so a drawing is only observable as the datagrams it puts
//! on the wire.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use govee_toolkit::codec::Args;
use govee_toolkit::{Catalog, Config, Drawing, Resolution};
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

async fn drawn(simulator: &Simulator) -> Vec<String> {
    wait_for(|| ptreal(simulator).pop())
        .await
        .unwrap_or_default()
}

fn drawing<'a>(colors: &'a [[u8; 3]], resolution: Resolution, args: &'a Args) -> Drawing<'a> {
    Drawing {
        cells: None,
        colors,
        resolution,
        args,
    }
}

/// The `every LED held still by the defaults` vector of
/// tests/fixtures/golden/lan/H61A0.json.
#[tokio::test]
async fn a_drawing_of_every_led_takes_the_defaults() {
    let rig = rig().await;
    let mut colors = vec![[255, 0, 0]; 42];
    colors[..10].fill([0, 255, 0]);

    rig.govee
        .one()
        .draw(&drawing(&colors, Resolution::Native, &Args::new()))
        .await
        .expect("the drawing goes out");

    assert_eq!(
        drawn(&rig.simulator).await,
        [
            "a300010403090064000000020a00ff000001023c",
            "a3010304050607080920ff00000a0b0c0d0e0f7e",
            "a302101112131415161718191a1b1c1d1e1f2081",
            "a3ff21222324252627282900000000000000007d",
            "33050a200300000000000000000000000000001f",
        ]
    );
    assert_eq!(ptreal(&rig.simulator).len(), 1, "one datagram, no arming");
}

/// The SDK spreads the zones over the LEDs: the firmware groups nothing in a
/// stored drawing.
#[tokio::test]
async fn a_zone_count_spreads_over_every_led() {
    let zones = rig().await;
    zones
        .govee
        .one()
        .draw(&drawing(
            &[[255, 0, 0], [0, 255, 0]],
            Resolution::Exact(2),
            &Args::new(),
        ))
        .await
        .expect("the drawing goes out");

    let leds = rig().await;
    let mut colors = vec![[255, 0, 0]; 42];
    colors[21..].fill([0, 255, 0]);
    leds.govee
        .one()
        .draw(&drawing(&colors, Resolution::Native, &Args::new()))
        .await
        .expect("the drawing goes out");

    assert_eq!(drawn(&zones.simulator).await, drawn(&leds.simulator).await);
}

/// A caller value wins over the default: the effect is the first byte of the
/// body, in the header frame.
#[tokio::test]
async fn an_argument_from_the_caller_wins_over_the_default() {
    let rig = rig().await;
    let args = Args::new().int("effect", 15);
    rig.govee
        .one()
        .draw(&Drawing {
            cells: Some(&[0, 1, 2]),
            colors: &[[0, 0, 255]],
            resolution: Resolution::App,
            args: &args,
        })
        .await
        .expect("the drawing goes out");

    let frames = drawn(&rig.simulator).await;
    assert!(frames[0].starts_with("a3000102030f00"), "{frames:?}");
}

#[tokio::test]
async fn a_cell_list_takes_one_color_or_one_per_cell() {
    let rig = rig().await;
    let error = rig
        .govee
        .one()
        .draw(&Drawing {
            cells: Some(&[0, 1, 2]),
            colors: &[[0, 0, 255], [255, 0, 0]],
            resolution: Resolution::App,
            args: &Args::new(),
        })
        .await
        .expect_err("3 cells, 2 colors");

    assert_eq!(error.code(), "color_count_mismatch");
    assert!(ptreal(&rig.simulator).is_empty(), "nothing went out");
}
