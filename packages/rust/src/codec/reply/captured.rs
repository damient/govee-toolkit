//! The fields a reply was read into.

use std::collections::BTreeMap;

use crate::codec::args::ArgValue;

/// The fields one or more replies were read into.
///
/// Keyed by the name the device file gave the field. Nothing here interprets a
/// name: what a field means is the file's business.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Captured(BTreeMap<String, ArgValue>);

impl Captured {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up one field.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ArgValue> {
        self.0.get(name)
    }

    /// Every field, by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ArgValue)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    /// Whether nothing was captured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many fields were captured.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Add a field, and replace any previous field of the same name.
    pub fn insert(&mut self, name: impl Into<String>, value: ArgValue) {
        self.0.insert(name.into(), value);
    }

    /// Take in everything another set captured.
    pub fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// The fields as JSON: an integer as a number, text as a string, bytes as
    /// lowercase hex.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        use std::fmt::Write as _;

        let mut map = serde_json::Map::new();
        for (name, value) in &self.0 {
            let json = match value {
                ArgValue::Int(v) => serde_json::Value::from(*v),
                ArgValue::Text(text) => serde_json::Value::from(text.clone()),
                ArgValue::Bytes(bytes) => {
                    let mut hex = String::with_capacity(bytes.len().saturating_mul(2));
                    for byte in bytes {
                        let _ = write!(hex, "{byte:02x}");
                    }
                    serde_json::Value::from(hex)
                }
                ArgValue::Rgb(_) | ArgValue::Zones(_) => continue,
            };
            map.insert(name.clone(), json);
        }
        serde_json::Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn captured_fields_reach_a_caller_as_json() {
        let mut captured = Captured::new();
        captured.insert("count", ArgValue::Int(42));
        captured.insert("version", ArgValue::Text("1.02.00".to_owned()));
        captured.insert("mac", ArgValue::Bytes(vec![0xaa, 0xbb]));
        let json = captured.to_json();
        assert_eq!(json["count"], 42);
        assert_eq!(json["version"], "1.02.00");
        assert_eq!(json["mac"], "aabb");
    }

    #[test]
    fn merging_takes_in_what_a_later_exchange_captured() {
        let mut first = Captured::new();
        first.insert("lit", ArgValue::Int(1));
        let mut second = Captured::new();
        second.insert("level", ArgValue::Int(64));

        first.merge(second);
        assert_eq!(first.len(), 2);
        assert_eq!(first.get("level"), Some(&ArgValue::Int(64)));
    }
}
