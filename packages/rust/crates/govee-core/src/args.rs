//! Argument values supplied at call time.

use std::collections::BTreeMap;

/// A value for one declared argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgValue {
    /// A whole number.
    Int(i64),
    /// A list of RGB triples.
    Rgb(Vec<[u8; 3]>),
}

impl ArgValue {
    /// The name used in error messages.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "an integer",
            Self::Rgb(_) => "a list of RGB triples",
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
