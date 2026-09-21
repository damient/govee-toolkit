//! The channel table, checked against the whole catalog and against inline
//! device files.

#![allow(clippy::expect_used, clippy::panic)]

use govee_toolkit::codec::{Catalog, Device};

use super::reach::{SEGMENTS, has};
use super::{Channel, Component, Error, Missing, Personality, Profile, Slot, UNIVERSE};

fn catalog() -> Catalog {
    Catalog::embedded().expect("the embedded catalog parses")
}

/// The names of the tables `device` serves.
fn personalities(device: &Device) -> Vec<Personality> {
    super::served(device)
        .iter()
        .filter_map(|table| table.as_ref().ok().map(Profile::personality))
        .collect()
}

/// Offsets run 1, 2, 3 and so on to the width, with no hole and no channel
/// twice. An operator patches a desk off this, so a hole is a fixture that
/// answers nothing.
fn check_offsets(sku: &str, profile: &Profile) {
    let expected: Vec<u16> = (1..=profile.width()).collect();
    let offsets: Vec<u16> = profile.channels().iter().map(|c| c.offset).collect();
    assert_eq!(offsets, expected, "{sku} `{}`", profile.personality());
}

/// A device whose zones are its addressable LEDs serves `pixel` alone, so it
/// is the one personality every segmented device has to answer.
#[test]
fn every_segmented_device_answers_a_zone_personality() {
    for device in catalog().devices() {
        if !has(device, SEGMENTS) {
            continue;
        }
        let zones = device
            .capabilities
            .segment_count()
            .expect("a device that reaches segments counts them");
        let personality = if device.capabilities.native_pixels() == Some(zones) {
            Personality::Pixel
        } else {
            Personality::Segment
        };
        let profile =
            Profile::of(device, personality).unwrap_or_else(|e| panic!("{}: {e}", device.sku));
        assert_eq!(profile.width(), u16::try_from(2 + 3 * zones).unwrap_or(0));
        check_offsets(&device.sku, &profile);
    }
}

/// Offset 1 and offset 2 mean the same thing on every personality and every
/// model, so a cue file carries between them.
#[test]
fn the_second_channel_of_every_personality_is_the_mode() {
    for device in catalog().devices() {
        for table in super::served(device) {
            let profile = table.expect("a served personality");
            let second = profile.channels().get(1).expect("a second channel");
            assert_eq!(second.slot, Slot::Mode, "{}", device.sku);
            assert_eq!(second.offset, 2, "{}", device.sku);
        }
    }
}

#[test]
fn no_personality_leaves_a_hole_or_takes_a_channel_twice() {
    for device in catalog().devices() {
        for table in super::served(device) {
            let profile = table.unwrap_or_else(|e| panic!("{}: {e}", device.sku));
            check_offsets(&device.sku, &profile);
            assert!(profile.width() <= UNIVERSE);
        }
    }
}

#[test]
fn the_first_channel_of_every_personality_is_the_dimmer() {
    for device in catalog().devices() {
        for table in super::served(device) {
            let profile = table.expect("a served personality");
            let first = profile.channels().first().expect("a channel");
            assert_eq!(first.slot, Slot::Dimmer, "{}", device.sku);
            assert!(first.scale.is_some(), "{}", device.sku);
        }
    }
}

/// `segment` lays the zones out in zone order, one triple each, so the
/// operator reads channel 3 as the red of zone 0.
#[test]
fn a_segment_personality_lays_one_triple_on_each_zone() {
    let catalog = catalog();
    let device = catalog.device("H61A0").expect("the SKU resolves");
    let profile = Profile::of(device, Personality::Segment).expect("a segment personality");
    assert_eq!(
        profile.channels().get(2..5),
        Some(
            [
                Channel::plain(
                    3,
                    Slot::Zone {
                        index: 0,
                        component: Component::Red
                    }
                ),
                Channel::plain(
                    4,
                    Slot::Zone {
                        index: 0,
                        component: Component::Green
                    }
                ),
                Channel::plain(
                    5,
                    Slot::Zone {
                        index: 0,
                        component: Component::Blue
                    }
                ),
            ]
            .as_slice()
        )
    );
}

