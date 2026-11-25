//! What a command declares about its arguments, and the roles the SDK finds
//! them by.
//!
//! The device file names every argument and every command; a role is how the
//! SDK reaches one without a name of its own in this code.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::bounds::Bounds;

/// How an argument may be supplied.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArgSpec {
    /// A whole number.
    Int {
        /// `[min, max]`, both inclusive, or the capability parameter the pair
        /// comes from. Absent where the device file declares no bound: the
        /// value goes out as the caller gave it, and the mode answers.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        range: Option<Bounds>,
        /// The value the codec sends when the caller supplies none.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<i64>,
        /// See [`ArgRole`].
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<ArgRole>,
    },
    /// A list of RGB triples, for a frame repeat group.
    RgbList {
        /// Optional cap on the number of triples.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        max_len: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<ArgRole>,
    },
    /// Text, sent as UTF-8 behind a length prefix.
    String {
        /// Optional cap in bytes of UTF-8, not characters: the length prefix
        /// counts bytes.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        max_len: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<ArgRole>,
    },
    /// Zone indices, sent as a bitmask.
    Zones {
        /// How many zones exist. An index past it is refused rather than
        /// dropped into a bit the firmware ignores.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<ArgRole>,
    },
    /// Bytes this crate does not interpret.
    Bytes {
        /// Optional cap on the length.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        max_len: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
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

    /// The name `devices/schema.yaml` gives this type, which is the `type`
    /// the serialized form carries.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Int { .. } => "int",
            Self::RgbList { .. } => "rgb_list",
            Self::String { .. } => "string",
            Self::Zones { .. } => "zones",
            Self::Bytes { .. } => "bytes",
        }
    }

    /// What the SDK fills this argument with, if the file says.
    #[must_use]
    pub fn default_value(&self) -> Option<i64> {
        match self {
            Self::Int { default, .. } => *default,
            _ => None,
        }
    }

    /// See [`ArgRole`].
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
/// back without being told a name. An argument the caller always passes needs
/// no role, and a captured field needs one only where the SDK models it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgRole {
    /// Whether to arm or disarm, on a
    /// [`Role::SegmentEnable`](super::Role::SegmentEnable) command. `1` arms.
    Enable,
    /// One RGB triple per zone, on a
    /// [`Role::SegmentColor`](super::Role::SegmentColor) command, or the single
    /// triple a [`Role::SegmentColorMasked`](super::Role::SegmentColorMasked)
    /// frame paints its zones, or one triple per LED on a
    /// [`Role::Draw`](super::Role::Draw) command.
    Colors,
    /// Which zones a
    /// [`Role::SegmentColorMasked`](super::Role::SegmentColorMasked) command
    /// paints, or which LEDs a
    /// [`Role::Draw`](super::Role::Draw) command paints,
    /// zero-based.
    Zones,
    /// Whether the firmware interpolates between zones, on a
    /// [`Role::SegmentColor`](super::Role::SegmentColor) command. Optional: a
    /// command declaring no argument for it is sent none.
    Gradient,
    /// Whether the device is on, captured from a reply. Non-zero is on.
    On,
    /// The device's brightness, captured from a reply, in whatever unit the
    /// firmware reports it.
    Brightness,
    /// The lit color, captured from a reply, packed as `0xRRGGBB`.
    Color,
    /// The red component of the color a command sets or imposes.
    Red,
    /// The green component of the same triple.
    Green,
    /// The blue component of the same triple.
    Blue,
    /// The white temperature, in kelvin. What a
    /// [`Role::ColorTemp`](super::Role::ColorTemp) command sets, and what a
    /// reply reports: `0` reports a device in color mode.
    ColorTemp,
    /// The red component of the RGB rendering of that temperature, which the
    /// SDK computes from the kelvin value. Optional.
    WhiteRed,
    /// The green component of the same rendering.
    WhiteGreen,
    /// The blue component of the same rendering.
    WhiteBlue,
    /// The network name to join, on a
    /// [`Role::WifiProvision`](super::Role::WifiProvision) command.
    Network,
    /// The password of that network. Plaintext on the wire.
    Password,
    /// The firmware's run mode. The SDK sends `0`, which is what production
    /// takes.
    RunMode,
    /// The host's UTC offset, whole hours. Sent apart from the minutes, and
    /// never as a combined offset.
    TimezoneHours,
    /// The remaining minutes of that offset, `0` on a whole-hour zone.
    TimezoneMinutes,
    /// The firmware's `IoT` protocol version. The SDK sends `0`.
    IotVersion,
    /// The endpoint the device must talk to, on a
    /// [`Role::WifiProvision`](super::Role::WifiProvision) command that carries
    /// one. The SDK derives it from [`ArgRole::ApiType`].
    ApiUrl,
    /// Which endpoint the device asks for, captured from a
    /// [`Role::WifiApiType`](super::Role::WifiApiType) reply.
    ApiType,
    /// Whether provisioning must carry the hidden-network flag, captured from
    /// the same reply. No command encodes that flag yet.
    HideSsid,
    /// Which effect to play. The identifiers are the mode's own: the same
    /// number names another effect over another mode.
    Effect,
    /// How loud the sound must be for the device to answer it.
    Sensitivity,
    /// Whether to render in fades rather than on the beat. `1` fades.
    /// Optional.
    Soft,
    /// Whether to impose the triple marked [`ArgRole::Red`],
    /// [`ArgRole::Green`] and [`ArgRole::Blue`]. `0` leaves the colors to the
    /// firmware. Optional.
    ColorMode,
    /// What a chunked transfer was answered with, captured from its
    /// acknowledgement. `0` says the device accepted the transfer.
    AckStatus,
    /// The 16-bit code of a scene, as the scene library gives it.
    SceneCode,
    /// The parameters of a scene, as the scene library gives them, sent as
    /// they are.
    SceneParam,
    /// The type the scene library gives a scene. Its `range:` names the types
    /// that the entry's transfer plays.
    SceneType,
}

