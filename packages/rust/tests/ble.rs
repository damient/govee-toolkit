//! Dispatch to a mode that is not `lan`, through a transport that is not a
//! radio: the transport claiming the enabled mode serves the device, the
//! codec's frames reach it unwrapped, fire-and-verify follows, and a scan uses
//! that transport's own window. The fixture is [`ble_fake`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod ble_fake;

use govee_toolkit::{Args, Mode};

use self::ble_fake::{Fake, MAC, POWER_ON, SCAN_WINDOW, SKU, enabling_ble, govee, id};
/// The two frames of the entry the fixture marks `role: status`.
const POWER_READ: [u8; 20] = [
    0xaa, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab,
];
const BRIGHTNESS_READ: [u8; 20] = [
    0xaa, 0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xae,
];

#[tokio::test]
async fn a_command_is_served_by_the_transport_claiming_the_enabled_mode() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let served = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(served.mode, Mode::Ble);
    assert_eq!(served.command, "power");
    // Nothing on this wire carries a name: the envelope belongs to `lan`.
    assert_eq!(served.cmd, "");
}

#[tokio::test]
async fn what_reaches_the_transport_is_the_frames_and_nothing_around_them() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![POWER_ON.to_vec()]);
}

#[tokio::test]
async fn a_command_carries_the_verification_the_device_file_names() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(
        ble.verified(),
        vec![POWER_READ.to_vec(), BRIGHTNESS_READ.to_vec()]
    );
}

#[tokio::test]
async fn a_status_request_is_the_entry_marked_with_the_role() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let status = govee.device(&id()).status().await.expect("it answers");

    // One entry, two exchanges: the file names both, and neither name is here.
    assert_eq!(
        ble.written(),
        vec![POWER_READ.to_vec(), BRIGHTNESS_READ.to_vec()]
    );
    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(100));
}

#[tokio::test]
async fn a_read_returns_the_fields_the_device_file_names() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let reply = govee
        .device(&id())
        .read("software", &Args::new())
        .await
        .expect("it answers");

    assert_eq!(
        reply.fields.get("version"),
        Some(&govee_toolkit::codec::ArgValue::Text("2.06.02".to_owned()))
    );
}

#[tokio::test]
async fn a_command_that_declares_no_reply_has_nothing_to_read() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .read("power", &Args::new().int("on", 1))
        .await
        .expect_err("the fixture declares no reply for it");

    assert_eq!(error.code(), "no_reply_layout");
}

#[tokio::test]
async fn an_argument_out_of_range_is_refused_before_anything_is_written() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 2))
        .await
        .expect_err("2 is outside the declared range");

    assert_eq!(error.code(), "out_of_range");
    assert!(ble.written().is_empty());
}

#[tokio::test]
async fn a_device_the_transport_has_not_heard_of_is_not_reached_another_way() {
    let ble = Fake::knowing_nothing();
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("nothing knows this device");

    assert_eq!(error.code(), "unknown_device");
    assert!(ble.written().is_empty());
}

#[tokio::test]
async fn a_mode_no_transport_serves_says_so_rather_than_choosing_another() {
    let ble = Fake::knowing(&id());
    let govee = govee(
        &ble,
        &format!("defaults:\n  modes: [lan]\ndevices:\n  \"{MAC}\":\n    sku: \"{SKU}\"\n"),
    );

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("this build attached no lan transport");

    assert_eq!(error.code(), "mode_not_implemented");
    assert!(ble.written().is_empty());
}

#[tokio::test]
async fn a_scan_listens_for_each_transport_s_own_window() {
    let ble = Fake::knowing(&id());
    // A `lan` window long enough that a scan carrying it to another mode is
    // unmistakable.
    let govee = govee(
        &ble,
        &format!("{}lan:\n  scan_window_ms: 9000\n", enabling_ble()),
    );

    govee.scan().await.expect("the scan runs");

    assert_eq!(ble.scanned(), vec![SCAN_WINDOW]);
}
