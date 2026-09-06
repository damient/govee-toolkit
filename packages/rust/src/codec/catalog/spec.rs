//! What a command declares about its arguments, and the roles the SDK finds
//! them by.
//!
//! The device file names every argument and every command; a role is how the
//! SDK reaches one without a name of its own living in this code.

use std::fmt;

use serde::Deserialize;

/// How an argument may be supplied.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArgSpec {
    /// A whole number, bounded inclusively.
    Int {
        /// `[min, max]`, both inclusive.
        range: [i64; 2],
        /// What the SDK fills this argument with. See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// A list of RGB triples, for a frame repeat group.
    RgbList {
        /// Optional cap on the number of triples.
        #[serde(default)]
        max_len: Option<usize>,
        /// What the SDK fills this argument with. See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Text, sent as UTF-8 behind a length prefix.
    String {
        /// Optional cap, in bytes of UTF-8 rather than characters: that is what
        /// the length prefix counts.
        #[serde(default)]
        max_len: Option<usize>,
        /// What the SDK fills this argument with. See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Zone indices, sent as a bitmask.
    Zones {
        /// How many zones exist. An index past it is refused rather than
        /// dropped into a bit the firmware ignores.
        #[serde(default)]
        count: Option<usize>,
        /// What the SDK fills this argument with. See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Bytes this crate does not interpret.
    Bytes {
        /// Optional cap on the length.
        #[serde(default)]
        max_len: Option<usize>,
        /// What the SDK fills this argument with. See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
}

impl ArgSpec {
    /// The name used in error messages.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int { .. } => crate::codec::args::INT,
            Self::RgbList { .. } => crate::codec::args::RGB_LIST,
            Self::String { .. } => crate::codec::args::TEXT,
            Self::Zones { .. } => crate::codec::args::ZONES,
            Self::Bytes { .. } => crate::codec::args::BYTES,
        }
    }

    /// What the SDK fills this argument with, if the file says.
    #[must_use]
    pub fn role(&self) -> Option<ArgRole> {
        match self {
            Self::Int { role, .. }
            | Self::RgbList { role, .. }
            | Self::String { role, .. }
            | Self::Zones { role, .. }
            | Self::Bytes { role, .. } => *role,
        }
    }
}

/// What one argument of a [`Role`] command carries, when the SDK supplies it
/// without being told a name.
///
/// The device file names arguments as well as commands, so a role the SDK
/// invokes on its own needs this to say which declared argument is which. An
/// argument the caller always passes needs no role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgRole {
    /// Whether to arm or disarm, on a [`Role::SegmentEnable`] command. `1`
    /// arms.
    Enable,
    /// One RGB triple per zone, on a [`Role::SegmentColor`] command.
    Colors,
    /// Whether the firmware interpolates between zones, on a
    /// [`Role::SegmentColor`] command. Optional: a command declaring no
    /// argument for it is sent none.
    Gradient,
}

impl ArgRole {
    /// Every argument role, so a caller can iterate them.
    pub(crate) const ALL: [Self; 3] = [Self::Enable, Self::Colors, Self::Gradient];

    /// The argument type this role has to be declared as.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Enable | Self::Gradient => crate::codec::args::INT,
            Self::Colors => crate::codec::args::RGB_LIST,
        }
    }
}

impl fmt::Display for ArgRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Enable => "enable",
            Self::Colors => "colors",
            Self::Gradient => "gradient",
        })
    }
}

/// What a command is for, when the SDK has to pick one without being told.
///
/// Only the device file names commands. A role lets it say which entry serves a
/// purpose the SDK has of its own, so no command name has to live in code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Reports the device's state. This is what fire-and-verify sends after a
    /// command, and what a `status()` call encodes.
    Status,
    /// Arms and disarms the raw segment channel. Must declare an argument
    /// marked [`ArgRole::Enable`].
    SegmentEnable,
    /// Paints every zone at once. Must declare an argument marked
    /// [`ArgRole::Colors`]; one marked [`ArgRole::Gradient`] is supplied when
    /// declared.
    SegmentColor,
}

impl Role {
    /// Every role the SDK picks a command by, so a caller can iterate them.
    pub(crate) const CLAIMABLE: [Self; 3] = [Self::Status, Self::SegmentEnable, Self::SegmentColor];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Status => "status",
            Self::SegmentEnable => "segment_enable",
            Self::SegmentColor => "segment_color",
        })
    }
}
