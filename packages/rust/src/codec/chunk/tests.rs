//! Cutting a body into frames: the two dialects, and what each frame carries.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::format_collect)]

use super::*;
use crate::codec::hex;

fn wifi() -> Layout {
    Layout::parse(
        "provision",
        "${ssid:str8} ${password:str8} ${run_mode} ${tz_hours} ${iot_version} ${tz_minutes}",
        &Chunk {
            size: 16,
            header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>".to_owned(),
            data: "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
            footer: "A1 <op:11> FF <pad:20> <xor>".to_owned(),
            ..Chunk::default()
        },
    )
    .expect("the layouts parse")
}

/// SSID `Test`, password `abc`, UTC+2, no API block.
#[test]
fn the_worked_provisioning_example_matches_byte_for_byte() {
    let args = Args::new()
        .text("ssid", "Test")
        .text("password", "abc")
        .int("run_mode", 0)
        .int("tz_hours", 2)
        .int("iot_version", 0)
        .int("tz_minutes", 0);
    let (frames, _) = wifi().build("provision", &args).unwrap();

    assert_eq!(
        frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
        [
            "a1110001000000000000000000000000000000b1",
            "a1110104546573740361626300020000000000e2",
            "a111ff000000000000000000000000000000004f",
        ]
    );
    assert!(frames.iter().all(|f| f.len() == 20));
}

#[test]
fn a_body_longer_than_one_slice_takes_one_data_frame_each() {
    let args = Args::new()
        .text("ssid", "0123456789abcdef")
        .text("password", "0123456789")
        .int("run_mode", 0)
        .int("tz_hours", 0)
        .int("iot_version", 0)
        .int("tz_minutes", 0);
    let (frames, _) = wifi().build("provision", &args).unwrap();

    // 1 + 16 + 1 + 10 + 4 = 32 body bytes, so two data frames.
    assert_eq!(frames.len(), 4);
    assert_eq!(frames.get(1).map(|f| f.get(2).copied()), Some(Some(1)));
    assert_eq!(frames.get(2).map(|f| f.get(2).copied()), Some(Some(2)));
    assert_eq!(frames.first().map(|f| f.get(3).copied()), Some(Some(2)));
}

#[test]
fn a_slice_size_of_zero_would_never_terminate() {
    let err = Layout::parse("x", "${b:bytes}", &Chunk::default()).expect_err("no size");
    assert_eq!(err.code(), "chunk_syntax");
}

/// The dialect of `docs/protocol/ble.md` 6: the header carries the first
/// thirteen bytes, the closing frame carries the last piece, and the count
/// in the header is every frame of the transfer.
fn music() -> Layout {
    Layout::parse(
        "music",
        "${colors_count} (${colors}:rgb)×${colors_count} ${tail:bytes}",
        &Chunk {
            size: 17,
            head_size: 13,
            header: "A3 00 01 ${total} 41 ${effect} ${chunk:bytes} <pad:20> <xor>".to_owned(),
            data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
            footer: "A3 FF ${chunk:bytes} <pad:20> <xor>".to_owned(),
            then: Some("33 05 13 ${effect} ${sensitivity} <pad:20> <xor>".to_owned()),
            ..Chunk::default()
        },
    )
    .expect("the layouts parse")
}

/// The seven colours the phone controller sends with no saved palette.
fn palette() -> Args {
    Args::new().rgb(
        "colors",
        [
            [255, 0, 0],
            [255, 127, 0],
            [255, 255, 0],
            [0, 255, 0],
            [0, 0, 255],
            [0, 255, 255],
            [139, 0, 255],
        ],
    )
}

/// Effect 50, sent to an H61A0 and rendered by it. A 25-byte body fits in
/// the header and the closing frame, so no data frame goes out.
#[test]
fn a_body_that_fits_the_header_and_the_footer_sends_two_frames_and_a_then() {
    let args = palette()
        .int("colors_count", 7)
        .bytes("tail", [3, 0, 99])
        .int("effect", 50)
        .int("sensitivity", 99);
    let (frames, _) = music().build("music", &args).unwrap();

    assert_eq!(
        frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
        [
            "a3000102413207ff0000ff7f00ffff0000ff0054",
            "a3ff0000ff00ffff8b00ff0300630000000000b7",
            "3305133263000000000000000000000000000074",
        ]
    );
    assert!(frames.iter().all(|f| f.len() == 20));
}

/// Effect 51, the same way. A 31-byte body leaves 18 bytes after the
/// header: one full data frame, and one byte in the closing frame.
#[test]
fn a_body_past_one_piece_puts_the_last_piece_in_the_closing_frame() {
    let args = palette()
        .int("colors_count", 7)
        .bytes("tail", [1, 1, 1, 25, 98, 1, 3, 6, 17])
        .int("effect", 51)
        .int("sensitivity", 99);
    let (frames, _) = music().build("music", &args).unwrap();

    assert_eq!(
        frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
        [
            "a3000103413307ff0000ff7f00ffff0000ff0054",
            "a3010000ff00ffff8b00ff010101196201030657",
            "a3ff11000000000000000000000000000000004d",
            "3305133363000000000000000000000000000075",
        ]
    );
}

#[test]
fn a_header_that_carries_body_bytes_has_to_say_how_many() {
    let err = Layout::parse(
        "music",
        "${tail:bytes}",
        &Chunk {
            size: 17,
            header: "A3 00 ${chunk:bytes} <pad:20> <xor>".to_owned(),
            data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
            footer: "A3 FF <pad:20> <xor>".to_owned(),
            ..Chunk::default()
        },
    )
    .expect_err("head_size is missing");
    assert_eq!(err.code(), "chunk_syntax");
}

#[test]
fn a_head_size_with_no_slice_in_the_header_is_refused() {
    let err = Layout::parse(
        "music",
        "${tail:bytes}",
        &Chunk {
            size: 17,
            head_size: 13,
            header: "A3 00 <pad:20> <xor>".to_owned(),
            data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
            footer: "A3 FF <pad:20> <xor>".to_owned(),
            ..Chunk::default()
        },
    )
    .expect_err("the header takes no slice");
    assert_eq!(err.code(), "chunk_syntax");
}
