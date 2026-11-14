//! What crosses between a Python value and a core value.
//!
//! A Python value becomes an `input::Value` here, and the core reads it. Every
//! shape rule, default and refusal is the core's.

use govee_toolkit::codec::Supplied;
use govee_toolkit::input::{self, Refused, Value};
use govee_toolkit::{Mode, Painting, Style};
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyByteArray, PyBytes, PyDict, PyFloat, PyInt, PyList, PySequence, PyString, PyTuple,
};
use serde::Serialize;

use crate::errors::value_error;

/// Raise a core refusal as `ValueError`.
pub(crate) fn read<T>(result: input::Result<T>) -> PyResult<T> {
    result.map_err(|Refused(message)| value_error(message))
}

/// Read a Python value as the shapes every host shares. `bool` comes before
/// `int`, since `bool` subclasses `int`.
pub(crate) fn value(object: &Bound<'_, PyAny>) -> PyResult<Value> {
    if object.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(flag) = object.cast::<PyBool>() {
        return Ok(Value::Bool(flag.is_true()));
    }
    if object.is_instance_of::<PyInt>() {
        return whole(object);
    }
    if let Ok(number) = object.cast::<PyFloat>() {
        return Ok(Value::Float(number.value()));
    }
    if let Ok(text) = object.cast::<PyString>() {
        return Ok(Value::Text(text.to_str()?.to_owned()));
    }
    if let Ok(bytes) = object.cast::<PyBytes>() {
        return Ok(Value::Bytes(bytes.as_bytes().to_vec()));
    }
    if let Ok(bytes) = object.cast::<PyByteArray>() {
        return Ok(Value::Bytes(bytes.to_vec()));
    }
    if let Ok(dict) = object.cast::<PyDict>() {
        return entries(dict).map(Value::Map);
    }
    if object.is_instance_of::<PyList>() || object.is_instance_of::<PyTuple>() {
        return list(object);
    }
    if let Ok(sequence) = object.cast::<PySequence>() {
        return list(sequence.as_any());
    }
    // An object with `__index__` or `__float__`, such as a NumPy scalar.
    if object.hasattr("__index__")? {
        return whole(object);
    }
    if let Ok(number) = object.extract::<f64>() {
        return Ok(Value::Float(number));
    }
    Ok(Value::Other)
}

fn whole(object: &Bound<'_, PyAny>) -> PyResult<Value> {
    object
        .extract::<i64>()
        .map(Value::Int)
        .map_err(|_| value_error(format!("{object} is outside the range of a 64-bit integer")))
}

fn list(object: &Bound<'_, PyAny>) -> PyResult<Value> {
    object
        .try_iter()?
        .map(|item| value(&item?))
        .collect::<PyResult<_>>()
        .map(Value::List)
}

fn entries(dict: &Bound<'_, PyDict>) -> PyResult<Vec<(String, Value)>> {
    dict.iter()
        .map(|(key, item)| {
            let key: String = key
                .extract()
                .map_err(|_| value_error(format!("the key {key} is no str")))?;
            Ok((key, value(&item)?))
        })
        .collect()
}

/// Read an optional value. `None`, passed or left out, reads as `Null`.
pub(crate) fn optional(object: Option<&Bound<'_, PyAny>>) -> PyResult<Value> {
    object.map_or(Ok(Value::Null), value)
}

/// A map of the keyword arguments a caller passed. A `None` is left out.
pub(crate) fn keywords(pairs: &[(&str, Option<&Bound<'_, PyAny>>)]) -> PyResult<Value> {
    let mut entries = Vec::new();
    for (key, object) in pairs {
        if let Some(object) = object.filter(|object| !object.is_none()) {
            entries.push(((*key).to_owned(), value(object)?));
        }
    }
    Ok(Value::Map(entries))
}

pub(crate) fn mode(name: &str) -> PyResult<Mode> {
    read(input::mode(name))
}

pub(crate) fn modes(names: Vec<String>) -> PyResult<Vec<Mode>> {
    names.into_iter().map(|name| mode(&name)).collect()
}

pub(crate) fn rgb(object: &Bound<'_, PyAny>) -> PyResult<[u8; 3]> {
    read(input::rgb(&value(object)?, Style::Python))
}

pub(crate) fn colors(object: &Bound<'_, PyAny>) -> PyResult<Vec<[u8; 3]>> {
    if let Ok(bytes) = object.cast::<PyBytes>() {
        return read(input::packed(bytes.as_bytes()));
    }
    read(input::colors(&value(object)?, Style::Python))
}

pub(crate) fn painting(
    colors: &Bound<'_, PyAny>,
    zones: Option<&Bound<'_, PyAny>>,
    resolution: Option<&Bound<'_, PyAny>>,
    gradient: bool,
) -> PyResult<Painting> {
    read(input::painting(
        self::colors(colors)?,
        &optional(zones)?,
        &optional(resolution)?,
        Some(gradient),
    ))
}

pub(crate) fn args(values: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, Supplied)>> {
    let values = match values {
        Some(dict) => Value::Map(entries(dict)?),
        None => Value::Null,
    };
    read(input::args(&values, Style::Python))
}

/// Hand a value the core serializes to Python, as dicts and lists.
pub(crate) fn to_py<T: Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    Ok(pythonize::pythonize(py, value)?.unbind())
}
