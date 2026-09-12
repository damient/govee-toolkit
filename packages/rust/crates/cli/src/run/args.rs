//! Values a person types, read against the type the device file declares.
//!
//! The device file declares the type of every argument, so nothing here
//! guesses one: `send` looks the argument up first, and this module reads the
//! text under that type. The range stays the codec's to check.

use govee_toolkit::codec::{ArgSpec, ArgValue};
use govee_toolkit::stream::Resolution;
use tokio::io::{AsyncBufReadExt, BufReader};

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

/// Read a color list: one `#RRGGBB` for every zone, or one per zone.
///
/// # Errors
///
/// [`Failure::usage`] for an item that is not a color.
pub(super) fn colors(text: &str) -> Result<Vec<[u8; 3]>, Failure> {
    list(text).map(rgb).collect()
}

/// The same list, read from one line of stdin where the text is `-`.
///
/// One line is one paint, so a list of 42 colors reaches the device from a
/// file or a pipe rather than from the command line.
///
/// # Errors
///
/// [`Failure::usage`] for an item that is not a color, and
/// [`Failure::internal`] where stdin cannot be read.
pub(super) async fn colors_or_stdin(text: &str) -> Result<Vec<[u8; 3]>, Failure> {
    if text.trim() != "-" {
        return colors(text);
    }
    let line = BufReader::new(tokio::io::stdin())
        .lines()
        .next_line()
        .await
        .map_err(|e| Failure::internal(e.to_string()))?
        .unwrap_or_default();
    colors(&line)
}

/// Read how many zones a frame states: `app`, `native`, or a count.
///
/// # Errors
///
/// [`Failure::usage`] for anything else.
pub(super) fn resolution(text: &str) -> Result<Resolution, Failure> {
    match text {
        "app" => Ok(Resolution::App),
        "native" => Ok(Resolution::Native),
        other => other.parse::<u16>().map(Resolution::Exact).map_err(|_| {
            Failure::usage(format!(
                "`{other}` is not a zone count; write `app`, `native`, or a number"
            ))
        }),
    }
}

/// The items of a list, each trimmed. A comma or a space separates them, and
/// an empty list has no items.
pub(super) fn list(text: &str) -> impl Iterator<Item = &str> {
    text.split([',', ' ', '\t'])
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

    #[test]
    fn a_color_list_reads_by_comma_or_by_space() {
        let wanted = Some(vec![[255, 0, 0], [0, 255, 0]]);
        assert_eq!(colors("#ff0000,#00ff00").ok(), wanted);
        assert_eq!(colors("ff0000 00ff00").ok(), wanted);
        assert_eq!(colors("#ff0000").ok(), Some(vec![[255, 0, 0]]));
        assert!(colors("#ff0000,green").is_err());
    }

    #[test]
    fn a_resolution_reads_by_name_or_by_number() {
        assert_eq!(resolution("app").ok(), Some(Resolution::App));
        assert_eq!(resolution("native").ok(), Some(Resolution::Native));
        assert_eq!(resolution("30").ok(), Some(Resolution::Exact(30)));
        assert!(resolution("many").is_err());
    }
}
