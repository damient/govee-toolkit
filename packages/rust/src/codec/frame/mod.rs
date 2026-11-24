//! The `frame:` mini-language used by raw-channel commands.
//!
//! A device file writes a frame as a whitespace-separated token string, so the
//! layout stays next to its command and no SKU name reaches this code.
//! `devices/schema.yaml` lists the tokens.
//!
//! Four rules, each of them an error. A `<span:…>` closes with a `</span>`
//! later in the same layout. `<len:16>` counts the bytes after
//! `<op:...>` up to but excluding the checksum, so a layout declaring it must
//! put `<op:...>` immediately after. A checksum — `<xor>` or `<sum>`, and one
//! of the two at most — comes last and `<pad:...>` immediately before it. And
//! a string past what its length prefix counts, or a zone past the width of
//! its mask, is refused.

use crate::codec::error::{Error, Result};

mod build;
mod fields;
mod groups;

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
    /// Zeros, up to a frame of this many bytes once the checksum is appended.
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
    /// A length field `width` bytes wide. It counts the bytes from after it
    /// up to the matching [`Token::SpanEnd`].
    Span {
        /// `1` or `2`.
        width: usize,
        /// Whether the least significant byte comes first.
        little: bool,
    },
    /// The end of the innermost open [`Token::Span`].
    SpanEnd,
    /// Indices grouped by color, one byte each: the group count, then for
    /// each group the index count, the color and the indices.
    Groups {
        /// The zone argument that holds the indices.
        cells: String,
        /// The list argument that holds one color, or one color per index.
        colors: String,
    },
    /// The XOR checksum of every preceding byte.
    Xor,
    /// The low byte of the sum of every preceding byte.
    Sum,
}

impl Token {
    fn is_checksum(&self) -> bool {
        matches!(self, Self::Xor | Self::Sum)
    }
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
    /// `<op:…>`, `<pad:…>` or a checksum appear more than once or in an
    /// impossible position, if `<xor>` and `<sum>` appear together, if
    /// `<len:16>` is not immediately followed by `<op:…>`, or if a
    /// `<span:…>` and a `</span>` do not pair up.
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
        let (checksum, checksum_repeats) = only(&tokens, Token::is_checksum);
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
        if checksum_repeats {
            return Err(bad(
                "`<xor>` and `<sum>` carry one checksum between them, and it appears more than \
                 once"
                    .to_owned(),
            ));
        }
        if let Some(i) = checksum
            && i + 1 != tokens.len()
        {
            return Err(bad("a checksum must be the last token".to_owned()));
        }
        if let Some(i) = pad
            && i + 1 + usize::from(checksum.is_some()) != tokens.len()
        {
            return Err(bad(
                "`<pad:…>` must come last, before the checksum where there is one".to_owned(),
            ));
        }
        if let Some(i) = len
            && !matches!(tokens.get(i + 1), Some(Token::Opcode(_)))
        {
            return Err(bad("`<len:16>` must be followed by `<op:…>`".to_owned()));
        }

        balanced(&tokens).map_err(|reason| bad(reason.to_owned()))?;
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
        self.tokens
            .iter()
            .flat_map(|t| match t {
                Token::Arg { name, .. }
                | Token::Text { name, .. }
                | Token::Mask { name, .. }
                | Token::Bytes { name } => [Some(name.as_str()), None],
                Token::Repeat { list, count, .. } => [Some(list.as_str()), Some(count.as_str())],
                Token::Groups { cells, colors } => [Some(cells.as_str()), Some(colors.as_str())],
                Token::Literal(_)
                | Token::Opcode(_)
                | Token::Len16
                | Token::Pad(_)
                | Token::Span { .. }
                | Token::SpanEnd
                | Token::Xor
                | Token::Sum => [None, None],
            })
            .flatten()
    }
}

/// Every `<span:…>` closes, after it opens.
fn balanced(tokens: &[Token]) -> std::result::Result<(), &'static str> {
    let mut open = 0usize;
    for token in tokens {
        match token {
            Token::Span { .. } => open += 1,
            Token::SpanEnd => {
                open = open
                    .checked_sub(1)
                    .ok_or("`</span>` closes no `<span:…>`")?;
            }
            _ => {}
        }
    }
    if open > 0 {
        return Err("a `<span:…>` has no `</span>`");
    }
    Ok(())
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
        "<sum>" => return Some(Token::Sum),
        "</span>" => return Some(Token::SpanEnd),
        "<span:8>" => {
            return Some(Token::Span {
                width: 1,
                little: false,
            });
        }
        "<span:16>" => {
            return Some(Token::Span {
                width: 2,
                little: false,
            });
        }
        "<span:16le>" => {
            return Some(Token::Span {
                width: 2,
                little: true,
            });
        }
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
    if kind == "groups" {
        let (cells, colors) = name.split_once(',')?;
        let (cells, colors) = (cells.trim(), colors.trim());
        if cells.is_empty() || colors.is_empty() || colors.contains(',') {
            return None;
        }
        return Some(Token::Groups {
            cells: cells.to_owned(),
            colors: colors.to_owned(),
        });
    }
    let name = name.to_owned();
    Some(match kind {
        "8" => Token::Arg { name, bits: 8 },
        "16" => Token::Arg { name, bits: 16 },
        "str8" => Token::Text { name, prefix: 1 },
        "str16" => Token::Text { name, prefix: 2 },
        "mask8" => Token::Mask { name, width: 1 },
        "mask16" => Token::Mask { name, width: 2 },
        "mask32" => Token::Mask { name, width: 4 },
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
mod tests;
