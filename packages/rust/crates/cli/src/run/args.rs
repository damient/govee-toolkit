//! Values a person types, read against the type the device file declares.
//!
//! The device file declares the type of every argument, so nothing here
//! guesses one: `send` looks the argument up first, and this module reads the
//! text under that type. The range stays the codec's to check.

use govee_toolkit::codec::{ArgSpec, ArgValue};

use crate::output::Failure;

/// The type name `devices/schema.yaml` uses, for a message a person reads.
pub(super) fn kind(spec: &ArgSpec) -> &'static str {
    match spec {
        ArgSpec::Int { .. } => "int",
        ArgSpec::RgbList { .. } => "rgb_list",
        ArgSpec::String { .. } => "string",
        ArgSpec::Zones { .. } => "zones",
        ArgSpec::Bytes { .. } => "bytes",
    }
}

/// Read one value under the type its argument declares.
///
/// # Errors
///
/// [`Failure::usage`] when the text does not read as that type. A value of the
/// right type but outside the declared range reaches the codec, which refuses
/// it there.
pub(super) fn parse(name: &str, spec: &ArgSpec, text: &str) -> Result<ArgValue, Failure> {
    match spec {
        ArgSpec::Int { .. } => text
            .trim()
            .parse::<i64>()
            .map(ArgValue::Int)
            .map_err(|_| refused(name, text, "a whole number")),
        ArgSpec::RgbList { .. } => list(text)
            .map(rgb)
            .collect::<Result<Vec<_>, _>>()
            .map(ArgValue::Rgb),
        ArgSpec::String { .. } => Ok(ArgValue::Text(text.to_owned())),
        ArgSpec::Zones { .. } => list(text)
            .map(|item| {
                item.parse::<u16>()
                    .map_err(|_| refused(name, item, "a zone index, zero-based"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(ArgValue::Zones),
        ArgSpec::Bytes { .. } => bytes(name, text).map(ArgValue::Bytes),
    }
}

/// Read `#RRGGBB`, or the same six digits with no `#`.
///
/// # Errors
///
/// [`Failure::usage`] for anything else.
pub(super) fn rgb(text: &str) -> Result<[u8; 3], Failure> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    let value = (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
        .ok_or_else(|| Failure::usage(format!("`{text}` is not a color; write `#RRGGBB`")))?;
    Ok([
        u8::try_from(value >> 16 & 0xFF).unwrap_or_default(),
        u8::try_from(value >> 8 & 0xFF).unwrap_or_default(),
        u8::try_from(value & 0xFF).unwrap_or_default(),
    ])
}

/// The items of a comma-separated list, each trimmed. An empty list has no
/// items.
pub(super) fn list(text: &str) -> impl Iterator<Item = &str> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// Read pairs of hexadecimal digits. Spaces and colons separate them, or
/// nothing does.
fn bytes(name: &str, text: &str) -> Result<Vec<u8>, Failure> {
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err(refused(name, text, "pairs of hexadecimal digits"));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let mut rest = digits.as_str();
    while !rest.is_empty() {
        let (pair, tail) = rest.split_at(2);
        out.push(
            u8::from_str_radix(pair, 16)
                .map_err(|_| refused(name, text, "pairs of hexadecimal digits"))?,
        );
        rest = tail;
    }
    Ok(out)
}

fn refused(name: &str, text: &str, wanted: &str) -> Failure {
    Failure::usage(format!("`{name}`: `{text}` is not {wanted}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int() -> ArgSpec {
        ArgSpec::Int {
            range: [0, 100],
            role: None,
        }
    }

    #[test]
    fn a_value_is_read_under_the_declared_type() {
        assert_eq!(parse("level", &int(), "50").ok(), Some(ArgValue::Int(50)));
        assert_eq!(
            parse(
                "zones",
                &ArgSpec::Zones {
                    count: None,
                    role: None
                },
                "0, 2,3"
            )
            .ok(),
            Some(ArgValue::Zones(vec![0, 2, 3]))
        );
    }

    #[test]
    fn a_value_of_the_wrong_type_is_refused() {
        assert!(parse("level", &int(), "bright").is_err());
    }

    #[test]
    fn a_range_belongs_to_the_codec_and_is_not_checked_here() {
        assert_eq!(parse("level", &int(), "400").ok(), Some(ArgValue::Int(400)));
    }

    #[test]
    fn bytes_read_with_or_without_separators() {
        let spec = ArgSpec::Bytes {
            max_len: None,
            role: None,
        };
        let wanted = Some(ArgValue::Bytes(vec![0xAA, 0x0F, 0x01]));
        assert_eq!(parse("raw", &spec, "aa 0f 01").ok(), wanted);
        assert_eq!(parse("raw", &spec, "aa:0f:01").ok(), wanted);
        assert_eq!(parse("raw", &spec, "aa0f01").ok(), wanted);
        assert!(parse("raw", &spec, "aa0").is_err());
    }

    #[test]
    fn a_color_reads_with_or_without_the_hash() {
        assert_eq!(rgb("#FF8000").ok(), Some([255, 128, 0]));
        assert_eq!(rgb("ff8000").ok(), Some([255, 128, 0]));
        assert!(rgb("#FF80").is_err());
    }
}
