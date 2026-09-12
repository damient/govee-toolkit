//! The precondition a command has: the device must be known to a mode.
//!
//! `Govee::ensure_known` is what pays for that, once, before the first
//! command — `docs/modes.md`. The fixture is [`ble_fake`].

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod ble_fake;

use std::sync::Arc;
use std::time::Duration;

use govee_toolkit::Mode;
use govee_toolkit::transport::Transport;
use tokio::time::Instant;

use self::ble_fake::{Fake, attach_enabling, enabling_ble, govee, id};

#[tokio::test]
async fn a_device_a_mode_already_knows_costs_no_scan() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let mode = govee
        .ensure_known(&id())
        .await
        .expect("the transport already holds it");

    assert_eq!(mode, Mode::Ble);
    assert!(ble.scanned().is_empty());
}

#[tokio::test]
async fn a_device_no_enabled_mode_finds_is_reported_before_any_command() {
    let ble = Fake::knowing_nothing();
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .ensure_known(&id())
        .await
        .expect_err("the scan hears nothing");

    assert_eq!(error.code(), "unknown_device");
}

/// The configuration's order is a preference, so the mode that answers first
/// does not take the place of the mode the user put first.
#[tokio::test(start_paused = true)]
async fn the_first_enabled_mode_wins_over_the_first_to_answer() {
    let slow = Duration::from_secs(2);
    let lan = Fake::finding(Mode::Lan, slow, slow);
    let ble = Fake::finding(Mode::Ble, Duration::from_secs(5), Duration::from_millis(10));
    let govee = attach_enabling(
        "lan, ble",
        &[
            Arc::clone(&lan) as Arc<dyn Transport>,
            Arc::clone(&ble) as Arc<dyn Transport>,
        ],
    );

    let start = Instant::now();
    let mode = govee.ensure_known(&id()).await.expect("both modes find it");

    assert_eq!(mode, Mode::Lan);
    // Both looked at the same time: the wait is the slower window, not the sum.
    assert_eq!(start.elapsed(), slow);
}

/// A mode that finds nothing hands the device to the next enabled one.
#[tokio::test(start_paused = true)]
async fn the_next_enabled_mode_answers_where_the_first_hears_nothing() {
    let lan = Fake::claiming(Mode::Lan, Duration::from_secs(1));
    let ble = Fake::finding(Mode::Ble, Duration::from_secs(5), Duration::from_millis(10));
    let govee = attach_enabling(
        "lan, ble",
        &[
            Arc::clone(&lan) as Arc<dyn Transport>,
            Arc::clone(&ble) as Arc<dyn Transport>,
        ],
    );

    let mode = govee.ensure_known(&id()).await.expect("`ble` finds it");

    assert_eq!(mode, Mode::Ble);
}
