//! What crosses between a Python value and a core value.

use govee_toolkit::codec::{ArgValue, Args};
use govee_toolkit::{Mode, Rate, Resolution};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFloat, PyInt, PyString};
use serde::Serialize;

use crate::errors::value_error;

/// Read a mode by the name the device files and the configuration use.
pub(crate) fn mode(name: &str) -> PyResult<Mode> {
    Mode::ALL
        .into_iter()
        .find(|mode| mode.to_string() == name)
        .ok_or_else(|| {
            value_error(format!(
                "`{name}` is not a mode; the modes are {}",
                mode_names().join(", ")
            ))
        })
}

/// Every mode the core knows, by name.
pub(crate) fn mode_names() -> Vec<String> {
    Mode::ALL.iter().map(ToString::to_string).collect()
}

/// Read the modes of a list.
pub(crate) fn modes(names: Vec<String>) -> PyResult<Vec<Mode>> {
    names.into_iter().map(|name| mode(&name)).collect()
}

/// Read one RGB triple. Every channel is a whole number from 0 to 255.
pub(crate) fn rgb(value: &Bound<'_, PyAny>) -> PyResult<[u8; 3]> {
    let channels: Vec<i64> = value.extract().map_err(|_| {
        value_error("a color is three whole numbers from 0 to 255, such as (255, 0, 0)")
    })?;
    let [r, g, b] = channels.as_slice() else {
        return Err(value_error(format!(
            "a color carries 3 channels, not {}",
            channels.len()
        )));
    };
    let channel = |v: &i64| {
        u8::try_from(*v).map_err(|_| value_error(format!("{v} is outside a color channel's 0-255")))
    };
    Ok([channel(r)?, channel(g)?, channel(b)?])
}

/// Read a color, or a list of them. One color is not wrapped by the caller.
pub(crate) fn colors(value: &Bound<'_, PyAny>) -> PyResult<Vec<[u8; 3]>> {
    if let Ok(one) = rgb(value) {
        return Ok(vec![one]);
    }
    value
        .try_iter()?
        .map(|item| rgb(&item?))
        .collect::<PyResult<Vec<_>>>()
}

/// Read one argument value.
///
/// The Python type decides the argument type: `bytes` are sent as they are, a
/// `str` is text, a `bool` and an `int` are whole numbers, a list of triples
/// is a list of colors, and a list of whole numbers is a list of zone
/// indices.
pub(crate) fn arg_value(value: &Bound<'_, PyAny>) -> PyResult<ArgValue> {
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(ArgValue::Bytes(bytes.as_bytes().to_vec()));
    }
    if let Ok(text) = value.cast::<PyString>() {
        return Ok(ArgValue::Text(text.extract()?));
    }
    if let Ok(flag) = value.extract::<bool>() {
        return Ok(ArgValue::Int(i64::from(flag)));
    }
    if let Ok(number) = value.extract::<i64>() {
        return Ok(ArgValue::Int(number));
    }
    if value.extract::<Vec<Vec<i64>>>().is_ok() {
        return Ok(ArgValue::Rgb(colors(value)?));
    }
    if let Ok(zones) = value.extract::<Vec<u16>>() {
        return Ok(ArgValue::Zones(zones));
    }
    Err(value_error(
        "an argument is an int, a bool, a str, bytes, a list of colors or a list of zone indices",
    ))
}

/// Read the arguments of a call.
pub(crate) fn args(values: Option<&Bound<'_, PyDict>>) -> PyResult<Args> {
    let mut built = Args::new();
    let Some(values) = values else {
        return Ok(built);
    };
    for (name, value) in values {
        built.insert(name.extract::<String>()?, arg_value(&value)?);
    }
    Ok(built)
}

/// Read how many zones a paint or a stream states: `"app"`, `"native"`, or a
/// count.
pub(crate) fn resolution(value: &Bound<'_, PyAny>) -> PyResult<Resolution> {
    if let Ok(name) = value.cast::<PyString>() {
        return match name.extract::<String>()?.as_str() {
            "app" => Ok(Resolution::App),
            "native" => Ok(Resolution::Native),
            other => Err(value_error(format!(
                "`{other}` is not a resolution; write \"app\", \"native\" or a zone count"
            ))),
        };
    }
    Ok(Resolution::Exact(value.extract::<u16>().map_err(|_| {
        value_error("a zone count is a whole number from 0 to 65535")
    })?))
}

/// Read how fast a stream sends: `"measured"`, or a rate in hertz.
pub(crate) fn rate(value: &Bound<'_, PyAny>) -> PyResult<Rate> {
    if let Ok(name) = value.cast::<PyString>() {
        return match name.extract::<String>()?.as_str() {
            "measured" => Ok(Rate::Measured),
            other => Err(value_error(format!(
                "`{other}` is not a rate; write \"measured\" or a number of hertz"
            ))),
        };
    }
    if value.is_instance_of::<PyFloat>() || value.is_instance_of::<PyInt>() {
        return Ok(Rate::Fixed(value.extract::<f64>()?));
    }
    Err(value_error("a rate is \"measured\" or a number of hertz"))
}

/// Read a resolution, or the default one: what the phone controller exposes.
pub(crate) fn resolution_or_default(value: Option<&Bound<'_, PyAny>>) -> PyResult<Resolution> {
    value.map_or(Ok(Resolution::App), resolution)
}

/// Read a rate, or the default one: what the device file measured.
pub(crate) fn rate_or_default(value: Option<&Bound<'_, PyAny>>) -> PyResult<Rate> {
    value.map_or(Ok(Rate::Measured), rate)
}

/// Hand a value the core serializes to Python, as dicts and lists.
pub(crate) fn to_py<T: Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    Ok(pythonize::pythonize(py, value)?.unbind())
}