fn device_file(capabilities: &str, lan: &str) -> String {
    format!(
        "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
         capabilities:\n{capabilities}\
         modes:\n  lan:\n    support: capped\n    capabilities: [{lan}]\n\
         commands:\n  lan:\n\
         \x20   power:\n      cmd: turn\n      role: power\n      args:\n\
         \x20       on: {{ type: int, range: [0, 1], role: \"on\" }}\n\
         \x20   brightness:\n      cmd: brightness\n      role: brightness\n      args:\n\
         \x20       level: {{ type: int, range: brightness.range, role: brightness }}\n\
         \x20   color:\n      cmd: colorwc\n      role: color\n      args:\n\
         \x20       r: {{ type: int, range: [0, 255], role: red }}\n\
         \x20       g: {{ type: int, range: [0, 255], role: green }}\n\
         \x20       b: {{ type: int, range: [0, 255], role: blue }}\n\
         \x20   paint:\n      cmd: razer\n      role: segment_color\n      args:\n\
         \x20       colors: {{ type: rgb_list, role: colors }}\n"
    )
}

fn parse(catalog: &Catalog) -> &Device {
    catalog.device("HTEST").expect("the SKU resolves")
}

const RGB: &str = "  power:\n  brightness:  { range: [1, 100] }\n  color:\n";

fn built(capabilities: &str, lan: &str) -> Catalog {
    Catalog::from_sources([("HTEST.yaml", device_file(capabilities, lan).as_str())])
        .expect("the device file parses")
}

/// The white channel holds its place where `lan` reaches no white
/// temperature, so `full` takes 6 channels on every device. It drives
/// nothing there: a scale is what a channel needs to send a value.
#[test]
fn a_device_that_reaches_no_white_keeps_the_channel_and_drives_nothing() {
    let catalog = built(RGB, "power, brightness, color");
    let device = parse(&catalog);
    assert_eq!(personalities(device), vec![Personality::Full]);
    let profile = Profile::of(device, Personality::Full).expect("a full personality");
    assert_eq!(profile.width(), 6);
    let white = profile.channels().get(5).expect("the white channel");
    assert_eq!(white.slot, Slot::WhiteTemp);
    assert_eq!(white.scale, None);
}

/// Every color component travels the whole pair its `lan` command declares,
/// so a device that takes less than a byte still takes the full fader.
#[test]
fn a_color_component_scales_into_the_pair_the_command_declares() {
    let catalog = built(RGB, "power, brightness, color");
    let device = parse(&catalog);
    let profile = Profile::of(device, Personality::Full).expect("a full personality");
    let red = profile.channels().get(2).expect("the red channel");
    assert_eq!(red.slot, Slot::Color(Component::Red));
    let scale = red.scale.expect("a scale");
    assert_eq!(scale.range(), [0, 255]);
    assert_eq!(scale.byte(0), 0);
    assert_eq!(scale.byte(255), 255);
}

/// A capability the hardware has and the mode does not reach drives no
/// channel: the bridge fails rather than approximate it over another mode.
#[test]
fn a_capability_out_of_the_mode_s_reach_serves_no_channel() {
    let capabilities = format!("{RGB}  segments:\n    count: 10\n");
    let catalog = built(&capabilities, "power, brightness, color");
    let device = parse(&catalog);
    assert_eq!(personalities(device), vec![Personality::Full]);
}

#[test]
fn a_zone_count_over_one_universe_is_an_error() {
    let capabilities = format!("{RGB}  segments:\n    count: 200\n");
    let catalog = built(&capabilities, "power, brightness, color, segments");
    let device = parse(&catalog);
    assert_eq!(
        Profile::of(device, Personality::Segment),
        Err(Error::TooWide {
            sku: "HTEST".to_owned(),
            personality: Personality::Segment,
            channels: 602,
        })
    );
}

/// An unmeasured native resolution is never extrapolated from the zone count.
#[test]
fn a_device_with_no_measured_pixels_serves_no_native_personality() {
    let capabilities = format!("{RGB}  segments:\n    count: 10\n");
    let catalog = built(&capabilities, "power, brightness, color, segments");
    let device = parse(&catalog);
    assert_eq!(
        personalities(device),
        vec![Personality::Full, Personality::Segment]
    );
    assert_eq!(
        Profile::of(device, Personality::Pixel),
        Err(Error::Unserved {
            sku: "HTEST".to_owned(),
            personality: Personality::Pixel,
            missing: Missing::NativePixels,
        })
    );
}

/// A device whose every zone is one addressable LED lays out one table, and
/// one table carries one name. `segment` would repeat `pixel`.
#[test]
fn one_pixel_per_zone_serves_the_pixel_personality_alone() {
    let capabilities = format!("{RGB}  segments:\n    count: 10\n    native_pixels: 10\n");
    let catalog = built(&capabilities, "power, brightness, color, segments");
    let device = parse(&catalog);
    assert_eq!(
        personalities(device),
        vec![Personality::Full, Personality::Pixel]
    );
    assert_eq!(
        Profile::of(device, Personality::Segment),
        Err(Error::Unserved {
            sku: "HTEST".to_owned(),
            personality: Personality::Segment,
            missing: Missing::OnePixelPerZone,
        })
    );
}