impl ArgRole {
    pub(crate) const ALL: [Self; 31] = [
        Self::Enable,
        Self::Colors,
        Self::Zones,
        Self::Gradient,
        Self::On,
        Self::Brightness,
        Self::Color,
        Self::Red,
        Self::Green,
        Self::Blue,
        Self::ColorTemp,
        Self::WhiteRed,
        Self::WhiteGreen,
        Self::WhiteBlue,
        Self::Network,
        Self::Password,
        Self::RunMode,
        Self::TimezoneHours,
        Self::TimezoneMinutes,
        Self::IotVersion,
        Self::ApiUrl,
        Self::ApiType,
        Self::HideSsid,
        Self::Effect,
        Self::Sensitivity,
        Self::Soft,
        Self::ColorMode,
        Self::AckStatus,
        Self::SceneCode,
        Self::SceneParam,
        Self::SceneType,
    ];

    /// The argument type a device file must declare for this role.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Enable
            | Self::Gradient
            | Self::On
            | Self::Brightness
            | Self::Color
            | Self::Red
            | Self::Green
            | Self::Blue
            | Self::ColorTemp
            | Self::WhiteRed
            | Self::WhiteGreen
            | Self::WhiteBlue
            | Self::RunMode
            | Self::TimezoneHours
            | Self::TimezoneMinutes
            | Self::IotVersion
            | Self::ApiType
            | Self::HideSsid
            | Self::Effect
            | Self::Sensitivity
            | Self::Soft
            | Self::ColorMode
            | Self::AckStatus
            | Self::SceneCode
            | Self::SceneType => crate::codec::args::INT,
            Self::SceneParam => crate::codec::args::BYTES,
            Self::Colors => crate::codec::args::RGB_LIST,
            Self::Zones => crate::codec::args::ZONES,
            Self::Network | Self::Password | Self::ApiUrl => crate::codec::args::TEXT,
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
            Self::Color => "color",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::ColorTemp => "color_temp",
            Self::WhiteRed => "white_red",
            Self::WhiteGreen => "white_green",
            Self::WhiteBlue => "white_blue",
            Self::Network => "network",
            Self::Password => "password",
            Self::RunMode => "run_mode",
            Self::TimezoneHours => "timezone_hours",
            Self::TimezoneMinutes => "timezone_minutes",
            Self::IotVersion => "iot_version",
            Self::ApiUrl => "api_url",
            Self::ApiType => "api_type",
            Self::HideSsid => "hide_ssid",
            Self::Effect => "effect",
            Self::Sensitivity => "sensitivity",
            Self::Soft => "soft",
            Self::ColorMode => "color_mode",
            Self::AckStatus => "ack_status",
            Self::SceneCode => "scene_code",
            Self::SceneParam => "scene_param",
            Self::SceneType => "scene_type",
        })
    }
}
