//! Wi-Fi provisioning against a simulated device.
//!
//! Shares the fake radio and fixtures of [`ble_sim`] through the same two
//! modules, so these run in CI with no Bluetooth on the machine.

#![cfg(feature = "ble")]
#![allow(clippy::expect_used)]

mod ble_fake;
mod ble_wire;

use std::sync::Arc;
use std::time::Duration;

use govee_toolkit::ble::{Options, Transport};
use govee_toolkit::{Config, Govee, Provisioned, WifiCredentials};
use govee_toolkit_sim::ble::{BleAdapter, BleDevice, BleFaults, BleOptions};

use self::ble_fake::{ENDPOINT, SKU, catalog, enabling_ble, id};
use self::ble_wire::Radio;

/// The `aa ab` read the transfer runs first. Type 0 names no endpoint, so the
/// transfer that carries none is the one that goes out.
const API_TYPE: u8 = 0xab;

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
async fn rig(device: &BleDevice) -> Govee {
    let adapter = BleAdapter::holding([device.clone()]);
    let ble = Transport::with_adapter(options(), Arc::new(Radio::new(adapter)), &catalog())
        .expect("the budget holds");

    let config: Config = serde_norway::from_str(&enabling_ble()).expect("the configuration parses");
    let govee = Govee::attach(config, catalog(), [Arc::new(ble.clone()) as Arc<_>])
        .expect("the configuration applies");

    govee.scan().await.expect("the scan runs");
    ble.bind(&id(), ENDPOINT).expect("the scan heard it");
    govee
}

fn device() -> BleDevice {
    let device = BleDevice::start(BleOptions {
        faults: BleFaults::default(),
        ..BleOptions::new(ENDPOINT, SKU)
    });
    device.set_read_answer(API_TYPE, &[0, 0]);
    device
}

fn credentials() -> WifiCredentials {
    WifiCredentials {
        network: "Net".to_owned(),
        password: "pw".to_owned(),
        utc_offset_hours: 1,
        utc_offset_minutes: 30,
    }
}

// `start_paused` runs the wake delay off the clock rather than waiting it out.
#[tokio::test(start_paused = true)]
async fn a_transfer_the_device_acknowledges_is_reported_as_accepted() {
    let device = device();
    let govee = rig(&device).await;

    let outcome = govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect("the device acknowledges the transfer");

    assert_eq!(outcome, Provisioned::Accepted);
}

#[tokio::test(start_paused = true)]
async fn a_transfer_the_device_refuses_fails_and_says_what_it_answered() {
    let device = device();
    device.set_transfer_status(3);
    let govee = rig(&device).await;

    let error = govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect_err("the device refuses the transfer");

    assert_eq!(error.code(), "provision_refused");
    assert!(error.to_string().contains('3'), "got {error}");
}

/// Every frame of the transfer goes out, and one answer covers them all.
#[tokio::test(start_paused = true)]
async fn every_frame_of_the_transfer_reaches_the_device() {
    let device = device();
    let govee = rig(&device).await;

    govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect("the device acknowledges the transfer");

    let transfer: Vec<Vec<u8>> = device
        .received()
        .into_iter()
        .filter(|frame| frame.starts_with(&[0xa1, 0x11]))
        .collect();
    assert!(
        transfer.len() >= 3,
        "a header, a data frame and a footer at least, got {transfer:?}"
    );
    assert_eq!(transfer.last().and_then(|frame| frame.get(2)), Some(&0xff));
}
