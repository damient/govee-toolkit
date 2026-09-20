//! What a command sent while the segment channel is armed puts on the wire.
//!
//! A unit can answer no status from the arming frame to the disarm
//! (`docs/protocol/lan.md` 2.3), so the SDK sends no verification there. The
//! simulator answers every request, which is why these tests count the
//! datagrams rather than read a health.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use govee_toolkit::stream::{Rate, Resolution, StreamOptions};
use govee_toolkit::{Args, Catalog, Config};
use govee_toolkit_sim::Simulator;

mod common;

use common::{Rig, SKU, id, wait_for};

const TEST_HZ: f64 = 20.0;

async fn rig() -> Rig {
    Rig::start(
        Config::default(),
        Catalog::embedded().expect("catalog"),
        SKU,
    )
    .await
}

fn options() -> StreamOptions {
    StreamOptions {
        resolution: Resolution::App,
        rate: Rate::Fixed(TEST_HZ),
        gradient: false,
    }
}

fn status_requests(simulator: &Simulator) -> usize {
    simulator
        .received()
        .iter()
        .filter(|received| received.is_status_request())
        .count()
}

async fn brightness(rig: &Rig) {
    rig.govee
        .device(&id())
        .send("brightness", &Args::new().int("level", 50))
        .await
        .expect("brightness reaches the device");
}

#[tokio::test]
async fn a_command_sent_while_the_channel_is_armed_asks_for_no_status() {
    let rig = rig().await;
    let stream = rig
        .govee
        .device(&id())
        .open_stream(options())
        .await
        .expect("the stream opens");
    rig.simulator.clear();

    brightness(&rig).await;

    // The command itself goes out; only the verification is skipped.
    wait_for(|| (!rig.simulator.received().is_empty()).then_some(())).await;
    assert_eq!(status_requests(&rig.simulator), 0);
    drop(stream);
}

#[tokio::test]
async fn a_command_sent_after_the_disarm_is_verified_again() {
    let rig = rig().await;
    let stream = rig
        .govee
        .device(&id())
        .open_stream(options())
        .await
        .expect("the stream opens");
    stream.close().await.expect("the disarm goes out");
    rig.simulator.clear();

    brightness(&rig).await;

    assert_eq!(
        wait_for(|| (status_requests(&rig.simulator) > 0).then_some(())).await,
        Some(())
    );
}
