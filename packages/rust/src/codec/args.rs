//! Argument values supplied at call time.

use std::collections::BTreeMap;

/// The name an integer goes by in a type-mismatch error.
pub(crate) const INT: &str = "an integer";
/// The name an RGB list goes by in a type-mismatch error.
pub(crate) const RGB_LIST: &str = "a list of RGB triples";
/// The name a string goes by in a type-mismatch error.
pub(crate) const TEXT: &str = "a string";
/// The name a zone list goes by in a type-mismatch error.
pub(crate) const ZONES: &str = "a list of zone indices";
/// The name an opaque blob goes by in a type-mismatch error.
pub(crate) const BYTES: &str = "a byte string";

/// A value for one declared argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgValue {
    /// A whole number.
    Int(i64),
    /// A list of RGB triples.
    Rgb(Vec<[u8; 3]>),
    /// Text, emitted as UTF-8 behind a length prefix.
    Text(String),
    /// Zone indices, zero-based, emitted as a bitmask.
    Zones(Vec<u16>),
    /// Bytes the caller supplies and this crate does not interpret.
    Bytes(Vec<u8>),
}

impl ArgValue {
    /// The name used in error messages.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => INT,
            Self::Rgb(_) => RGB_LIST,
            Self::Text(_) => TEXT,
            Self::Zones(_) => ZONES,
            Self::Bytes(_) => BYTES,
        }
    }
}

/// The arguments passed to one command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args(BTreeMap<String, ArgValue>);

impl Args {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an integer argument.
    #[must_use]
    pub fn int(mut self, name: impl Into<String>, value: i64) -> Self {
        self.0.insert(name.into(), ArgValue::Int(value));
        self
    }

    /// Add a list of RGB triples.
    #[must_use]
    pub fn rgb(mut self, name: impl Into<String>, colors: impl Into<Vec<[u8; 3]>>) -> Self {
        self.0.insert(name.into(), ArgValue::Rgb(colors.into()));
        self
    }

    /// Add a string argument.
    ///
    /// It reaches the device as UTF-8 behind the length prefix its field
    /// declares. Nothing is escaped or transliterated: what the caller passes
    /// is what the firmware is asked to match.
    #[must_use]
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(name.into(), ArgValue::Text(value.into()));
        self
    }

    /// Add a list of zone indices, zero-based.
    ///
    /// Emitted as a bitmask, least significant bit first. Which zones exist is
    /// the device file's business, not this type's.
    #[must_use]
    pub fn zones(mut self, name: impl Into<String>, zones: impl Into<Vec<u16>>) -> Self {
        self.0.insert(name.into(), ArgValue::Zones(zones.into()));
        self
    }

    /// Add bytes to be sent as they are.
    #[must_use]
    pub fn bytes(mut self, name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.0.insert(name.into(), ArgValue::Bytes(value.into()));
        self
    }

    /// Look up a value.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ArgValue> {
        self.0.get(name)
    }

    /// Iterate over the supplied names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Whether nothing was supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many values were supplied.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Args {
    /// Insert a value, replacing any previous one under the same name.
    pub fn insert(&mut self, name: impl Into<String>, value: ArgValue) {
        self.0.insert(name.into(), value);
    }
}
