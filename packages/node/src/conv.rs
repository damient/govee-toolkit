//! What crosses between a JavaScript value and a core value.

use govee_toolkit::codec::{Supplied, UnknownMode};
use govee_toolkit::{Mode, ParseError, Rate, Resolution};
use napi::bindgen_prelude::{JsObjectValue, Object, Unknown};
use napi::{Env, JsValue, ValueType};
use serde::Serialize;

use crate::errors::value_error;

pub(crate) fn mode(env: &Env, name: &str) -> napi::Result<Mode> {
    name.parse()
        .map_err(|e: UnknownMode| value_error(env, e.to_string()))
}

pub(crate) fn modes(env: &Env, names: Vec<String>) -> napi::Result<Vec<Mode>> {
    names.iter().map(|name| mode(env, name)).collect()
}

/// The largest whole number a JavaScript `number` carries exactly. Past it
/// the value the caller wrote and the value that arrives are two numbers.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Read a number that carries no fraction. A level, a zone and a channel are
/// all whole numbers, and JavaScript writes every one of them as a `number`.
fn whole(env: &Env, value: &Unknown<'_>) -> napi::Result<i64> {
    let number = value.coerce_to_number()?.get_double()?;
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(value_error(env, format!("{number} is not a whole number")));
    }
    if number.abs() > MAX_SAFE_INTEGER {
        return Err(value_error(
            env,
            format!("{number} is past the largest whole number JavaScript carries exactly"),
        ));
    }
    // The fraction is gone and the bound is under 2^53, so the cast keeps the
    // value the caller wrote.
    #[allow(clippy::cast_possible_truncation)]
    Ok(number as i64)
}

