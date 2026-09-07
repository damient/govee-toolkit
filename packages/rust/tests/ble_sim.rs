//! The `ble` transport, end to end against a simulated device.
//!
//! Everything here is the real send path: the facade, the transport, the
//! budget, the breaker and the link. Only the radio is fake, so these run in
//! CI, where there is none.
//!
//! The device is [`govee_toolkit_sim::ble`], and [`ble_wire`] is what joins it
//! to the transport.

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
use govee_toolkit::transport::DeviceId;
use govee_toolkit::{Args, Config, Govee, Mode, State};
use govee_toolkit_sim::ble::{BleAdapter, BleDevice, BleFaults, BleOptions, Stall};

use self::ble_fake::{MAC, SKU, catalog, enabling_ble};
use self::ble_wire::Radio;

/// The handle the fake radio addresses the device by. Not the identity: the
/// two are related by `bind`, as they are on hardware.
const ENDPOINT: &str = "11:22:33:44:55:66";

/// The power frame the fixture declares, at `on = 1`.
const POWER_ON: [u8; 20] = [
    0x33, 0x01, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x33,
];

fn id() -> DeviceId {
    DeviceId::new(MAC)
}

/// Timings short enough that a test does not wait on a window, and long enough
/// for a task to run. Verification is off: a test that wants a read asks for
/// one, so what reaches the device is what the test sent.
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
    let ble = Transport::with_adapter(options, Arc::new(Radio(adapter))).expect("the budget holds");

    let config: Config = serde_norway::from_str(&enabling_ble()).expect("the configuration parses");
    let govee = Govee::attach(config, catalog(), [Arc::new(ble.clone()) as Arc<_>])
        .expect("the configuration applies");

    govee.scan().await.expect("the scan runs");
    // An advertisement carries the Bluetooth handle, and this crate identifies
    // a device by its Wi-Fi MAC. Nothing infers one from the other.
    ble.bind(&id(), ENDPOINT).expect("the scan heard it");
    govee
}

fn device(faults: BleFaults) -> BleDevice {
    BleDevice::start(BleOptions {
        faults,
        ..BleOptions::new(ENDPOINT, SKU)
    })
}

#[tokio::test]
async fn the_frames_the_codec_built_reach_the_device() {
    let device = device(BleFaults::default());
    let govee = rig(&device, options()).await;

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(device.received(), vec![POWER_ON.to_vec()]);
}

#[tokio::test]
async fn a_status_request_reads_both_exchanges_back() {
    let device = device(BleFaults::default());
    device.set_read_answer(0x01, &[1]);
    device.set_read_answer(0x04, &[42]);
    let govee = rig(&device, options()).await;

    let status = govee.device(&id()).status().await.expect("it answers");

    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(42));
    assert_eq!(device.received_count(), 2);
}

#[tokio::test]
async fn a_silent_device_is_unreachable_and_the_breaker_records_it() {
    let device = device(BleFaults {
        silent: true,
        ..BleFaults::default()
    });
    device.set_read_answer(0x01, &[1]);
    let govee = rig(&device, options()).await;

    let error = govee
        .device(&id())
        .status()
        .await
        .expect_err("nothing answers");
    assert_eq!(error.code(), "unreachable");

    // The frames still went out: this wire acknowledges nothing, so silence is
    // all there is to go on.
    assert_eq!(device.received_count(), 1);
    let health = govee
        .device(&id())
        .health(Mode::Ble)
        .expect("the device is known");
    assert!(health.failures > 0);
}

#[tokio::test]
async fn a_device_of_another_family_carries_nothing_to_write_to() {
    let device = BleDevice::start(BleOptions {
        carries_service: false,
        ..BleOptions::new(ENDPOINT, SKU)
    });
    let govee = rig(&device, options()).await;

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("there is no characteristic to write to");

    assert_eq!(error.code(), "io");
    assert!(device.received().is_empty());
}

#[tokio::test]
async fn a_refused_connection_degrades_the_mode() {
    let device = device(BleFaults {
        refuse_connection: true,
        ..BleFaults::default()
    });
    let govee = rig(&device, options()).await;

    for _ in 0..8 {
        let _ = govee
            .device(&id())
            .send("power", &Args::new().int("on", 1))
            .await;
    }

    // Three refusals degrade the mode, and the cooldown then refuses the rest
    // without reaching for the radio at all.
    let health = govee
        .device(&id())
        .health(Mode::Ble)
        .expect("the device is known");
    assert_eq!(health.state, State::Degraded);
    assert!(!health.available);
    assert_eq!(health.failures, 3);
}

/// The budget exists for this: a firmware written to too fast stops answering
/// for seconds, and the transport must never take a device there.
#[tokio::test]
async fn the_write_budget_keeps_the_firmware_under_its_burst() {
    let device = device(BleFaults {
        stall: Some(Stall {
            after: 10,
            within: Duration::from_millis(100),
            lasts: Duration::from_secs(2),
        }),
        ..BleFaults::default()
    });
    let govee = rig(
        &device,
        Options {
            writes_per_second: 50.0,
            burst: 1,
            ..options()
        },
    )
    .await;

    for _ in 0..20 {
        govee
            .device(&id())
            .send("power", &Args::new().int("on", 1))
            .await
            .expect("the command goes out");
    }

    assert_eq!(device.received_count(), 20);
    assert_eq!(device.stalls(), 0);
}
