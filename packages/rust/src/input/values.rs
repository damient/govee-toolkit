//! One value read as a color, a zone list, a resolution, a rate, a duration
//! or a command argument.

use std::time::Duration;

use super::{Refused, Result, Value};
use crate::codec::Mode;
use crate::codec::coerce::{self, Supplied};
use crate::stream::{Rate, Resolution};
use crate::summary::Style;
use crate::verbs::Painting;

/// Read a whole number. A boolean reads as `0` or `1`, and a float with no
/// fraction reads as the whole number it carries.
pub(crate) fn whole(value: &Value) -> Result<i64> {
    match value {
        Value::Int(number) => Ok(*number),
        Value::Bool(flag) => Ok(i64::from(*flag)),
        // `i64::MAX as f64` rounds up to 2^63, so the bound is exclusive.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        Value::Float(number)
            if number.is_finite() && number.fract() == 0.0 && number.abs() < i64::MAX as f64 =>
        {
            Ok(*number as i64)
        }
        Value::Float(number) => Err(Refused::new(format!("{number} is not a whole number"))),
        _ => Err(Refused::new("a whole number is expected")),
    }
}

fn ints(items: &[Value]) -> Result<Vec<i64>> {
    items.iter().map(whole).collect()
}

fn triple(channels: &[i64]) -> Result<[u8; 3]> {
    coerce::triple(channels).map_err(|refused| Refused::new(refused.to_string()))
}

/// Read one color: three channels from 0 to 255, or three bytes.
///
/// # Errors
///
/// [`Refused`] for any other shape, a count other than three, or a channel
/// outside 0-255.
pub fn rgb(value: &Value, style: Style) -> Result<[u8; 3]> {
    match value {
        Value::Bytes(bytes) => triple(&bytes.iter().copied().map(i64::from).collect::<Vec<_>>()),
        Value::List(items) => triple(&ints(items)?),
        _ => Err(Refused::new(match style {
            Style::Python => {
                "a color is three whole numbers from 0 to 255, such as (255, 0, 0), or three bytes"
            }
            _ => {
                "a color is three whole numbers from 0 to 255, such as [255, 0, 0], or three bytes"
            }
        })),
    }
}

fn is_number(value: &Value) -> bool {
    matches!(value, Value::Int(_) | Value::Float(_) | Value::Bool(_))
}

/// Read a paint: one color fills every zone, a list states one color per
/// zone, and a byte run carries three bytes for every zone.
///
/// # Errors
///
/// [`Refused`] for any other shape, or a byte run that is no multiple of 3.
pub fn colors(value: &Value, style: Style) -> Result<Vec<[u8; 3]>> {
    match value {
        Value::Bytes(bytes) => packed(bytes),
        Value::List(items) if items.first().is_some_and(is_number) => {
            Ok(vec![triple(&ints(items)?)?])
        }
        Value::List(items) => items.iter().map(|item| rgb(item, style)).collect(),
        _ => Err(Refused::new(match style {
            Style::Python => "a paint is one color, a list of colors, or bytes",
            _ => "a paint is one color, an array of colors, or bytes",
        })),
    }
}

/// Read a paint from a byte run, three bytes for every zone. A binding that
/// holds the bytes calls this and builds no [`Value`].
///
/// # Errors
///
/// [`Refused`] for a byte run that is no multiple of 3.
pub fn packed(bytes: &[u8]) -> Result<Vec<[u8; 3]>> {
    let (zones, rest) = bytes.as_chunks::<3>();
    if !rest.is_empty() {
        return Err(Refused::new(format!(
            "a paint carries 3 bytes for every zone, and {} bytes is no multiple of 3",
            bytes.len()
        )));
    }
    Ok(zones.to_vec())
}

/// Read the arguments of `segment()` around colors already read. `Null`
/// reads as absent: every zone, the default resolution, no gradient.
///
/// # Errors
///
/// [`Refused`] for zones or a resolution of the wrong shape.
pub fn painting(
    colors: Vec<[u8; 3]>,
    zones: &Value,
    resolution: &Value,
    gradient: Option<bool>,
) -> Result<Painting> {
    Ok(Painting {
        zones: match zones {
            Value::Null => None,
            zones => Some(self::zones(zones)?),
        },
        colors,
        resolution: self::resolution(resolution)?,
        gradient: gradient.unwrap_or(false),
    })
}

/// Read zone indices, zero-based.
///
/// # Errors
///
/// [`Refused`] for any shape but a list, or an index outside 0-65535.
pub fn zones(value: &Value) -> Result<Vec<u16>> {
    let Value::List(items) = value else {
        return Err(Refused::new("zones are a list of whole numbers"));
    };
    items
        .iter()
        .map(|item| {
            let index = whole(item)?;
            u16::try_from(index)
                .map_err(|_| Refused::new(format!("{index} is outside a zone index's 0-65535")))
        })
        .collect()
}

