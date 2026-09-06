//! The `reply:` mini-language: reading the bytes a device answers with.
//!
//! The same shape as `frame:`, in the other direction. A layout is a
//! whitespace-separated token string; a literal byte must be there, and a
//! `${name}` field captures. Nothing is built, so the tokens that build —
//! `<xor>`, `<len:16>`, `<op:…>`, `<pad:…>` and a repeat group — are refused:
//! a `reply:` is capture-only.
//!
//! | Token | Reads |
//! | ----- | ----- |
//! | `BB` | that literal byte, which the reply must carry |
//! | `${name}` | one byte, as an integer |
//! | `${name:16}` | two bytes, big-endian, as an integer |
//! | `${name:bytes:6}` | exactly six bytes, as they are |
//! | `${name:ascii}` | the rest of the reply as text, trailing zeros trimmed |
//! | `${name:ascii:17}` | exactly seventeen bytes, read the same way |
//!
//! Unbounded `${name:ascii}` reads to the end, so it comes last. Give it a
//! length when the reply carries anything after the text. A wire that ends
//! every frame on a checksum is the usual reason: a read to the end swallows
//! that byte and fails on it as unprintable.
//!
//! Either way the field must keep at least one character once the padding is
//! off. A reply that stops at the layout's last literal, and one that carries
//! nothing but zeros, are both refused rather than captured as an empty
//! string. What it keeps must be printable ASCII, `0x20` to `0x7e`: a binary
//! answer of low bytes is a mismatch, not text. Bytes past the layout are
//! ignored, since a wire whose frames are a fixed size pads a short reply out.
//!
//! A reply that does not carry a literal the layout requires is refused whole.
//! Nothing partial is returned: a frame that does not match is another
//! command's answer, or a firmware that does not do what the file says.

use crate::codec::args::ArgValue;
use crate::codec::error::{Error, Result};

mod captured;

pub use captured::Captured;

/// What one captured field reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// A big-endian integer, `bits` wide.
    Int {
        /// `8` or `16`.
        bits: u32,
    },
    /// A fixed run of bytes.
    Bytes {
        /// How many.
        len: usize,
    },
    /// Text, trailing zeros trimmed.
    Ascii {
        /// How many bytes it spans, or `None` to read to the end of the reply.
        len: Option<usize>,
    },
}

/// One element of a parsed reply layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A byte the reply must carry.
    Literal(u8),
    /// A field to capture.
    Capture {
        /// The name it is captured under.
        name: String,
        /// What it reads.
        field: Field,
    },
}

/// A parsed `reply:` layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    tokens: Vec<Token>,
}

impl Layout {
    /// Parse a `reply:` string.
    ///
    /// `command` only names the command in error messages.
    ///
    /// # Errors
    ///
    /// [`Error::ReplySyntax`] if a token is unrecognized, if it builds bytes
    /// rather than matching them, if two fields share a name, or if an
    /// unbounded `${name:ascii}` is not last.
    pub fn parse(command: &str, source: &str) -> Result<Self> {
        let bad = |reason: String| Error::ReplySyntax {
            command: command.to_owned(),
            reply: source.to_owned(),
            reason,
        };

        let mut tokens = Vec::new();
        for raw in source.split_whitespace() {
            if raw.starts_with('<') || raw.starts_with('(') {
                return Err(bad(format!(
                    "`{raw}` builds bytes; a `reply:` only matches them"
                )));
            }
            tokens
                .push(parse_token(raw).ok_or_else(|| bad(format!("unrecognized token `{raw}`")))?);
        }
        if tokens.is_empty() {
            return Err(bad("empty".to_owned()));
        }

        let mut names: Vec<&str> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            let Token::Capture { name, field } = token else {
                continue;
            };
            if names.contains(&name.as_str()) {
                return Err(bad(format!("`{name}` is captured twice")));
            }
            names.push(name);
            if *field == (Field::Ascii { len: None }) && i + 1 != tokens.len() {
                return Err(bad(
                    "`:ascii` with no length reads to the end of the reply, so it must be the \
                     last token; give it one to read a field the rest of the frame follows"
                        .to_owned(),
                ));
            }
        }

