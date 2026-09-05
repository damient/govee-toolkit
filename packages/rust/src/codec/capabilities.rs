//! What a device can do, and what a mode reaches of it.
//!
//! Capability names are data. The codec reads one of them, [`SEGMENTS`], for
//! the zone counts the segment stream needs, and treats every other as an
//! opaque string — a device file may declare a capability no SDK has heard of.
//! Parameters are the exception: one the codec does not know is refused rather
//! than ignored.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer};

/// The capability carrying addressable zones, and the only name the codec
/// reads.
pub const SEGMENTS: &str = "segments";

/// Parameters qualifying one capability.
///
/// Each field belongs to the capability that declares it; the doc comment says
/// which. All are optional, and an unknown one fails the file to load.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CapabilityParams {
    /// Accepted bounds, inclusive — `brightness`.
    pub range: Option<[i64; 2]>,
    /// Accepted bounds in kelvin, inclusive — `colortemp`.
    pub range_kelvin: Option<[i64; 2]>,
    /// Zones the Govee app exposes — `segments`.
    pub count: Option<u32>,
    /// Individually addressable LEDs, measured on a physical unit —
    /// `segments`. Absent means nobody measured one: the number belongs to the
    /// unit's length and is never extrapolated from another unit.
    pub native_pixels: Option<u32>,
}

/// What the hardware can do, regardless of mode.
///
/// A capability the hardware does not have is **absent**, and one it has with
/// nothing to qualify it carries empty parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ModeCapabilities {
    /// The literal `all`: every capability the hardware has.
    All(AllKeyword),
    /// An explicit subset, by capability name.
    Subset(Vec<String>),
}

impl ModeCapabilities {
    /// The names this mode reaches, resolved against the hardware's set.
    ///
    /// A name the hardware does not declare is kept, so that a caller checking
    /// the file sees it; `crate::codec::validate` reports it.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllKeyword {
    /// `capabilities: all`
    All,
}

/// Why a capability the hardware has is out of a mode's reach.
///
/// The vocabulary is documented in `docs/compatibility.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
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
