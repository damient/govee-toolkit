//! Emitting the bytes of a parsed frame layout.
//!
//! Argument values are read, never defaulted: a missing or too-wide value is an
//! error, because the firmware would silently clamp it and report success.

use super::fields::{
    bytes_arg, int_arg, mask, pad, push_int, rgb_arg, text_arg, too_long, write_len, zones_arg,
};
use super::{Frame, RepeatItem, Token};
use crate::codec::args::{ArgValue, Args};
use crate::codec::error::Result;

impl Frame {
    /// Emit the bytes.
    ///
    /// `args` must already be resolved and validated — see
    /// [`crate::codec::command`]; a missing or mistyped value is an error here
    /// too, never a default.
    ///
    /// # Errors
    ///
    /// [`Error::MissingArg`](crate::codec::Error::MissingArg),
    /// [`Error::ArgType`](crate::codec::Error::ArgType) or
    /// [`Error::FrameWidth`](crate::codec::Error::FrameWidth) if a value is
    /// absent, of the wrong shape, or too wide for its field;
    /// [`Error::FieldTooLong`](crate::codec::Error::FieldTooLong) for a string
    /// its length prefix cannot count or a zone its mask cannot carry, and
    /// [`Error::FrameOverflow`](crate::codec::Error::FrameOverflow) for a frame
    /// already past the size `<pad:…>` declares.
    pub fn build(&self, command: &str, args: &Args) -> Result<Vec<u8>> {
        let mut out: Vec<u8> = Vec::with_capacity(self.size_hint(args));
        let mut len_pos: Option<usize> = None;
        let mut payload_start: Option<usize> = None;
        let has_xor = matches!(self.tokens.last(), Some(Token::Xor));

        for token in &self.tokens {
            match token {
                Token::Literal(b) => out.push(*b),
                Token::Len16 => {
                    len_pos = Some(out.len());
                    out.extend_from_slice(&[0, 0]);
                }
                // Parsing guarantees this follows `<len:16>` when there is one,
                // so the payload the length counts starts here.
                Token::Opcode(bytes) => {
                    out.extend_from_slice(bytes);
                    payload_start = Some(out.len());
                }
                Token::Arg { name, bits } => {
                    let value = int_arg(command, args, name)?;
                    push_int(&mut out, command, name, value, *bits)?;
                }
                Token::Text { name, prefix } => {
                    let text = text_arg(command, args, name)?.as_bytes();
                    let max = (1usize << (prefix * 8)) - 1;
                    if text.len() > max {
                        return Err(too_long(command, name, text.len(), max));
                    }
                    let len = text.len().to_be_bytes();
                    out.extend_from_slice(len.get(len.len() - prefix..).unwrap_or_default());
                    out.extend_from_slice(text);
                }
                Token::Mask { name, width } => out.extend_from_slice(&mask(
                    command,
                    zones_arg(command, args, name)?,
                    name,
                    *width,
                )?),
                Token::Bytes { name } => {
                    out.extend_from_slice(bytes_arg(command, args, name)?);
                }
                Token::Pad(size) => pad(&mut out, command, *size, has_xor)?,
                Token::Repeat { list, count, item } => {
                    let n = usize::try_from(int_arg(command, args, count)?).unwrap_or(0);
                    let colors = rgb_arg(command, args, list)?;
                    match item {
                        RepeatItem::Rgb => {
                            for rgb in colors.iter().take(n) {
                                out.extend_from_slice(rgb);
                            }
                        }
                    }
                }
                // `<xor>` is validated as the last token, so nothing follows.
                Token::Xor => break,
            }
        }

        if let (Some(pos), Some(start)) = (len_pos, payload_start) {
            let len = out.len() - start;
            write_len(&mut out, pos, len);
        }
        if has_xor {
            out.push(out.iter().fold(0u8, |acc, b| acc ^ b));
        }
        Ok(out)
    }
}

