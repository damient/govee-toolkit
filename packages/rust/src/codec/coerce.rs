//! Values read under the type a device file declares.
//!
//! A person types text on the command line and a binding hands over a native
//! value. Neither front end decides what an argument is: the declared
//! [`ArgSpec`] does, and this module is where the two meet. A front end that
//! guessed the type from what it received would send a zone bitmask for a
//! color, or text for a number, on a device file the caller never read.
//!
//! The range stays the encoder's to check. A value of the declared type and
//! the wrong size passes here and reaches [`Error::OutOfRange`].

use crate::codec::args::{self, ArgValue};
use crate::codec::catalog::ArgSpec;
use crate::codec::error::{Error, Result};

/// The name a type-mismatch error uses for a flat list of numbers. It is a
/// shape a caller supplies, and never a type a device file declares.
const INTS: &str = "a list of whole numbers";

/// A value as a caller supplied it, before the device file says what it is.
///
/// A front end reports the shape it received and nothing more. [`read`] turns
/// one into the [`ArgValue`] the declared type asks for, or reports why it
/// cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Supplied {
    /// One whole number.
    Int(i64),
    /// Text. Every declared type reads text — see [`read`] for the syntax.
    Text(String),
    /// Bytes to send as they are.
    Bytes(Vec<u8>),
    /// A flat list of whole numbers: zone indices, byte values, or the three
    /// channels of one color.
    Ints(Vec<i64>),
    /// A list of RGB triples.
    Colors(Vec<[u8; 3]>),
}

impl Supplied {
    /// The name error messages use for this shape.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => args::INT,
            Self::Text(_) => args::TEXT,
            Self::Bytes(_) => args::BYTES,
            Self::Ints(_) => INTS,
            Self::Colors(_) => args::RGB_LIST,
        }
    }
}

/// Read one supplied value under the type `spec` declares.
///
/// Text reads under every declared type: a whole number for `int`, `#RRGGBB`
/// colors for `rgb_list`, zero-based indices for `zones`, pairs of
/// hexadecimal digits for `bytes`, and itself for `string`. A comma, a space
/// or a tab separates the items of a list.
///
/// A native value reads under the type it can only mean: a flat list of three
/// numbers is one color under `rgb_list`, byte values under `bytes`, and zone
/// indices under `zones`.
///
/// # Errors
///
/// [`Error::ArgType`] if the shape cannot carry the declared type, and
/// [`Error::ArgSyntax`] if it carries it and does not read as one.
pub fn read(command: &str, arg: &str, spec: &ArgSpec, value: Supplied) -> Result<ArgValue> {
    match (spec, value) {
        (ArgSpec::Int { .. }, Supplied::Int(number)) => Ok(ArgValue::Int(number)),
        (ArgSpec::Int { .. }, Supplied::Text(text)) => text
            .trim()
            .parse::<i64>()
            .map(ArgValue::Int)
            .map_err(|_| syntax(command, arg, "a whole number", &text)),

        (ArgSpec::String { .. }, Supplied::Text(text)) => Ok(ArgValue::Text(text)),

        (ArgSpec::Bytes { .. }, Supplied::Bytes(raw)) => Ok(ArgValue::Bytes(raw)),
        (ArgSpec::Bytes { .. }, Supplied::Text(text)) => bytes(&text)
            .map(ArgValue::Bytes)
            .ok_or_else(|| syntax(command, arg, "pairs of hexadecimal digits", &text)),
        (ArgSpec::Bytes { .. }, Supplied::Ints(items)) => items
            .iter()
            .map(|item| {
                u8::try_from(*item)
                    .map_err(|_| syntax(command, arg, "a byte from 0 to 255", &item.to_string()))
            })
            .collect::<Result<Vec<u8>>>()
            .map(ArgValue::Bytes),

        (ArgSpec::Zones { .. }, Supplied::Int(index)) => {
            zone(command, arg, index).map(|index| ArgValue::Zones(vec![index]))
        }
        (ArgSpec::Zones { .. }, Supplied::Ints(items)) => items
            .iter()
            .map(|item| zone(command, arg, *item))
            .collect::<Result<Vec<u16>>>()
            .map(ArgValue::Zones),
        (ArgSpec::Zones { .. }, Supplied::Text(text)) => zones(&text)
            .map(ArgValue::Zones)
            .ok_or_else(|| syntax(command, arg, "zone indices, zero-based", &text)),

        (ArgSpec::RgbList { .. }, Supplied::Colors(list)) => Ok(ArgValue::Rgb(list)),
        (ArgSpec::RgbList { .. }, Supplied::Ints(channels)) => {
            triple(command, arg, &channels).map(|color| ArgValue::Rgb(vec![color]))
        }
        (ArgSpec::RgbList { .. }, Supplied::Text(text)) => colors(&text)
            .map(ArgValue::Rgb)
            .ok_or_else(|| syntax(command, arg, "colors, each written `#RRGGBB`", &text)),

        (spec, value) => Err(Error::ArgType {
            command: command.to_owned(),
            arg: arg.to_owned(),
            expected: spec.type_name(),
            got: value.type_name(),
        }),
    }
}

/// Read one `#RRGGBB` color. The `#` is optional.
#[must_use]
pub fn rgb(text: &str) -> Option<[u8; 3]> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    let [_, red, green, blue] = value.to_be_bytes();
    Some([red, green, blue])
}

/// Read a list of `#RRGGBB` colors. `None` if one item does not read.
#[must_use]
pub fn colors(text: &str) -> Option<Vec<[u8; 3]>> {
    list(text).map(rgb).collect()
}