        Ok(Self { tokens })
    }

    /// Every name the layout captures, in the order it reads them.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.tokens.iter().filter_map(|t| match t {
            Token::Capture { name, .. } => Some(name.as_str()),
            Token::Literal(_) => None,
        })
    }

    /// Read the fields out of one reply.
    ///
    /// # Errors
    ///
    /// [`Error::ReplyMismatch`] if the reply is shorter than the layout, if a
    /// literal does not match, if an `:ascii` field reads no text once its
    /// padding is off, or if it carries anything but printable ASCII. Nothing
    /// captured before the failure is returned.
    pub fn read(&self, command: &str, bytes: &[u8]) -> Result<Captured> {
        let bad = |reason: String| Error::ReplyMismatch {
            command: command.to_owned(),
            reason,
        };
        let short = |at: usize, want: usize| {
            bad(format!(
                "the reply is {} bytes, and the layout reads {want} more at byte {at}",
                bytes.len()
            ))
        };

        let mut captured = Captured::new();
        let mut at = 0usize;
        for token in &self.tokens {
            match token {
                Token::Literal(expected) => {
                    let Some(got) = bytes.get(at) else {
                        return Err(short(at, 1));
                    };
                    if got != expected {
                        return Err(bad(format!(
                            "byte {at} is {got:#04x}, not the {expected:#04x} the layout requires"
                        )));
                    }
                    at = at.saturating_add(1);
                }
                Token::Capture { name, field } => {
                    at = read_field(&mut captured, name, *field, bytes, at, &bad, &short)?;
                }
            }
        }
        Ok(captured)
    }
}

/// Returns where the next field starts.
fn read_field(
    captured: &mut Captured,
    name: &str,
    field: Field,
    bytes: &[u8],
    at: usize,
    bad: &impl Fn(String) -> Error,
    short: &impl Fn(usize, usize) -> Error,
) -> Result<usize> {
    let width = match field {
        Field::Int { bits } => (bits as usize).div_ceil(8),
        Field::Bytes { len } => len,
        Field::Ascii { len } => len.unwrap_or_else(|| bytes.len().saturating_sub(at)),
    };
    let end = at.saturating_add(width);
    let Some(slice) = bytes.get(at..end) else {
        return Err(short(at, width));
    };

    let value = match field {
        Field::Int { .. } => ArgValue::Int(slice.iter().fold(0i64, |acc, b| {
            acc.saturating_mul(256).saturating_add(i64::from(*b))
        })),
        Field::Bytes { .. } => ArgValue::Bytes(slice.to_vec()),
        Field::Ascii { .. } => {
            let text = trim_padding(slice);
            // All-zero padding and a reply that stops at the last literal both
            // leave nothing, and neither is the text the layout describes.
            if text.is_empty() {
                return Err(bad(format!("`{name}` reads no text at byte {at}")));
            }
            if !text.iter().all(|b| (0x20..=0x7e).contains(b)) {
                return Err(bad(format!("`{name}` is not printable text")));
            }
            ArgValue::Text(String::from_utf8_lossy(text).into_owned())
        }
    };
    captured.insert(name, value);
    Ok(end)
}

/// A fixed-size wire pads a short answer with zeros; they carry nothing.
fn trim_padding(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    bytes.get(..end).unwrap_or_default()
}

fn parse_token(raw: &str) -> Option<Token> {
    if let Some(inner) = raw.strip_prefix("${").and_then(|r| r.strip_suffix('}')) {
        let (name, kind) = match inner.split_once(':') {
            None => (inner, "8"),
            Some((name, kind)) => (name, kind),
        };
        if name.is_empty() {
            return None;
        }
        let field = match kind {
            "8" => Field::Int { bits: 8 },
            "16" => Field::Int { bits: 16 },
            "ascii" => Field::Ascii { len: None },
            _ => {
                if let Some(len) = kind.strip_prefix("ascii:") {
                    Field::Ascii {
                        len: Some(len.parse().ok()?),
                    }
                } else {
                    let len: usize = kind.strip_prefix("bytes:")?.parse().ok()?;
                    Field::Bytes { len }
                }
            }
        };
        return Some(Token::Capture {
            name: name.to_owned(),
            field,
        });
    }
    (raw.len() == 2 && raw.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u8::from_str_radix(raw, 16).ok())
        .flatten()
        .map(Token::Literal)
}

#[cfg(test)]
mod tests;