/// Read a resolution: a name, or a zone count. `Null` reads as the default.
///
/// # Errors
///
/// [`Refused`] for an unknown name, a count outside 0-65535, or any other
/// shape.
pub fn resolution(value: &Value) -> Result<Resolution> {
    match value {
        Value::Null => Ok(Resolution::default()),
        Value::Text(name) => name
            .parse()
            .map_err(|e: crate::stream::ParseError| Refused::new(e.to_string())),
        Value::Int(_) | Value::Float(_) => {
            let count = whole(value)?;
            u16::try_from(count)
                .map(Resolution::Exact)
                .map_err(|_| Refused::new("a zone count is a whole number from 0 to 65535"))
        }
        _ => Err(Refused::new(
            "a resolution is a zone count, or \"app\", \"native\" or \"groups\"",
        )),
    }
}

/// Read a rate: `"measured"`, or a number of hertz. `Null` reads as the
/// default. The range stays the stream's to refuse.
///
/// # Errors
///
/// [`Refused`] for an unknown name or any other shape. A boolean is no rate.
pub fn rate(value: &Value) -> Result<Rate> {
    match value {
        Value::Null => Ok(Rate::default()),
        Value::Text(name) => name
            .parse()
            .map_err(|e: crate::stream::ParseError| Refused::new(e.to_string())),
        #[allow(clippy::cast_precision_loss)]
        Value::Int(hz) => Ok(Rate::Fixed(*hz as f64)),
        Value::Float(hz) => Ok(Rate::Fixed(*hz)),
        _ => Err(Refused::new("a rate is \"measured\" or a number of hertz")),
    }
}

/// Read a duration, in seconds. `key` names it in the refusal.
///
/// # Errors
///
/// [`Refused`] for a negative or non-finite number, or any other shape.
pub(crate) fn seconds(key: &str, value: &Value) -> Result<Duration> {
    #[allow(clippy::cast_precision_loss)]
    let number = match value {
        Value::Int(number) => *number as f64,
        Value::Float(number) => *number,
        _ => {
            return Err(Refused::new(format!(
                "`{key}` is a number of seconds, 0 or more"
            )));
        }
    };
    Duration::try_from_secs_f64(number).map_err(|_| {
        Refused::new(format!(
            "`{key}` is {number}, and a duration is a number of seconds, 0 or more"
        ))
    })
}

/// Read a mode name.
///
/// # Errors
///
/// [`Refused`] where the text names no mode.
pub fn mode(name: &str) -> Result<Mode> {
    name.parse()
        .map_err(|e: crate::codec::UnknownMode| Refused::new(e.to_string()))
}

/// Read the shape of one command argument, never its type: the device file
/// declares the type, and [`coerce::read`] reads the value under it.
///
/// # Errors
///
/// [`Refused`] for a shape that no argument type takes.
pub(crate) fn supplied(value: &Value, style: Style) -> Result<Supplied> {
    match value {
        Value::Bool(_) | Value::Int(_) | Value::Float(_) => Ok(Supplied::Int(whole(value)?)),
        Value::Text(text) => Ok(Supplied::Text(text.clone())),
        Value::Bytes(bytes) => Ok(Supplied::Bytes(bytes.clone())),
        Value::List(items) if matches!(items.first(), Some(Value::List(_) | Value::Bytes(_))) => {
            Ok(Supplied::Colors(
                items
                    .iter()
                    .map(|item| rgb(item, style))
                    .collect::<Result<_>>()?,
            ))
        }
        Value::List(items) => Ok(Supplied::Ints(ints(items)?)),
        _ => Err(Refused::new(match style {
            Style::Python => {
                "an argument is an int, a bool, a str, bytes, a list of colors or a list of whole numbers"
            }
            _ => {
                "an argument is a number, a boolean, a string, a Uint8Array, an array of colors or an array of whole numbers"
            }
        })),
    }
}

/// Read the arguments of a command, by the names the device file gives
/// them. `Null` reads as no argument.
///
/// # Errors
///
/// [`Refused`] for an argument of a shape that no argument type takes, or
/// where the value is no map.
pub fn args(values: &Value, style: Style) -> Result<Vec<(String, Supplied)>> {
    match values {
        Value::Null => Ok(Vec::new()),
        Value::Map(entries) => entries
            .iter()
            .map(|(name, value)| Ok((name.clone(), supplied(value, style)?)))
            .collect(),
        _ => Err(Refused::new("the arguments are a map of names to values")),
    }
}
