//! What a device can do, and what a mode reaches of it.
//!
//! Capability names are data: the codec reads [`SEGMENTS`] for the zone counts
//! a stream needs, and every other name is an opaque string. A device file that
//! takes an argument's bounds from a capability names both the capability and
//! the parameter itself (see [`catalog::Bounds`]). Parameters are the exception
//! to the opacity — an unknown one is refused, not ignored.
//!
//! [`catalog::Bounds`]: crate::codec::catalog::Bounds

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// The capability carrying addressable zones.
pub const SEGMENTS: &str = "segments";

/// The parameters of [`CapabilityParams`] that carry a pair, so a device file
/// can point an argument's `range:` at one. In the order they are declared.
pub const PAIR_PARAMS: [&str; 2] = ["range", "range_kelvin"];

/// Parameters qualifying one capability, named below. All optional, and an
/// unknown one fails the file to load.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CapabilityParams {
    /// Accepted bounds, inclusive — `brightness`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[i64; 2]>,
    /// Accepted bounds in kelvin, inclusive — `colortemp`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_kelvin: Option<[i64; 2]>,
    /// Zones the Govee app exposes — `segments`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Individually addressable LEDs, measured on a physical unit —
    /// `segments`. Absent means nobody measured one; it is never
    /// extrapolated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_pixels: Option<u32>,
}

impl CapabilityParams {
    /// The pair the parameter `name` declares. `None` where the parameter is
    /// absent, and where it is not one of [`PAIR_PARAMS`].
    #[must_use]
    pub fn pair(&self, name: &str) -> Option<[i64; 2]> {
        match name {
            "range" => self.range,
            "range_kelvin" => self.range_kelvin,
            _ => None,
        }
    }
}

/// What the hardware can do, regardless of mode. A capability it does not have
/// is **absent**, never `false`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Capabilities(BTreeMap<String, CapabilityParams>);

impl Capabilities {
    /// Whether the hardware declares `name`.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    /// The parameters declared for `name`, or `None` when the hardware does not
    /// have it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CapabilityParams> {
        self.0.get(name)
    }

    /// Every capability declared, in name order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Zones the Govee app exposes, from `segments.count`.
    #[must_use]
    pub fn segment_count(&self) -> Option<u32> {
        self.get(SEGMENTS).and_then(|params| params.count)
    }

    /// Addressable LEDs measured on the unit, from `segments.native_pixels`.
    /// `None` means nobody measured one.
    #[must_use]
    pub fn native_pixels(&self) -> Option<u32> {
        self.get(SEGMENTS).and_then(|params| params.native_pixels)
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        // `power:` with nothing after it is a capability with no parameters,
        // which YAML hands over as null.
        let declared = BTreeMap::<String, Option<CapabilityParams>>::deserialize(de)?;
        Ok(Self(
            declared
                .into_iter()
                .map(|(name, params)| (name, params.unwrap_or_default()))
                .collect(),
        ))
    }
}

/// The capabilities a mode reaches.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ModeCapabilities {
    /// The literal `all`: every capability the hardware has.
    All(AllKeyword),
    /// An explicit subset, by capability name.
    Subset(Vec<String>),
}

impl ModeCapabilities {
    /// The names this mode reaches. One the hardware does not declare is kept
    /// rather than dropped, and `crate::codec::validate` reports it.
    #[must_use]
    pub fn resolve<'a>(&'a self, hardware: &'a Capabilities) -> Vec<&'a str> {
        match self {
            Self::All(_) => hardware.names().collect(),
            Self::Subset(names) => names.iter().map(String::as_str).collect(),
        }
    }
}

impl Default for ModeCapabilities {
    fn default() -> Self {
        Self::Subset(Vec::new())
    }
}

/// The `all` keyword, as it appears in a device file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AllKeyword {
    /// `capabilities: all`
    All,
}

/// Why a capability the hardware has is out of a mode's reach.
///
/// The vocabulary is documented in `docs/compatibility.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Established that this transport does not carry it. A claim about the
    /// transport, and only correct when somebody established it.
    Transport,
    /// The transport carries it, but this file declares no command for it yet.
    Unimplemented,
    /// Nobody checked whether this mode reaches it. The default, and the
    /// honest answer until somebody probes it.
    #[default]
    Unprobed,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Transport => "transport",
            Self::Unimplemented => "unimplemented",
            Self::Unprobed => "unprobed",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityParams, PAIR_PARAMS};

    /// `pair` and `PAIR_PARAMS` name the same set, or a device file points at
    /// a parameter the lookup silently answers nothing for.
    #[test]
    fn every_listed_parameter_reads_back() {
        let params = CapabilityParams {
            range: Some([1, 100]),
            range_kelvin: Some([2000, 9000]),
            ..CapabilityParams::default()
        };
        for name in PAIR_PARAMS {
            assert!(params.pair(name).is_some(), "`{name}` reads nothing");
        }
        assert_eq!(params.pair("count"), None);
    }
}
