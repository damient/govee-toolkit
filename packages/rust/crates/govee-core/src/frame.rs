//! The `frame:` mini-language used by raw-channel commands.
//!
//! A device file describes a raw frame as a whitespace-separated token string,
//! so the byte layout stays readable next to the command that sends it and the
//! codec below stays generic — no SKU and no command name appears in this file.
//!
//! | Token | Emits |
//! | ----- | ----- |
//! | `BB` | that literal byte, two hex digits |
//! | `${name}` | one byte, from the integer argument `name` |
//! | `${name:16}` | two bytes, big-endian, from `name` |
//! | `<len:16>` | the payload length, big-endian, filled in once the frame is built |
//! | `(${list}:rgb)×${count}` | `count` RGB triples, taken from the list argument `list` |
//! | `<xor>` | the XOR of every preceding byte |
//!
//! `<len:16>` counts the bytes emitted **after the token that follows it** — in
//! this dialect that token is the opcode — up to but excluding the checksum.
//! That is the definition in `docs/protocol/lan.md` 2.3: the length covers the
//! payload alone, header, opcode and checksum excluded.
//!
//! `<xor>`, when present, must be the last token.

use crate::args::{ArgValue, Args};
use crate::error::{Error, Result};

/// One element of a parsed frame layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A fixed byte.
    Literal(u8),
    /// An integer argument, emitted at `bits` width, big-endian.
    Arg {
        /// The argument name.
        name: String,
        /// `8` or `16`.
        bits: u32,
    },
    /// The 16-bit payload length, big-endian.
    Len16,
    /// `count` items drawn from a list argument.
    Repeat {
        /// The list argument.
        list: String,
        /// The integer argument holding how many items to emit.
        count: String,
        /// What one item looks like.
        item: RepeatItem,
    },
    /// The XOR checksum of every preceding byte.
    Xor,
}

/// The shape of one item in a repeat group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatItem {
    /// Three bytes: red, green, blue.
    Rgb,
}

/// A parsed `frame:` layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    tokens: Vec<Token>,
}

impl Frame {
    /// Parse a `frame:` string.
    ///
    /// `command` only names the command in error messages.
    ///
    /// # Errors
    ///
    /// [`Error::FrameSyntax`] if a token is unrecognized, or if `<len:16>` /
    /// `<xor>` appear more than once or in an impossible position.
    pub fn parse(command: &str, source: &str) -> Result<Self> {
        let bad = |reason: String| Error::FrameSyntax {
            command: command.to_owned(),
            frame: source.to_owned(),
            reason,
        };

        let mut tokens = Vec::new();
        for raw in source.split_whitespace() {
            tokens
                .push(parse_token(raw).ok_or_else(|| bad(format!("unrecognized token `{raw}`")))?);
        }
        if tokens.is_empty() {
            return Err(bad("empty".to_owned()));
        }

        let len_at = position_of(&tokens, |t| matches!(t, Token::Len16));
        if count_of(&tokens, |t| matches!(t, Token::Len16)) > 1 {
            return Err(bad("`<len:16>` appears more than once".to_owned()));
        }
        if count_of(&tokens, |t| matches!(t, Token::Xor)) > 1 {
            return Err(bad("`<xor>` appears more than once".to_owned()));
        }
        if let Some(i) = position_of(&tokens, |t| matches!(t, Token::Xor))
            && i + 1 != tokens.len()
        {
            return Err(bad("`<xor>` must be the last token".to_owned()));
        }
        // The token right after the length field is the opcode, and the payload
        // it counts starts after that. Both must exist.
        if let Some(i) = len_at {
            match tokens.get(i + 1) {
                None | Some(Token::Xor) => {
                    return Err(bad("`<len:16>` must be followed by an opcode".to_owned()));
                }
                Some(_) => {}
            }
        }

        Ok(Self { tokens })
    }

    /// The parsed tokens.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Every repeat group in the layout, as `(list argument, count argument)`.
    pub fn repeat_groups(&self) -> impl Iterator<Item = (&str, &str)> {
        self.tokens.iter().filter_map(|t| match t {
            Token::Repeat { list, count, .. } => Some((list.as_str(), count.as_str())),
            _ => None,
        })
    }

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

fn position_of(tokens: &[Token], pred: impl Fn(&Token) -> bool) -> Option<usize> {
    tokens.iter().position(pred)
}

fn count_of(tokens: &[Token], pred: impl Fn(&Token) -> bool) -> usize {
    tokens.iter().filter(|t| pred(t)).count()
}

fn parse_token(raw: &str) -> Option<Token> {
    match raw {
        "<len:16>" => return Some(Token::Len16),
        "<xor>" => return Some(Token::Xor),
        _ => {}
    }
    if let Some(inner) = raw.strip_prefix("${").and_then(|r| r.strip_suffix('}')) {
        return parse_arg_ref(inner).map(|(name, bits)| Token::Arg {
            name: name.to_owned(),
            bits,
        });
    }
    if raw.starts_with('(') {
        return parse_repeat(raw);
    }
    if raw.len() == 2 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return u8::from_str_radix(raw, 16).ok().map(Token::Literal);
    }
    None
}

/// `name` or `name:16`.
fn parse_arg_ref(inner: &str) -> Option<(&str, u32)> {
    match inner.split_once(':') {
        None if !inner.is_empty() => Some((inner, 8)),
        Some((name, "16")) if !name.is_empty() => Some((name, 16)),
        _ => None,
    }
}

/// `(${list}:rgb)×${count}`, with `x` accepted for the multiplication sign.
fn parse_repeat(raw: &str) -> Option<Token> {
    let (group, rest) = raw.strip_prefix('(')?.split_once(')')?;
    let count = rest
        .strip_prefix('\u{d7}')
        .or_else(|| rest.strip_prefix('x'))?;
    let count = count.strip_prefix("${").and_then(|c| c.strip_suffix('}'))?;
    let (list, kind) = group.split_once(':')?;
    let list = list.strip_prefix("${").and_then(|l| l.strip_suffix('}'))?;
    let item = match kind {
        "rgb" => RepeatItem::Rgb,
        _ => return None,
    };
    if list.is_empty() || count.is_empty() {
        return None;
    }
    Some(Token::Repeat {
        list: list.to_owned(),
        count: count.to_owned(),
        item,
    })
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
    fn ascii_x_is_accepted_for_the_multiplication_sign() {
        let a = Frame::parse("seg", "(${c}:rgb)×${n}").unwrap();
        let b = Frame::parse("seg", "(${c}:rgb)x${n}").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_a_checksum_that_is_not_last() {
        let err = Frame::parse("x", "BB <xor> 01").unwrap_err();
        assert_eq!(err.code(), "frame_syntax");
    }

    #[test]
    fn rejects_a_length_field_with_no_opcode_after_it() {
        let err = Frame::parse("x", "BB <len:16> <xor>").unwrap_err();
        assert_eq!(err.code(), "frame_syntax");
    }

    #[test]
    fn rejects_an_unknown_token() {
        let err = Frame::parse("x", "BB {on}").unwrap_err();
        assert_eq!(err.code(), "frame_syntax");
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