/// Read an array of whole numbers. Anything that is not an array is refused
/// by the caller, which states what it wanted.
fn ints(env: &Env, array: &Object<'_>) -> napi::Result<Vec<i64>> {
    let length = array.get_array_length()?;
    (0..length)
        .map(|index| whole(env, &array.get_element::<Unknown<'_>>(index)?))
        .collect()
}

pub(crate) fn triple(env: &Env, channels: &[i64]) -> napi::Result<[u8; 3]> {
    let [r, g, b] = channels else {
        return Err(value_error(
            env,
            format!("a color carries 3 channels, not {}", channels.len()),
        ));
    };
    let channel = |v: &i64| {
        u8::try_from(*v)
            .map_err(|_| value_error(env, format!("{v} is outside a color channel's 0-255")))
    };
    Ok([channel(r)?, channel(g)?, channel(b)?])
}

pub(crate) fn rgb(env: &Env, value: &Unknown<'_>) -> napi::Result<[u8; 3]> {
    let refused = || {
        value_error(
            env,
            "a color is three whole numbers from 0 to 255, such as [255, 0, 0]",
        )
    };
    if value.get_type()? != ValueType::Object {
        return Err(refused());
    }
    let array = value.coerce_to_object()?;
    if !array.is_array()? {
        return Err(refused());
    }
    triple(env, &ints(env, &array)?)
}

/// Read a color, or an array of them. One color needs no wrapping array.
pub(crate) fn colors(env: &Env, value: &Unknown<'_>) -> napi::Result<Vec<[u8; 3]>> {
    if let Ok(one) = rgb(env, value) {
        return Ok(vec![one]);
    }
    let array = value.coerce_to_object()?;
    if !array.is_array()? {
        return Err(value_error(
            env,
            "a paint is one color, or an array of colors",
        ));
    }
    (0..array.get_array_length()?)
        .map(|index| rgb(env, &array.get_element::<Unknown<'_>>(index)?))
        .collect()
}

/// Read the bytes of a `Buffer` or a typed array, element by element. A
/// typed array is no array, so the count comes off `length`.
fn bytes(env: &Env, array: &Object<'_>) -> napi::Result<Vec<u8>> {
    let length = array.get::<u32>("length")?.unwrap_or(0);
    (0..length)
        .map(|index| {
            let value = whole(env, &array.get_element::<Unknown<'_>>(index)?)?;
            u8::try_from(value)
                .map_err(|_| value_error(env, format!("{value} is outside a byte's 0-255")))
        })
        .collect()
}

/// Read the shape of one argument value, never its type: the device file
/// declares the type, and `codec::coerce` reads the value under it. A boolean
/// is a whole number, as it is in JavaScript.
pub(crate) fn supplied(env: &Env, value: &Unknown<'_>) -> napi::Result<Supplied> {
    let refused = || {
        value_error(
            env,
            "an argument is a number, a boolean, a string, a Uint8Array, an array of colors or an array of whole numbers",
        )
    };
    match value.get_type()? {
        ValueType::Boolean => Ok(Supplied::Int(i64::from(value.coerce_to_bool()?))),
        ValueType::Number => Ok(Supplied::Int(whole(env, value)?)),
        ValueType::String => Ok(Supplied::Text(
            value.coerce_to_string()?.into_utf8()?.into_owned()?,
        )),
        ValueType::Object => {
            let object = value.coerce_to_object()?;
            if object.is_buffer()? || object.is_typedarray()? {
                return Ok(Supplied::Bytes(bytes(env, &object)?));
            }
            if !object.is_array()? {
                return Err(refused());
            }
            if object.get_array_length()? > 0
                && object.get_element::<Unknown<'_>>(0)?.get_type()? == ValueType::Object
            {
                return Ok(Supplied::Colors(colors(env, value)?));
            }
            Ok(Supplied::Ints(ints(env, &object)?))
        }
        _ => Err(refused()),
    }
}

/// Read the arguments an entry declares, by the names the device file gives
/// them.
pub(crate) fn args(env: &Env, values: Option<Object<'_>>) -> napi::Result<Vec<(String, Supplied)>> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    Object::keys(&values)?
        .into_iter()
        .filter_map(|name| {
            let read = values
                .get::<Unknown<'_>>(&name)
                .transpose()?
                .and_then(|value| supplied(env, &value))
                .map(|value| (name, value));
            Some(read)
        })
        .collect()
}

pub(crate) fn resolution(env: &Env, value: &Unknown<'_>) -> napi::Result<Resolution> {
    if value.get_type()? == ValueType::String {
        let name = value.coerce_to_string()?.into_utf8()?.into_owned()?;
        return name
            .parse()
            .map_err(|e: ParseError| value_error(env, e.to_string()));
    }
    let count = whole(env, value)?;
    Ok(Resolution::Exact(u16::try_from(count).map_err(|_| {
        value_error(env, "a zone count is a whole number from 0 to 65535")
    })?))
}

pub(crate) fn rate(env: &Env, value: &Unknown<'_>) -> napi::Result<Rate> {
    match value.get_type()? {
        ValueType::String => {
            let name = value.coerce_to_string()?.into_utf8()?.into_owned()?;
            name.parse()
                .map_err(|e: ParseError| value_error(env, e.to_string()))
        }
        ValueType::Number => Ok(Rate::Fixed(value.coerce_to_number()?.get_double()?)),
        _ => Err(value_error(
            env,
            "a rate is \"measured\" or a number of hertz",
        )),
    }
}

pub(crate) fn resolution_or_default(
    env: &Env,
    value: Option<&Unknown<'_>>,
) -> napi::Result<Resolution> {
    value.map_or_else(|| Ok(Resolution::default()), |v| resolution(env, v))
}

pub(crate) fn rate_or_default(env: &Env, value: Option<&Unknown<'_>>) -> napi::Result<Rate> {
    value.map_or_else(|| Ok(Rate::default()), |v| rate(env, v))
}

/// Hand a value the core serializes to JavaScript, as objects and arrays.
pub(crate) fn to_js<T: Serialize>(env: &Env, value: &T) -> napi::Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| value_error(env, e.to_string()))
}
