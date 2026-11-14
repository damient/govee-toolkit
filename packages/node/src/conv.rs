//! A JavaScript value, as the core's host-neutral [`Value`].
//!
//! The shape rules, the defaults and the refusals live in
//! `govee_toolkit::input`. This module keeps the checks that only JavaScript
//! needs: the safe integer range, and the bytes of a typed array.

use govee_toolkit::Style;
use govee_toolkit::codec::{Mode, Supplied};
use govee_toolkit::input::{self, Refused, Value};
use napi::bindgen_prelude::{Either, JsObjectValue, Object, Unknown};
use napi::{Env, JsValue, ValueType};
use serde::Serialize;

use crate::errors::value_error;

/// How the core spells a key and words a refusal for this binding.
pub(crate) const STYLE: Style = Style::Javascript;

/// The channels of a color, or of a paint: the bytes of a `Uint8Array`, or
/// any other value.
pub(crate) type Channels<'a> = Either<&'a [u8], Unknown<'a>>;

/// The largest whole number a JavaScript `number` carries exactly. Past it
/// the value the caller wrote and the value that arrives are two numbers.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Hand a refusal of the core to JavaScript as a `TypeError`.
pub(crate) fn read<T>(env: &Env, result: input::Result<T>) -> napi::Result<T> {
    result.map_err(|Refused(message)| value_error(env, message))
}

fn number(env: &Env, number: f64) -> napi::Result<Value> {
    if !number.is_finite() || number.fract() != 0.0 {
        return Ok(Value::Float(number));
    }
    if number.abs() > MAX_SAFE_INTEGER {
        return Err(value_error(
            env,
            format!("{number} is past the largest whole number JavaScript carries exactly"),
        ));
    }
    // Checked above: no fraction, and under 2^53.
    #[allow(clippy::cast_possible_truncation)]
    Ok(Value::Int(number as i64))
}

/// Read one value. A typed array read here crosses once per element: a
/// `Uint8Array` crosses once where [`child`] reads it through its owner.
pub(crate) fn value(env: &Env, value: &Unknown<'_>) -> napi::Result<Value> {
    known(env, value.get_type()?, value)
}

/// [`value`], where the caller already asked for the type. Each question
/// to the engine is one crossing, and a paint asks per zone and per channel.
fn known(env: &Env, kind: ValueType, value: &Unknown<'_>) -> napi::Result<Value> {
    Ok(match kind {
        ValueType::Undefined | ValueType::Null => Value::Null,
        ValueType::Boolean => Value::Bool(value.coerce_to_bool()?),
        ValueType::Number => number(env, value.coerce_to_number()?.get_double()?)?,
        ValueType::String => Value::Text(value.coerce_to_string()?.into_utf8()?.into_owned()?),
        ValueType::Object => object(env, &value.coerce_to_object()?)?,
        _ => Value::Other,
    })
}

/// `None` reads as `Null`.
pub(crate) fn optional(env: &Env, from: Option<&Unknown<'_>>) -> napi::Result<Value> {
    from.map_or(Ok(Value::Null), |from| value(env, from))
}

/// Read an object: a typed array, an array, or a map of keys. A `Buffer` is a
/// `Uint8Array`, so one question covers both.
fn object(env: &Env, from: &Object<'_>) -> napi::Result<Value> {
    if from.is_typedarray()? {
        return elements(env, from).map(Value::Bytes);
    }
    untyped(env, from)
}

/// [`object`], where the caller already knows that it is no typed array.
fn untyped(env: &Env, from: &Object<'_>) -> napi::Result<Value> {
    if from.is_array()? {
        let length = from.get_array_length()?;
        return (0..length)
            .map(|index| {
                let item = from.get_element::<Unknown<'_>>(index)?;
                child(env, &item, || from.get_element::<&[u8]>(index).ok())
            })
            .collect::<napi::Result<_>>()
            .map(Value::List);
    }
    Object::keys(from)?
        .into_iter()
        .map(|key| {
            let item = match from.get::<Unknown<'_>>(&key)? {
                Some(item) => child(env, &item, || from.get::<&[u8]>(&key).ok().flatten())?,
                None => Value::Null,
            };
            Ok((key, item))
        })
        .collect::<napi::Result<_>>()
        .map(Value::Map)
}

/// `typed` reads a `Uint8Array` in one crossing. Only the owner of the value
/// answers that read, so the caller passes it.
fn child<'a>(
    env: &Env,
    item: &Unknown<'_>,
    typed: impl FnOnce() -> Option<&'a [u8]>,
) -> napi::Result<Value> {
    let kind = item.get_type()?;
    if kind != ValueType::Object {
        return known(env, kind, item);
    }
    let from = item.coerce_to_object()?;
    if from.is_typedarray()? {
        return match typed() {
            Some(bytes) => Ok(Value::Bytes(bytes.to_vec())),
            None => elements(env, &from).map(Value::Bytes),
        };
    }
    untyped(env, &from)
}

/// A typed array of another element type than `Uint8Array` is no byte run,
/// so each element must be a byte.
fn elements(env: &Env, array: &Object<'_>) -> napi::Result<Vec<u8>> {
    let length = array.get::<u32>("length")?.unwrap_or(0);
    (0..length)
        .map(
            |index| match value(env, &array.get_element::<Unknown<'_>>(index)?)? {
                Value::Int(byte) => u8::try_from(byte)
                    .map_err(|_| value_error(env, format!("{byte} is outside a byte's 0-255"))),
                _ => Err(value_error(env, "a typed array carries whole numbers")),
            },
        )
        .collect()
}

/// Read an options object. `None` reads as `Null`.
pub(crate) fn options(env: &Env, from: Option<&Object<'_>>) -> napi::Result<Value> {
    from.map_or(Ok(Value::Null), |from| object(env, from))
}

pub(crate) fn channels(env: &Env, from: &Channels<'_>) -> napi::Result<Value> {
    match from {
        Either::A(bytes) => Ok(Value::Bytes(bytes.to_vec())),
        Either::B(other) => value(env, other),
    }
}

pub(crate) fn rgb(env: &Env, from: &Channels<'_>) -> napi::Result<[u8; 3]> {
    read(env, input::rgb(&channels(env, from)?, STYLE))
}

pub(crate) fn colors(env: &Env, from: &Channels<'_>) -> napi::Result<Vec<[u8; 3]>> {
    match from {
        Either::A(bytes) => read(env, input::packed(bytes)),
        Either::B(other) => read(env, input::colors(&value(env, other)?, STYLE)),
    }
}

/// The arguments of a command, by the names the device file gives them.
pub(crate) fn args(env: &Env, from: Option<&Object<'_>>) -> napi::Result<Vec<(String, Supplied)>> {
    read(env, input::args(&options(env, from)?, STYLE))
}

pub(crate) fn mode(env: &Env, name: &str) -> napi::Result<Mode> {
    read(env, input::mode(name))
}

pub(crate) fn modes(env: &Env, names: Vec<String>) -> napi::Result<Vec<Mode>> {
    names.iter().map(|name| mode(env, name)).collect()
}

pub(crate) fn option_modes<const N: usize>(
    env: &Env,
    options: Option<&Object<'_>>,
    keys: [&str; N],
) -> napi::Result<[Option<Mode>; N]> {
    read(
        env,
        input::modes(&self::options(env, options)?, "option", keys),
    )
}

/// Hand a value the core serializes to JavaScript, as objects and arrays.
pub(crate) fn to_js<T: Serialize>(env: &Env, value: &T) -> napi::Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| value_error(env, e.to_string()))
}
