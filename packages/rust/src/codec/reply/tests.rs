//! Reading a reply: what each field shape captures, and what it refuses.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn read(layout: &str, bytes: &[u8]) -> Result<Captured> {
    Layout::parse("x", layout)?.read("x", bytes)
}

#[test]
fn a_literal_the_reply_does_not_carry_refuses_the_whole_frame() {
    let error = read("AA 04 ${level}", &[0xaa, 0x01, 0x64]).expect_err("byte 1 is not 04");
    assert_eq!(error.code(), "reply_mismatch");
}

#[test]
fn one_byte_is_captured_as_an_integer() {
    let captured = read("AA 04 ${level}", &[0xaa, 0x04, 0x64]).unwrap();
    assert_eq!(captured.get("level"), Some(&ArgValue::Int(100)));
    assert_eq!(captured.len(), 1);
}

#[test]
fn two_bytes_are_captured_big_endian() {
    let captured = read("AA 40 ${count:16}", &[0xaa, 0x40, 0x00, 0x2a]).unwrap();
    assert_eq!(captured.get("count"), Some(&ArgValue::Int(42)));
}

/// A version answers as text, not as binary version bytes.
#[test]
fn trailing_text_is_captured_without_its_padding() {
    let mut frame = vec![0xaa, 0x21];
    frame.extend_from_slice(b"2.06.02");
    frame.resize(20, 0);
    let captured = read("AA 21 ${version:ascii}", &frame).unwrap();
    assert_eq!(
        captured.get("version"),
        Some(&ArgValue::Text("2.06.02".to_owned()))
    );
}

#[test]
fn a_fixed_run_of_bytes_is_captured_as_it_is() {
    let frame = [0xaa, 0x14, 1, 2, 3, 4, 5, 6, 0, 0];
    let captured = read("AA 14 ${mac:bytes:6}", &frame).unwrap();
    assert_eq!(
        captured.get("mac"),
        Some(&ArgValue::Bytes(vec![1, 2, 3, 4, 5, 6]))
    );
}

#[test]
fn a_binary_answer_of_low_bytes_is_not_text() {
    let error = read("AA 21 ${version:ascii}", &[0xaa, 0x21, 0x01, 0x02, 0x06])
        .expect_err("control bytes are not a version string");
    assert_eq!(error.code(), "reply_mismatch");
}

#[test]
fn a_reply_that_stops_before_its_text_is_refused() {
    let error = read("AA 21 ${version:ascii}", &[0xaa, 0x21]).expect_err("nothing left to read");
    assert_eq!(error.code(), "reply_mismatch");
}

#[test]
fn a_reply_of_padding_alone_is_refused() {
    let mut frame = vec![0xaa, 0x21];
    frame.resize(20, 0);
    let error = read("AA 21 ${version:ascii}", &frame).expect_err("padding is not text");
    assert_eq!(error.code(), "reply_mismatch");
}

#[test]
fn a_reply_too_short_for_the_layout_is_refused() {
    let error = read("AA 40 ${count:16}", &[0xaa, 0x40, 0x00]).expect_err("one byte short");
    assert_eq!(error.code(), "reply_mismatch");
}

#[test]
fn bytes_past_the_layout_are_ignored() {
    let captured = read("AA 01 ${on}", &[0xaa, 0x01, 0x01, 0, 0, 0]).unwrap();
    assert_eq!(captured.get("on"), Some(&ArgValue::Int(1)));
}

#[test]
fn a_layout_that_builds_bytes_is_not_a_reply() {
    for source in [
        "AA 01 ${on} <xor>",
        "AA <len:16> <op:01>",
        "(${c}:rgb)×${n}",
    ] {
        let error = Layout::parse("x", source).expect_err("capture-only");
        assert_eq!(error.code(), "reply_syntax");
    }
}

/// The wire this reads ends every frame on a checksum, which is neither
/// padding nor text: the length is what leaves it out.
#[test]
fn text_of_a_given_length_stops_before_what_follows_it() {
    let mut frame = vec![0xaa, 0x21];
    frame.extend_from_slice(b"2.06.02");
    frame.resize(19, 0);
    frame.push(0xbd);
    let captured = read("AA 21 ${version:ascii:17}", &frame).unwrap();
    assert_eq!(
        captured.get("version"),
        Some(&ArgValue::Text("2.06.02".to_owned()))
    );
    read("AA 21 ${version:ascii}", &frame).expect_err("the checksum is not text");
}

#[test]
fn text_of_a_given_length_may_be_followed_by_more_of_the_layout() {
    let captured = read(
        "AA 21 ${version:ascii:3} ${on}",
        &[0xaa, 0x21, b'1', b'.', b'0', 1],
    )
    .unwrap();
    assert_eq!(
        captured.get("version"),
        Some(&ArgValue::Text("1.0".to_owned()))
    );
    assert_eq!(captured.get("on"), Some(&ArgValue::Int(1)));
}

#[test]
fn text_that_reads_to_the_end_has_to_be_last() {
    let error = Layout::parse("x", "AA ${v:ascii} ${on}").expect_err("nothing follows it");
    assert_eq!(error.code(), "reply_syntax");
}

#[test]
fn one_name_is_captured_once() {
    let error = Layout::parse("x", "AA ${on} ${on}").expect_err("captured twice");
    assert_eq!(error.code(), "reply_syntax");
}

#[test]
fn rejects_an_unknown_field_shape() {
    let error = Layout::parse("x", "AA ${v:str8}").expect_err("not a capture shape");
    assert_eq!(error.code(), "reply_syntax");
}
