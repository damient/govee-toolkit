//! Emitting the bytes of a parsed frame layout.
//!
//! Argument values are read, never defaulted: a missing or too-wide value is an
//! error, because the firmware would silently clamp it and report success.

use super::{Frame, RepeatItem, Token};
use crate::args::{ArgValue, Args};
use crate::error::{Error, Result};

impl Frame {
    /// Emit the bytes.
    ///
    /// `args` must already be resolved and validated — see
    /// [`crate::command`]; a missing or mistyped value is an error here too,
    /// never a default.
    ///
    /// # Errors
    ///
    /// [`Error::MissingArg`], [`Error::ArgType`] or [`Error::FrameWidth`] if a
    /// value is absent, of the wrong shape, or too wide for its field.
    pub fn build(&self, command: &str, args: &Args) -> Result<Vec<u8>> {
        let mut out: Vec<u8> = Vec::new();
        let mut len_pos: Option<usize> = None;
        let mut payload_start: Option<usize> = None;
        let mut opcode_pending = false;

        for token in &self.tokens {
            match token {
                Token::Literal(b) => out.push(*b),
                Token::Len16 => {
                    len_pos = Some(out.len());
                    out.extend_from_slice(&[0, 0]);
                    opcode_pending = true;
                    continue;
                }
                Token::Arg { name, bits } => {
                    let value = int_arg(command, args, name)?;
                    push_int(&mut out, command, name, value, *bits)?;
                }
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
                Token::Xor => {
                    if let (Some(pos), Some(start)) = (len_pos, payload_start) {
                        let len = out.len() - start;
                        write_len(&mut out, pos, len);
                    }
                    out.push(out.iter().fold(0u8, |acc, b| acc ^ b));
                    return Ok(out);
                }
            }
            if opcode_pending {
                payload_start = Some(out.len());
                opcode_pending = false;
            }
        }

        if let (Some(pos), Some(start)) = (len_pos, payload_start) {
            let len = out.len() - start;
            write_len(&mut out, pos, len);
        }
        Ok(out)
    }
}

fn write_len(out: &mut [u8], pos: usize, len: usize) {
    let len = u16::try_from(len).unwrap_or(u16::MAX).to_be_bytes();
    if let Some(slot) = out.get_mut(pos..pos + 2) {
        slot.copy_from_slice(&len);
    }
}

fn push_int(out: &mut Vec<u8>, command: &str, name: &str, value: i64, bits: u32) -> Result<()> {
    let too_wide = || Error::FrameWidth {
        command: command.to_owned(),
        arg: name.to_owned(),
        value,
        bits,
    };
    match bits {
        8 => out.push(u8::try_from(value).map_err(|_| too_wide())?),
        16 => out.extend_from_slice(&u16::try_from(value).map_err(|_| too_wide())?.to_be_bytes()),
        _ => return Err(too_wide()),
    }
    Ok(())
}

fn int_arg(command: &str, args: &Args, name: &str) -> Result<i64> {
    match args.get(name) {
        Some(ArgValue::Int(v)) => Ok(*v),
        Some(other) => Err(Error::ArgType {
            command: command.to_owned(),
            arg: name.to_owned(),
            expected: "an integer",
            got: other.type_name(),
        }),
        None => Err(Error::MissingArg {
            command: command.to_owned(),
            arg: name.to_owned(),
        }),
    }
}

fn rgb_arg<'a>(command: &str, args: &'a Args, name: &str) -> Result<&'a [[u8; 3]]> {
    match args.get(name) {
        Some(ArgValue::Rgb(v)) => Ok(v),
        Some(other) => Err(Error::ArgType {
            command: command.to_owned(),
            arg: name.to_owned(),
            expected: "a list of RGB triples",
            got: other.type_name(),
        }),
        None => Err(Error::MissingArg {
            command: command.to_owned(),
            arg: name.to_owned(),
        }),
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
        let frame = Frame::parse("arm", "BB 00 01 B1 ${on} <xor>").unwrap();
        let bytes = frame.build("arm", &Args::new().int("on", 1)).unwrap();
        assert_eq!(hex(&bytes), "bb0001b1010a");
    }

    #[test]
    fn length_covers_the_payload_only() {
        let frame = Frame::parse(
            "seg",
            "BB <len:16> B0 ${gradient} ${n} (${colors}:rgb)×${n} <xor>",
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
                Frame::parse("seg", "BB <len:16> B0 ${gradient} ${n} (${colors}:rgb)×${n} <xor>")
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
