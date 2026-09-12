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

use std::sync::Arc;
use std::time::Duration;

use govee_toolkit::transport::Transport;
use govee_toolkit::{Args, Mode};
use tokio::time::Instant;

use self::ble_fake::{Fake, MAC, POWER_ON, SCAN_WINDOW, SKU, attach, enabling_ble, govee, id};
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

/// A window is time on the wire, so two modes spend theirs at the same time.
#[tokio::test(start_paused = true)]
async fn a_scan_spends_the_windows_at_the_same_time() {
    let long = Duration::from_secs(9);
    let ble = Fake::claiming(Mode::Ble, SCAN_WINDOW);
    let lan = Fake::claiming(Mode::Lan, long);
    let govee = attach(&[
        Arc::clone(&ble) as Arc<dyn Transport>,
        Arc::clone(&lan) as Arc<dyn Transport>,
    ]);

    let start = Instant::now();
    govee.scan().await.expect("the scan runs");

    assert_eq!(start.elapsed(), long);
    assert_eq!(ble.scanned(), vec![SCAN_WINDOW]);
    assert_eq!(lan.scanned(), vec![long]);
}

/// The wake frame the fixture declares, at `on = 1` and `on = 0`.
const LINK_ON: [u8; 20] = [
    0x33, 0x17, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x25,
];
const LINK_OFF: [u8; 20] = [
    0x33, 0x17, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x24,
];
const API_READ: [u8; 20] = [
    0xaa, 0xab, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01,
];

fn credentials() -> govee_toolkit::WifiCredentials {
    govee_toolkit::WifiCredentials {
        network: "Net".to_owned(),
        password: "pw".to_owned(),
        utc_offset_hours: 1,
        utc_offset_minutes: 30,
    }
}

// `start_paused` runs the wake delay off the clock rather than waiting it out.
#[tokio::test(start_paused = true)]
async fn provisioning_wakes_the_module_transfers_and_releases_it_in_that_order() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect("every write goes out");

    let written = ble.written();
    // The endpoint is read first: which transfer entry to send depends on it.
    assert_eq!(written.first().unwrap(), &API_READ.to_vec());
    assert_eq!(written.get(1).unwrap(), &LINK_ON.to_vec());
    assert_eq!(written.last().unwrap(), &LINK_OFF.to_vec());

    // Everything between the two link frames is the transfer, and nothing else.
    let transfer = &written[2..written.len() - 1];
    assert!(
        transfer.len() >= 3,
        "a header, a body and a footer at least"
    );
    assert!(transfer.iter().all(|frame| frame[0..2] == [0xa1, 0x11]));
}

#[tokio::test(start_paused = true)]
async fn provisioning_sends_the_endpoint_the_device_asked_for() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect("every write goes out");

    // Type 2 selects an endpoint, so the entry carrying one is what went out.
    // Its bytes ride the data frames: byte 2 numbers them, and the header and
    // the footer take 0x00 and 0xff.
    let body: Vec<u8> = ble
        .written()
        .iter()
        .filter(|frame| frame[0..2] == [0xa1, 0x11] && !matches!(frame[2], 0x00 | 0xff))
        .flat_map(|frame| frame[3..19].to_vec())
        .collect();
    let text = String::from_utf8_lossy(&body).into_owned();
    assert!(text.contains("https://device.govee.com"), "got {text:?}");
    assert!(text.contains("Net"));
}

#[tokio::test(start_paused = true)]
async fn provisioning_refuses_a_device_that_does_not_enable_ble() {
    let ble = Fake::knowing(&id());
    let lan_only =
        format!("defaults:\n  modes: [lan]\ndevices:\n  \"{MAC}\":\n    sku: \"{SKU}\"\n");
    let govee = govee(&ble, &lan_only);

    let error = govee
        .device(&id())
        .provision_wifi(&credentials())
        .await
        .expect_err("ble is not enabled for it");

    assert_eq!(error.code(), "no_mode_available");
    assert!(ble.written().is_empty(), "nothing may reach the radio");
}

/// The `role: color_temp` entry of the fixture, at 4000 K.
///
/// The kelvin value, the rendering the SDK computes for it, and a mask naming
/// the six zones the fixture's zone argument bounds — not the four
/// `capabilities.segments.count` exposes. A mask that stopped at the count
/// would leave the zones past it holding the color they had.
const WHITE_4000K: [u8; 20] = [
    0x33, 0x05, 0x15, 0x01, 0, 0, 0, 0x0f, 0xa0, 0xff, 0xce, 0xa6, 0x3f, 0, 0, 0, 0, 0, 0, 0x25,
];

#[tokio::test]
async fn a_white_temperature_carries_its_rendering_and_every_zone_the_mask_names() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .color_temp(4000)
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![WHITE_4000K.to_vec()]);
}

#[tokio::test]
async fn a_white_temperature_out_of_range_is_refused_rather_than_clamped() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .color_temp(1200)
        .await
        .expect_err("1200 K is outside the declared range");

    assert_eq!(error.code(), "out_of_range");
    assert!(ble.written().is_empty());
}

/// The `role: segment_color_masked` entry of the fixture, painting red over
/// every zone its mask names — six, against the four
/// `capabilities.segments.count` exposes.
const PAINT_RED: [u8; 20] = [
    0x33, 0x05, 0x15, 0x01, 0xff, 0, 0, 0, 0, 0, 0, 0, 0x3f, 0, 0, 0, 0, 0, 0, 0xe2,
];

#[tokio::test]
async fn painting_every_zone_covers_what_the_mask_reaches() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .segment(None, [255, 0, 0], false)
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![PAINT_RED.to_vec()]);
}