impl Frame {
    /// Bytes this layout will emit, so the buffer is allocated once. A repeat
    /// group and a variable-width field are sized from the value they will
    /// carry; `<pad:…>` sizes the frame on its own.
    fn size_hint(&self, args: &Args) -> usize {
        let mut size = 0usize;
        for token in &self.tokens {
            size += match token {
                Token::Literal(_) | Token::Xor => 1,
                Token::Len16 => 2,
                Token::Opcode(bytes) => bytes.len(),
                Token::Arg { bits, .. } => (*bits as usize).div_ceil(8),
                Token::Mask { width, .. } => *width,
                Token::Text { name, prefix } => match args.get(name) {
                    Some(ArgValue::Text(text)) => prefix + text.len(),
                    _ => *prefix,
                },
                Token::Bytes { name } => match args.get(name) {
                    Some(ArgValue::Bytes(bytes)) => bytes.len(),
                    _ => 0,
                },
                Token::Pad(declared) => return *declared,
                Token::Repeat { list, item, .. } => match item {
                    RepeatItem::Rgb => match args.get(list) {
                        Some(ArgValue::Rgb(colors)) => colors.len() * 3,
                        _ => 0,
                    },
                },
            };
        }
        size
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::format_collect,
        clippy::indexing_slicing
    )]

    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The one frame in the documentation that comes from a real capture.
    #[test]
    fn arm_frame_matches_the_captured_bytes() {
        let frame = Frame::parse("arm", "BB <len:16> <op:B1> ${on} <xor>").unwrap();
        let bytes = frame.build("arm", &Args::new().int("on", 1)).unwrap();
        assert_eq!(hex(&bytes), "bb0001b1010a");
    }

    #[test]
    fn length_covers_the_payload_only() {
        let frame = Frame::parse(
            "seg",
            "BB <len:16> <op:B0> ${gradient} ${n} (${colors}:rgb)×${n} <xor>",
        )
        .unwrap();
        let args = Args::new()
            .int("gradient", 0)
            .int("n", 2)
            .rgb("colors", vec![[255, 0, 0], [0, 255, 0]]);
        let bytes = frame.build("seg", &args).unwrap();
        // len = 2 + 3 × 2 = 8, header, opcode and checksum excluded.
        assert_eq!(hex(&bytes), "bb0008b00002ff000000ff0001");
    }

    #[test]
    fn a_string_travels_behind_its_length_prefix() {
        let frame = Frame::parse("x", "${s:str8} ${t:str16}").unwrap();
        let args = Args::new().text("s", "Test").text("t", "ab");
        assert_eq!(hex(&frame.build("x", &args).unwrap()), "045465737400026162");
    }

    #[test]
    fn a_string_longer_than_its_prefix_counts_is_refused() {
        let frame = Frame::parse("x", "${s:str8}").unwrap();
        let args = Args::new().text("s", "a".repeat(256));
        assert_eq!(
            frame.build("x", &args).unwrap_err().code(),
            "field_too_long"
        );
    }

    #[test]
    fn zones_become_one_bit_each_least_significant_first() {
        let frame = Frame::parse("x", "${z:mask16}").unwrap();
        let args = Args::new().zones("z", vec![0, 1, 14]);
        assert_eq!(hex(&frame.build("x", &args).unwrap()), "0340");
    }

    /// The firmware drops a bit past its zone count in silence, and a saturated
    /// mask looks exactly like an ignored one, so the width is enforced here.
    #[test]
    fn a_zone_the_mask_cannot_carry_is_refused() {
        let frame = Frame::parse("x", "${z:mask8}").unwrap();
        let args = Args::new().zones("z", vec![8]);
        assert_eq!(frame.build("x", &args).unwrap_err().code(), "out_of_range");
    }

    #[test]
    fn padding_fills_the_frame_out_to_its_declared_size() {
        let frame = Frame::parse("x", "33 01 ${on} <pad:20> <xor>").unwrap();
        let bytes = frame.build("x", &Args::new().int("on", 1)).unwrap();
        assert_eq!(bytes.len(), 20);
        assert_eq!(hex(&bytes), "3301010000000000000000000000000000000033");
    }

    #[test]
    fn a_frame_already_past_its_declared_size_is_refused() {
        let frame = Frame::parse("x", "${b:bytes} <pad:4> <xor>").unwrap();
        let args = Args::new().bytes("b", vec![0; 8]);
        assert_eq!(
            frame.build("x", &args).unwrap_err().code(),
            "frame_overflow"
        );
    }

    #[test]
    fn rejects_a_value_too_wide_for_its_field() {
        let frame = Frame::parse("x", "BB ${v}").unwrap();
        let err = frame.build("x", &Args::new().int("v", 256)).unwrap_err();
        assert_eq!(err.code(), "frame_width");
    }

    proptest::proptest! {
        /// The checksum is the XOR of everything before it, whatever the frame
        /// carries, and the length field always agrees with what follows the
        /// opcode.
        #[test]
        fn checksum_and_length_hold_for_any_zone_count(
            colors in proptest::collection::vec(proptest::array::uniform3(0u8..=255), 1..40),
            gradient in 0i64..=1,
        ) {
            let frame =
                Frame::parse(
                    "seg",
                    "BB <len:16> <op:B0> ${gradient} ${n} (${colors}:rgb)×${n} <xor>",
                )
                    .unwrap();
            let n = i64::try_from(colors.len()).unwrap();
            let args = Args::new().int("gradient", gradient).int("n", n).rgb("colors", colors.clone());
            let bytes = frame.build("seg", &args).unwrap();

            let (body, checksum) = bytes.split_at(bytes.len() - 1);
            proptest::prop_assert_eq!(
                body.iter().fold(0u8, |acc, b| acc ^ b),
                checksum[0]
            );

            let declared = u16::from_be_bytes([bytes[1], bytes[2]]);
            proptest::prop_assert_eq!(usize::from(declared), 2 + 3 * colors.len());
            proptest::prop_assert_eq!(bytes.len(), 4 + usize::from(declared) + 1);
        }
    }
}
