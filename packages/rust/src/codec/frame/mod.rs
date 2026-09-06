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
//! | `${name:str8}` | one length byte, then `name` as UTF-8 |
//! | `${name:str16}` | two big-endian length bytes, then `name` as UTF-8 |
//! | `${name:mask8}` | one byte of zone bits, least significant bit first |
//! | `${name:mask16}` | two bytes of zone bits, least significant bit first |
//! | `${name:bytes}` | the bytes of `name`, as they are |
//! | `<op:B0>` | the opcode, one or more literal bytes, marked as the opcode |
//! | `<len:16>` | the payload length, big-endian, filled in once the frame is built |
//! | `<pad:20>` | zeros, so that the whole frame is 20 bytes once `<xor>` is appended |
//! | `(${list}:rgb)×${count}` | `count` RGB triples, taken from the list argument `list` |
//! | `<xor>` | the XOR of every preceding byte |
//!
//! `<len:16>` counts the bytes emitted after `<op:…>`, up to but excluding the
//! checksum: the payload alone, header, opcode and checksum excluded — the
//! definition in `docs/protocol/lan.md` 2.3. A frame that declares `<len:16>`
//! must name its opcode with `<op:…>` immediately after, so the boundary the
//! length measures from is written down rather than inferred from position.
//!
//! A string longer than its length prefix can count, and a zone index past the
//! width of its mask, are both errors: the field would carry something other
//! than what the caller asked for.
//!
//! `<xor>`, when present, must be the last token, and `<pad:…>` the one before
//! it: padding that is not at the end would be followed by bytes the declared
//! size does not account for.

use crate::codec::error::{Error, Result};

mod build;
mod fields;

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
    /// A string argument, behind a length prefix `prefix` bytes wide.
    Text {
        /// The argument name.
        name: String,
        /// `1` or `2` length bytes, big-endian.
        prefix: usize,
    },
    /// Zone indices as a bitmask `width` bytes wide, least significant bit
    /// first.
    Mask {
        /// The argument name.
        name: String,
        /// How many bytes the mask occupies.
        width: usize,
    },
    /// A byte-string argument, emitted as it is.
    Bytes {
        /// The argument name.
        name: String,
    },
    /// The opcode, as literal bytes. The payload `<len:16>` counts starts
    /// after it.
    Opcode(Vec<u8>),
    /// The 16-bit payload length, big-endian.
    Len16,
    /// Zeros, up to a frame of this many bytes once `<xor>` is appended.
    Pad(usize),
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
    /// [`Error::FrameSyntax`] if a token is unrecognized, if `<len:16>`,
    /// `<op:…>`, `<pad:…>` or `<xor>` appear more than once or in an impossible
    /// position, or if `<len:16>` is not immediately followed by `<op:…>`.
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

        let (len, len_repeats) = only(&tokens, |t| matches!(t, Token::Len16));
        let (_, op_repeats) = only(&tokens, |t| matches!(t, Token::Opcode(_)));
        let (xor, xor_repeats) = only(&tokens, |t| matches!(t, Token::Xor));
        let (pad, pad_repeats) = only(&tokens, |t| matches!(t, Token::Pad(_)));
        if pad_repeats {
            return Err(bad("`<pad:…>` appears more than once".to_owned()));
        }
        if len_repeats {
            return Err(bad("`<len:16>` appears more than once".to_owned()));
        }
        if op_repeats {
            return Err(bad("`<op:…>` appears more than once".to_owned()));
        }
        if xor_repeats {
            return Err(bad("`<xor>` appears more than once".to_owned()));
        }
        if let Some(i) = xor
            && i + 1 != tokens.len()
        {
            return Err(bad("`<xor>` must be the last token".to_owned()));
        }
        if let Some(i) = pad
            && i + 1 + usize::from(xor.is_some()) != tokens.len()
        {
            return Err(bad(
                "`<pad:…>` must come last, before `<xor>` where there is one".to_owned(),
            ));
        }
        if let Some(i) = len
            && !matches!(tokens.get(i + 1), Some(Token::Opcode(_)))
        {
            return Err(bad("`<len:16>` must be followed by `<op:…>`".to_owned()));
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

    /// Every argument the layout reads, in the order it reads them. A name
    /// referenced twice appears twice.
    pub fn arg_names(&self) -> impl Iterator<Item = &str> {
        self.tokens.iter().flat_map(|t| match t {
            Token::Arg { name, .. }
            | Token::Text { name, .. }
            | Token::Mask { name, .. }
            | Token::Bytes { name } => vec![name.as_str()],
            Token::Repeat { list, count, .. } => vec![list.as_str(), count.as_str()],
            Token::Literal(_) | Token::Opcode(_) | Token::Len16 | Token::Pad(_) | Token::Xor => {
                Vec::new()
            }
        })
    }
}

/// Where the first matching token is, and whether another one follows it.
fn only(tokens: &[Token], pred: impl Fn(&Token) -> bool) -> (Option<usize>, bool) {
    let mut matches = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| pred(t))
        .map(|(i, _)| i);
    (matches.next(), matches.next().is_some())
}

fn parse_token(raw: &str) -> Option<Token> {
    match raw {
        "<len:16>" => return Some(Token::Len16),
        "<xor>" => return Some(Token::Xor),
        _ => {}
    }
    if let Some(hex) = raw.strip_prefix("<op:").and_then(|r| r.strip_suffix('>')) {
        return parse_opcode(hex);
    }
    if let Some(size) = raw.strip_prefix("<pad:").and_then(|r| r.strip_suffix('>')) {
        return size.parse().ok().filter(|s| *s > 0).map(Token::Pad);
    }
    if let Some(inner) = raw.strip_prefix("${").and_then(|r| r.strip_suffix('}')) {
        return parse_arg_ref(inner);
    }
    if raw.starts_with('(') {
        return parse_repeat(raw);
    }
    hex_byte(raw).map(Token::Literal)
}

/// Exactly two hex digits, as the byte they spell.
fn hex_byte(raw: &str) -> Option<u8> {
    (raw.len() == 2 && raw.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u8::from_str_radix(raw, 16).ok())
        .flatten()
}

/// `<op:…>` carries an even number of hex digits: `B0`, or `B0B1` for a
/// two-byte opcode.
fn parse_opcode(hex: &str) -> Option<Token> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = hex
        .as_bytes()
        .chunks(2)
        .map(|pair| hex_byte(std::str::from_utf8(pair).ok()?))
        .collect::<Option<Vec<u8>>>()?;
    Some(Token::Opcode(bytes))
}

/// `name`, or `name:` followed by the field's shape.
fn parse_arg_ref(inner: &str) -> Option<Token> {
    let (name, kind) = match inner.split_once(':') {
        None => (inner, "8"),
        Some((name, kind)) => (name, kind),
    };
    if name.is_empty() {
        return None;
    }
    let name = name.to_owned();
    Some(match kind {
        "8" => Token::Arg { name, bits: 8 },
        "16" => Token::Arg { name, bits: 16 },
        "str8" => Token::Text { name, prefix: 1 },
        "str16" => Token::Text { name, prefix: 2 },
        "mask8" => Token::Mask { name, width: 1 },
        "mask16" => Token::Mask { name, width: 2 },
        "bytes" => Token::Bytes { name },
        _ => return None,
    })
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
}
