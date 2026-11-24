#![allow(clippy::expect_used)]

use super::*;

#[test]
fn ascii_x_is_accepted_for_the_multiplication_sign() {
    let a = Frame::parse("seg", "(${c}:rgb)×${n}");
    let b = Frame::parse("seg", "(${c}:rgb)x${n}");
    assert_eq!(a.ok(), b.ok());
}

#[test]
fn rejects_a_checksum_that_is_not_last() {
    let err = Frame::parse("x", "BB <xor> 01").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_the_two_checksums_together() {
    let err = Frame::parse("x", "BB <xor> <sum>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_a_length_field_with_no_opcode_after_it() {
    let err = Frame::parse("x", "BB <len:16> <xor>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_a_length_field_followed_by_a_plain_literal() {
    let err = Frame::parse("x", "BB <len:16> B0 ${on} <xor>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn parses_a_multi_byte_opcode() {
    let frame = Frame::parse("x", "BB <len:16> <op:B0B1> ${on}").expect("should parse");
    assert_eq!(
        frame.tokens().get(2),
        Some(&Token::Opcode(vec![0xB0, 0xB1]))
    );
}

#[test]
fn rejects_an_opcode_with_an_odd_number_of_digits() {
    let err = Frame::parse("x", "<op:B> ${on}").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn parses_every_field_shape() {
    let frame = Frame::parse(
        "x",
        "${s:str8} ${t:str16} ${m:mask8} ${z:mask16} ${b:bytes} <pad:20> <xor>",
    )
    .expect("should parse");
    assert_eq!(
        frame.arg_names().collect::<Vec<_>>(),
        ["s", "t", "m", "z", "b"]
    );
}

#[test]
fn rejects_an_unknown_field_shape() {
    let err = Frame::parse("x", "${s:str32}").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_a_second_padding_field() {
    let err = Frame::parse("x", "<pad:20> <pad:20> <xor>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_padding_that_something_follows() {
    let err = Frame::parse("x", "A1 <pad:20> BB <xor>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_padding_to_nothing() {
    let err = Frame::parse("x", "BB <pad:0> <xor>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_an_unknown_token() {
    let err = Frame::parse("x", "BB {on}").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_a_span_that_does_not_close() {
    let err = Frame::parse("x", "<span:16le> 01").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn rejects_a_span_end_with_no_span() {
    let err = Frame::parse("x", "01 </span>").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}

#[test]
fn groups_read_two_arguments() {
    let frame = Frame::parse("x", "${cells,colors:groups}").expect("should parse");
    assert_eq!(frame.arg_names().collect::<Vec<_>>(), ["cells", "colors"]);
}

#[test]
fn rejects_groups_with_one_argument() {
    let err = Frame::parse("x", "${cells:groups}").expect_err("should not parse");
    assert_eq!(err.code(), "frame_syntax");
}
