//! What crosses between a Python value and a core value.

use govee_toolkit::codec::{Supplied, UnknownMode};
use govee_toolkit::{Mode, ParseError, Rate, Resolution};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFloat, PyInt, PyString};
use serde::Serialize;

use crate::errors::value_error;

pub(crate) fn mode(name: &str) -> PyResult<Mode> {
    name.parse()
        .map_err(|e: UnknownMode| value_error(e.to_string()))
}

pub(crate) fn modes(names: Vec<String>) -> PyResult<Vec<Mode>> {
    names.into_iter().map(|name| mode(&name)).collect()
}

pub(crate) fn triple(channels: &[i64]) -> PyResult<[u8; 3]> {
    let [r, g, b] = channels else {
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

pub(crate) fn rgb(value: &Bound<'_, PyAny>) -> PyResult<[u8; 3]> {
    let channels: Vec<i64> = value.extract().map_err(|_| {
        value_error("a color is three whole numbers from 0 to 255, such as (255, 0, 0)")
    })?;
    triple(&channels)
}

/// Read a color, or a list of them. One color needs no wrapping list.
pub(crate) fn colors(value: &Bound<'_, PyAny>) -> PyResult<Vec<[u8; 3]>> {
    if let Ok(one) = rgb(value) {
        return Ok(vec![one]);
    }
    value
        .try_iter()?
        .map(|item| rgb(&item?))
        .collect::<PyResult<Vec<_>>>()
}

/// Read the shape of one argument value, never its type: the device file
/// declares the type, and `codec::coerce` reads the value under it. A `bool`
/// is a whole number, as it is in Python.
pub(crate) fn supplied(value: &Bound<'_, PyAny>) -> PyResult<Supplied> {
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(Supplied::Bytes(bytes.as_bytes().to_vec()));
    }
    if let Ok(text) = value.cast::<PyString>() {
        return Ok(Supplied::Text(text.extract()?));
    }
    if let Ok(flag) = value.extract::<bool>() {
        return Ok(Supplied::Int(i64::from(flag)));
    }
    if let Ok(number) = value.extract::<i64>() {
        return Ok(Supplied::Int(number));
    }
    if let Ok(rows) = value.extract::<Vec<Vec<i64>>>() {
        return Ok(Supplied::Colors(
            rows.iter()
                .map(|row| triple(row))
                .collect::<PyResult<Vec<_>>>()?,
        ));
    }
    if let Ok(numbers) = value.extract::<Vec<i64>>() {
        return Ok(Supplied::Ints(numbers));
    }
    Err(value_error(
        "an argument is an int, a bool, a str, bytes, a list of colors or a list of whole numbers",
    ))
}

pub(crate) fn args(values: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, Supplied)>> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    values
        .iter()
        .map(|(name, value)| Ok((name.extract::<String>()?, supplied(&value)?)))
        .collect()
}

pub(crate) fn resolution(value: &Bound<'_, PyAny>) -> PyResult<Resolution> {
    if let Ok(name) = value.cast::<PyString>() {
        return name
            .extract::<String>()?
            .parse()
            .map_err(|e: ParseError| value_error(e.to_string()));
    }
    Ok(Resolution::Exact(value.extract::<u16>().map_err(|_| {
        value_error("a zone count is a whole number from 0 to 65535")
    })?))
}

pub(crate) fn rate(value: &Bound<'_, PyAny>) -> PyResult<Rate> {
    if let Ok(name) = value.cast::<PyString>() {
        return name
            .extract::<String>()?
            .parse()
            .map_err(|e: ParseError| value_error(e.to_string()));
    }
    if value.is_instance_of::<PyFloat>() || value.is_instance_of::<PyInt>() {
        return Ok(Rate::Fixed(value.extract::<f64>()?));
    }
    Err(value_error("a rate is \"measured\" or a number of hertz"))
}

pub(crate) fn resolution_or_default(value: Option<&Bound<'_, PyAny>>) -> PyResult<Resolution> {
    value.map_or_else(|| Ok(Resolution::default()), resolution)
}

pub(crate) fn rate_or_default(value: Option<&Bound<'_, PyAny>>) -> PyResult<Rate> {
    value.map_or_else(|| Ok(Rate::default()), rate)
}

/// Hand a value the core serializes to Python, as dicts and lists.
pub(crate) fn to_py<T: Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    Ok(pythonize::pythonize(py, value)?.unbind())
}
