//! The `role: music` verb over a transport that is not a radio: what the
//! fixture's entry marks is what goes out, and nothing else. The fixture is
//! [`ble_fake`].

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod ble_fake;

use govee_toolkit::Music;

use self::ble_fake::{Fake, enabling_ble, govee, id};

/// The fixture's `role: music` entry, effect 3 at sensitivity 60, with the
/// colors left to the firmware.
const EFFECT_3: [u8; 20] = [
    0x33, 0x05, 0x13, 0x03, 0x3c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x1a,
];

/// The same entry, effect 1 in fades, on a red the caller imposes.
const EFFECT_1_RED: [u8; 20] = [
    0x33, 0x05, 0x13, 0x01, 0, 0x01, 0x01, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xdb,
];

fn playing(effect: i64, sensitivity: i64) -> Music {
    Music {
        effect,
        sensitivity,
        soft: false,
        color: None,
    }
}

#[tokio::test]
async fn an_effect_carries_the_sensitivity_and_leaves_the_colors_to_the_firmware() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .music(&playing(3, 60))
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![EFFECT_3.to_vec()]);
}

#[tokio::test]
async fn a_color_the_caller_imposes_sets_the_switch_beside_it() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());
    let music = Music {
        soft: true,
        color: Some([255, 0, 0]),
        ..playing(1, 0)
    };

    govee
        .device(&id())
        .music(&music)
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![EFFECT_1_RED.to_vec()]);
}

#[tokio::test]
async fn an_effect_outside_the_declared_range_is_refused_rather_than_clamped() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .music(&playing(8, 0))
        .await
        .expect_err("8 is outside the declared range");

    assert_eq!(error.code(), "out_of_range");
    assert!(ble.written().is_empty());
}
