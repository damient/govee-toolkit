//! A binding turns a host value into a [`Value`] and stops there. Every shape
//! rule, default and refusal lives here, so that each language refuses with
//! the same words.

mod options;
mod values;

pub use options::{modes, names, verbs, walk};
pub use values::{args, colors, mode, packed, painting, rate, resolution, rgb, zones};

use crate::summary::Style;

/// A host value, in the shapes that every host language shares.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, `undefined` or `None`.
    Null,
    /// A boolean. Reads as `0` or `1` where a whole number is expected.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number with a fraction, or one the host carries as a float.
    Float(f64),
    /// Text.
    Text(String),
    /// A byte run: a `Uint8Array` or Python `bytes`.
    Bytes(Vec<u8>),
    /// An array, a list or a tuple.
    List(Vec<Value>),
    /// An object or a dict, in the order the host gives the keys.
    Map(Vec<(String, Value)>),
    /// A value of no shape above.
    Other,
}

/// A value that the core refuses. Nothing was sent. A binding raises it as
/// `TypeError` in JavaScript and `ValueError` in Python.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Refused(pub String);

impl Refused {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// The result of a read in this module.
pub type Result<T> = std::result::Result<T, Refused>;

impl Value {
    /// The value under `key`. `Null` reads as absent.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        let Self::Map(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
            .filter(|value| !matches!(value, Self::Null))
    }

    /// Refuse a key outside `keys`.
    ///
    /// # Errors
    ///
    /// [`Refused`] for the first unknown key, or where the value is no map.
    pub fn known_keys(&self, what: &str, keys: &[&str]) -> Result<()> {
        let Self::Map(entries) = self else {
            return Err(Refused::new(format!(
                "the {what} options are a map of keys"
            )));
        };
        match entries
            .iter()
            .find(|(key, _)| !keys.contains(&key.as_str()))
        {
            Some((key, _)) => Err(Refused::new(format!(
                "`{key}` is no {what} key; the keys are {}",
                keys.join(", ")
            ))),
            None => Ok(()),
        }
    }
}

/// `color_temp` in Python, `colorTemp` in JavaScript.
pub(crate) fn key(style: Style, snake: &str) -> String {
    if style == Style::Python {
        return snake.to_owned();
    }
    let mut camel = String::with_capacity(snake.len());
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            camel.extend(c.to_uppercase());
            upper = false;
        } else {
            camel.push(c);
        }
    }
    camel
}

#[cfg(test)]
mod tests;
