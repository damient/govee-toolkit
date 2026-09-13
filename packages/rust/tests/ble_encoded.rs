//! The `ble` transport against a device that encodes the link.
//!
//! Shares the fake radio and fixtures of [`ble_sim`] through the same two
//! modules, so these run in CI with no Bluetooth on the machine.

#![cfg(feature = "ble")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod ble_fake;
mod ble_wire;

use std::sync::Arc;
use std::time::Duration;

use govee_toolkit::ble::{Options, Transport};
use govee_toolkit::{Args, Config, Govee};
use govee_toolkit_sim::ble::{BleAdapter, BleDevice, BleFaults, BleOptions};

use self::ble_fake::{ENDPOINT, POWER_ON, SKU, catalog, enabling_ble, id};
use self::ble_wire::Radio;

/// Short enough not to wait on a window, long enough for a task to run.
fn options() -> Options {
    Options {
        scan_window: Duration::from_millis(20),
        rescan_window: Duration::from_millis(20),
        connect_timeout: Duration::from_millis(200),
        status_timeout: Duration::from_millis(200),
        verify_interval: None,
        ..Options::default()
    }
}

/// One device on the air, an SDK attached to it, and the scan already run.
async fn rig(device: &BleDevice, options: Options) -> Govee {
    let adapter = BleAdapter::holding([device.clone()]);
    let ble = Transport::with_adapter(options, Arc::new(Radio::new(adapter)), &catalog())
        .expect("the budget holds");
    let config: Config = serde_norway::from_str(&enabling_ble()).expect("the configuration parses");
    let govee = Govee::attach(config, catalog(), [Arc::new(ble.clone()) as Arc<_>])
        .expect("the configuration applies");
    govee.scan().await.expect("the scan runs");
    ble.bind(&id(), ENDPOINT).expect("the scan heard it");
    govee
}

/// A device that advertises the encoding flag. The link runs the handshake
/// and encodes every frame; the device records what the frames carried once
/// decoded, so the assertions below read plaintext on both ends.
fn encoded_device() -> BleDevice {
    BleDevice::start(BleOptions {
        encoded: true,
        ..BleOptions::new(ENDPOINT, SKU)
    })
}

#[tokio::test]
async fn an_encoded_device_gets_the_handshake_and_encoded_frames() {
    let device = encoded_device();
    let govee = rig(&device, options()).await;

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    // The handshake handed a seed out, and the one command frame arrived under
    // it: the device recorded the plaintext, so it decoded what it was sent.
    assert!(device.session_seed().is_some());
    assert_eq!(device.received(), vec![POWER_ON.to_vec()]);
}

#[tokio::test]
async fn an_encoded_device_answers_a_status_request_under_the_session_seed() {
    let device = encoded_device();
    device.set_read_answer(0x01, &[1]);
    device.set_read_answer(0x04, &[42]);
    let govee = rig(&device, options()).await;

    let status = govee.device(&id()).status().await.expect("it answers");

    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(42));
    assert_eq!(device.received_count(), 2);
}

#[tokio::test]
async fn an_encoded_device_that_does_not_answer_the_handshake_is_reported() {
    let device = BleDevice::start(BleOptions {
        encoded: true,
        faults: BleFaults {
            silent: true,
            ..BleFaults::default()
        },
        ..BleOptions::new(ENDPOINT, SKU)
    });
    let govee = rig(
        &device,
        Options {
            handshake_timeout: Duration::from_millis(50),
            ..options()
        },
    )
    .await;

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("the handshake gets no answer");

    assert_eq!(error.code(), "io");
    assert!(error.to_string().contains("encoded link"));
    // No command frame went out: the link never opened.
    assert!(device.received().is_empty());
    assert!(device.session_seed().is_none());
}