/// Read a list of zone indices, zero-based. `None` if one item does not read.
#[must_use]
pub fn zones(text: &str) -> Option<Vec<u16>> {
    list(text).map(|item| item.parse::<u16>().ok()).collect()
}

/// Read pairs of hexadecimal digits. A space or a colon separates the pairs,
/// or nothing does. `None` on an odd count of digits or on a digit that is
/// not hexadecimal.
#[must_use]
pub fn bytes(text: &str) -> Option<Vec<u8>> {
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .collect();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let mut rest = digits.as_str();
    while !rest.is_empty() {
        let (pair, tail) = rest.split_at(2);
        out.push(u8::from_str_radix(pair, 16).ok()?);
        rest = tail;
    }
    Some(out)
}

/// Split a list. A comma, a space or a tab separates the items.
pub fn list(text: &str) -> impl Iterator<Item = &str> {
    text.split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

fn zone(command: &str, arg: &str, index: i64) -> Result<u16> {
    u16::try_from(index)
        .map_err(|_| syntax(command, arg, "a zone index, zero-based", &index.to_string()))
}

fn triple(command: &str, arg: &str, channels: &[i64]) -> Result<[u8; 3]> {
    let shown = || {
        channels
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let [red, green, blue] = channels else {
        return Err(syntax(
            command,
            arg,
            "one color, as three channels",
            &shown(),
        ));
    };
    let channel = |value: &i64| {
        u8::try_from(*value)
            .map_err(|_| syntax(command, arg, "a channel from 0 to 255", &value.to_string()))
    };
    Ok([channel(red)?, channel(green)?, channel(blue)?])
}

fn syntax(command: &str, arg: &str, expected: &str, got: &str) -> Error {
    Error::ArgSyntax {
        command: command.to_owned(),
        arg: arg.to_owned(),
        expected: expected.to_owned(),
        got: got.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int() -> ArgSpec {
        ArgSpec::Int {
            range: None,
            role: None,
        }
    }

    fn rgb_list() -> ArgSpec {
        ArgSpec::RgbList {
            max_len: None,
            role: None,
        }
    }

    fn zones_spec() -> ArgSpec {
        ArgSpec::Zones {
            count: None,
            role: None,
        }
    }

    fn bytes_spec() -> ArgSpec {
        ArgSpec::Bytes {
            max_len: None,
            role: None,
        }
    }

    fn string() -> ArgSpec {
        ArgSpec::String {
            max_len: None,
            role: None,
        }
    }

    fn read_it(spec: &ArgSpec, value: Supplied) -> Result<ArgValue> {
        read("command", "arg", spec, value)
    }

    #[test]
    fn a_flat_triple_is_one_color_and_not_three_zones() {
        assert_eq!(
            read_it(&rgb_list(), Supplied::Ints(vec![255, 0, 0])).ok(),
            Some(ArgValue::Rgb(vec![[255, 0, 0]]))
        );
        assert_eq!(
            read_it(&zones_spec(), Supplied::Ints(vec![255, 0, 0])).ok(),
            Some(ArgValue::Zones(vec![255, 0, 0]))
        );
    }

    #[test]
    fn a_list_of_numbers_is_bytes_where_the_file_declares_bytes() {
        assert_eq!(
            read_it(&bytes_spec(), Supplied::Ints(vec![0xAA, 0x0F])).ok(),
            Some(ArgValue::Bytes(vec![0xAA, 0x0F]))
        );
        assert!(read_it(&bytes_spec(), Supplied::Ints(vec![256])).is_err());
    }

    #[test]
    fn text_reads_under_every_declared_type() {
        assert_eq!(
            read_it(&int(), Supplied::Text("50".to_owned())).ok(),
            Some(ArgValue::Int(50))
        );
        assert_eq!(
            read_it(&string(), Supplied::Text("50".to_owned())).ok(),
            Some(ArgValue::Text("50".to_owned()))
        );
        assert_eq!(
            read_it(&rgb_list(), Supplied::Text("#ff0000 00ff00".to_owned())).ok(),
            Some(ArgValue::Rgb(vec![[255, 0, 0], [0, 255, 0]]))
        );
        assert_eq!(
            read_it(&zones_spec(), Supplied::Text("0, 2,3".to_owned())).ok(),
            Some(ArgValue::Zones(vec![0, 2, 3]))
        );
        assert_eq!(
            read_it(&bytes_spec(), Supplied::Text("aa:0f".to_owned())).ok(),
            Some(ArgValue::Bytes(vec![0xAA, 0x0F]))
        );
    }

    #[test]
    fn a_shape_the_declared_type_cannot_carry_is_refused() {
        let wrong = read_it(&string(), Supplied::Int(7));
        assert_eq!(wrong.err().map(|e| e.code()), Some("arg_type"));
        assert!(read_it(&int(), Supplied::Colors(vec![[1, 2, 3]])).is_err());
        assert!(read_it(&rgb_list(), Supplied::Bytes(vec![1])).is_err());
    }

    #[test]
    fn a_shape_that_does_not_read_is_refused() {
        let bad = read_it(&int(), Supplied::Text("bright".to_owned()));
        assert_eq!(bad.err().map(|e| e.code()), Some("arg_syntax"));
        assert!(read_it(&rgb_list(), Supplied::Ints(vec![255, 0])).is_err());
        assert!(read_it(&rgb_list(), Supplied::Ints(vec![300, 0, 0])).is_err());
        assert!(read_it(&zones_spec(), Supplied::Int(-1)).is_err());
    }

    #[test]
    fn a_range_belongs_to_the_encoder_and_is_not_checked_here() {
        assert_eq!(
            read_it(&int(), Supplied::Int(4_000)).ok(),
            Some(ArgValue::Int(4_000))
        );
    }
}
