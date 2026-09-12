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
        /// See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// A list of RGB triples, for a frame repeat group.
    RgbList {
        /// Optional cap on the number of triples.
        #[serde(default)]
        max_len: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Text, sent as UTF-8 behind a length prefix.
    String {
        /// Optional cap in bytes of UTF-8, not characters: the length prefix
        /// counts bytes.
        #[serde(default)]
        max_len: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Zone indices, sent as a bitmask.
    Zones {
        /// How many zones exist. An index past it is refused rather than
        /// dropped into a bit the firmware ignores.
        #[serde(default)]
        count: Option<usize>,
        /// See [`ArgRole`].
        #[serde(default)]
        role: Option<ArgRole>,
    },
    /// Bytes this crate does not interpret.
    Bytes {
        /// Optional cap on the length.
        #[serde(default)]
        max_len: Option<usize>,
        /// See [`ArgRole`].
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
/// An argument the caller always passes needs no role, and a captured field
/// needs one only where the SDK models it.
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
    /// The lit color, captured from a reply, packed as `0xRRGGBB`.
    Color,
    /// The red component to set, on a [`Role::Color`] command, or the color a
    /// [`Role::Music`] command imposes.
    Red,
    /// The green component of the same triple.
    Green,
    /// The blue component of the same triple.
    Blue,
    /// The white temperature, in kelvin. What a [`Role::ColorTemp`] command
    /// sets, and what a reply reports: `0` reports a device in color mode.
    ColorTemp,
    /// The red component of the RGB rendering of that temperature, on a
    /// [`Role::ColorTemp`] command. The SDK computes it from the kelvin
    /// value. Optional: a command that declares none is sent none.
    WhiteRed,
    /// The green component of the same rendering.
    WhiteGreen,
    /// The blue component of the same rendering.
    WhiteBlue,
    /// The network name to join, on a [`Role::WifiProvision`] command.
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
    /// The endpoint the device must talk to, on a [`Role::WifiProvision`]
    /// command that carries one. The SDK derives it from [`ArgRole::ApiType`].
    ApiUrl,
    /// Which endpoint the device asks for, captured from a
    /// [`Role::WifiApiType`] reply.
    ApiType,
    /// Whether provisioning must carry the hidden-network flag, captured from
    /// the same reply. No command encodes that flag yet.
    HideSsid,
    /// Which effect a [`Role::Music`] command plays. The identifiers are the
    /// mode's own: the same number names another effect over another mode.
    Effect,
    /// How loud the sound must be for a [`Role::Music`] command to answer it,
    /// in the unit that argument's `range:` gives.
    Sensitivity,
    /// Whether a [`Role::Music`] command renders in fades rather than on the
    /// beat. `1` fades. Optional: a command that declares none is sent none.
    Soft,
    /// Whether a [`Role::Music`] command imposes a color. `0` leaves the
    /// colors to the firmware, `1` plays the triple the same command marks
    /// [`ArgRole::Red`], [`ArgRole::Green`] and [`ArgRole::Blue`]. Optional.
    ColorMode,
}

impl ArgRole {
    pub(crate) const ALL: [Self; 27] = [
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
            | Self::ColorMode => crate::codec::args::INT,
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
    /// Turns the device on and off. Must declare an argument marked
    /// [`ArgRole::On`]. `1` turns on.
    Power,
    /// Sets the brightness. Must declare an argument marked
    /// [`ArgRole::Brightness`], in the unit that argument's `range:` gives.
    Brightness,
    /// Sets one color over the whole device. Must declare arguments marked
    /// [`ArgRole::Red`], [`ArgRole::Green`] and [`ArgRole::Blue`].
    Color,
    /// Sets the white temperature, and the white it renders. Must declare an
    /// argument marked [`ArgRole::ColorTemp`]. Where the frame carries the
    /// rendering, it must declare the three marked [`ArgRole::WhiteRed`],
    /// [`ArgRole::WhiteGreen`] and [`ArgRole::WhiteBlue`], which the SDK
    /// computes from the kelvin value.
    ColorTemp,
    /// Arms and disarms the raw segment channel. Must declare an argument
    /// marked [`ArgRole::Enable`].
    SegmentEnable,
    /// Paints every zone at once. Must declare an argument marked
    /// [`ArgRole::Colors`]; one marked [`ArgRole::Gradient`] is supplied when
    /// declared.
    SegmentColor,
    /// Paints one color over the zones a mask names, so a stream over it
    /// costs one write per distinct color. Must declare an argument marked
    /// [`ArgRole::Colors`] and one marked [`ArgRole::Zones`].
    SegmentColorMasked,
    /// Sets zone interpolation, on a mode that carries it in a frame of its
    /// own. Must declare an argument marked [`ArgRole::Gradient`].
    SegmentGradient,
    /// Wakes the Wi-Fi module before provisioning, and releases it after. Must
    /// declare an argument marked [`ArgRole::Enable`].
    WifiLink,
    /// Reports which endpoint the device asks provisioning for. Must capture a
    /// field marked [`ArgRole::ApiType`]. A file that declares none makes the
    /// SDK provision without an endpoint.
    WifiApiType,
    /// Carries the network credentials. Must declare arguments marked
    /// [`ArgRole::Network`], [`ArgRole::Password`], [`ArgRole::RunMode`],
    /// [`ArgRole::TimezoneHours`], [`ArgRole::TimezoneMinutes`] and
    /// [`ArgRole::IotVersion`].
    WifiProvision,
    /// The same, plus an argument marked [`ArgRole::ApiUrl`]. The SDK sends
    /// this one when [`Role::WifiApiType`] reports a type.
    WifiProvisionWithApi,
    /// Plays an effect the device renders from its own microphone. Must
    /// declare an argument marked [`ArgRole::Effect`]. The SDK fills the ones
    /// marked [`ArgRole::Sensitivity`], [`ArgRole::Soft`],
    /// [`ArgRole::ColorMode`], [`ArgRole::Red`], [`ArgRole::Green`] and
    /// [`ArgRole::Blue`] where the entry declares them.
    Music,
}

impl Role {
    pub(crate) const CLAIMABLE: [Self; 14] = [
        Self::Status,
        Self::Power,
        Self::Brightness,
        Self::Color,
        Self::ColorTemp,
        Self::SegmentEnable,
        Self::SegmentColor,
        Self::SegmentColorMasked,
        Self::SegmentGradient,
        Self::WifiLink,
        Self::WifiApiType,
        Self::WifiProvision,
        Self::WifiProvisionWithApi,
        Self::Music,
    ];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Status => "status",
            Self::Power => "power",
            Self::Brightness => "brightness",
            Self::Color => "color",
            Self::ColorTemp => "color_temp",
            Self::SegmentEnable => "segment_enable",
            Self::SegmentColor => "segment_color",
            Self::SegmentColorMasked => "segment_color_masked",
            Self::SegmentGradient => "segment_gradient",
            Self::WifiLink => "wifi_link",
            Self::WifiApiType => "wifi_api_type",
            Self::WifiProvision => "wifi_provision",
            Self::WifiProvisionWithApi => "wifi_provision_with_api",
            Self::Music => "music",
        })
    }
}
