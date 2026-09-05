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

use crate::codec::error::{Error, Result};

mod build;

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

        let lens: Vec<usize> = positions(&tokens, |t| matches!(t, Token::Len16));
        let xors: Vec<usize> = positions(&tokens, |t| matches!(t, Token::Xor));
        if lens.len() > 1 {
            return Err(bad("`<len:16>` appears more than once".to_owned()));
        }
        if xors.len() > 1 {
            return Err(bad("`<xor>` appears more than once".to_owned()));
        }
        if let Some(i) = xors.first()
            && i + 1 != tokens.len()
        {
            return Err(bad("`<xor>` must be the last token".to_owned()));
        }
        // The token right after the length field is the opcode, and the payload
        // it counts starts after that. Both must exist.
        if let Some(&i) = lens.first() {
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
}

fn positions(tokens: &[Token], pred: impl Fn(&Token) -> bool) -> Vec<usize> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| pred(t))
        .map(|(i, _)| i)
        .collect()
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
    fn rejects_an_unknown_token() {
        let err = Frame::parse("x", "BB {on}").expect_err("should not parse");
        assert_eq!(err.code(), "frame_syntax");
    }
}
