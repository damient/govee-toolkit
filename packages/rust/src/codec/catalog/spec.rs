//! What a command declares about its arguments, and the roles the SDK finds
//! them by.
//!
//! The device file names every argument and every command; a role is how the
//! SDK reaches one without a name of its own in this code.

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
        /// Optional cap in bytes of UTF-8, not characters: the length prefix
        /// counts bytes.
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
    /// The name error messages use for this type.
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

/// What one declared argument carries, when the SDK fills it in or reads it
/// back without being told a name.
///
/// An argument the caller always passes needs no role. A field a `reply:`
/// layout captures needs one only where the SDK models it: a transport's
/// `DeviceStatus` models [`ArgRole::On`] and [`ArgRole::Brightness`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgRole {
    /// Whether to arm or disarm, on a [`Role::SegmentEnable`] command. `1`
    /// arms.
    Enable,
    /// One RGB triple per zone, on a [`Role::SegmentColor`] command, or the
    /// single triple a [`Role::SegmentColorMasked`] frame paints its zones.
    Colors,
    /// Which zones a [`Role::SegmentColorMasked`] command paints, zero-based.
    Zones,
    /// Whether the firmware interpolates between zones, on a
    /// [`Role::SegmentColor`] command. Optional: a command declaring no
    /// argument for it is sent none.
    Gradient,
    /// Whether the device is on, captured from a reply. Non-zero is on.
    On,
    /// The device's brightness, captured from a reply, in whatever unit the
    /// firmware reports it.
    Brightness,
}

impl ArgRole {
    pub(crate) const ALL: [Self; 6] = [
        Self::Enable,
        Self::Colors,
        Self::Zones,
        Self::Gradient,
        Self::On,
        Self::Brightness,
    ];

    /// The argument type a device file must declare for this role.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Enable | Self::Gradient | Self::On | Self::Brightness => crate::codec::args::INT,
            Self::Colors => crate::codec::args::RGB_LIST,
            Self::Zones => crate::codec::args::ZONES,
        }
    }
}

impl fmt::Display for ArgRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Enable => "enable",
            Self::Colors => "colors",
            Self::Zones => "zones",
            Self::Gradient => "gradient",
            Self::On => "on",
            Self::Brightness => "brightness",
        })
    }
}

/// What a command is for, when the SDK must pick one without being told.
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
    /// Paints one color over the zones a mask names. Must declare an argument
    /// marked [`ArgRole::Colors`] and one marked [`ArgRole::Zones`].
    ///
    /// A frame carries one color, so a stream over such a command sends one
    /// write per run of equal color rather than one per frame.
    SegmentColorMasked,
    /// Sets whether the firmware interpolates between zones, on a mode that
    /// carries that setting in a frame of its own rather than in the painting
    /// one. Must declare an argument marked [`ArgRole::Gradient`].
    SegmentGradient,
}

impl Role {
    pub(crate) const CLAIMABLE: [Self; 5] = [
        Self::Status,
        Self::SegmentEnable,
        Self::SegmentColor,
        Self::SegmentColorMasked,
        Self::SegmentGradient,
    ];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Status => "status",
            Self::SegmentEnable => "segment_enable",
            Self::SegmentColor => "segment_color",
            Self::SegmentColorMasked => "segment_color_masked",
            Self::SegmentGradient => "segment_gradient",
        })
    }
}
