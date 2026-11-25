//! The roles a command claims, so the SDK finds it without a name of its own.

use std::fmt;

use serde::{Deserialize, Serialize};

/// What a command is for, when the SDK must pick one without being told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Reports the device's state. This is what fire-and-verify sends after a
    /// command, and what a `status()` call encodes.
    Status,
    /// Turns the device on and off. Must declare an argument marked
    /// [`ArgRole::On`](super::ArgRole::On). `1` turns on.
    Power,
    /// Sets the brightness. Must declare an argument marked
    /// [`ArgRole::Brightness`](super::ArgRole::Brightness), in the unit that
    /// argument's `range:` gives.
    Brightness,
    /// Sets one color over the whole device. Must declare arguments marked
    /// [`ArgRole::Red`](super::ArgRole::Red),
    /// [`ArgRole::Green`](super::ArgRole::Green) and
    /// [`ArgRole::Blue`](super::ArgRole::Blue).
    Color,
    /// Sets the white temperature, and the white it renders. Must declare an
    /// argument marked [`ArgRole::ColorTemp`](super::ArgRole::ColorTemp), plus
    /// the three white components where the frame carries the rendering.
    ColorTemp,
    /// Arms and disarms the raw segment channel. Must declare an argument
    /// marked [`ArgRole::Enable`](super::ArgRole::Enable).
    SegmentEnable,
    /// Paints every zone at once. Must declare an argument marked
    /// [`ArgRole::Colors`](super::ArgRole::Colors); one marked
    /// [`ArgRole::Gradient`](super::ArgRole::Gradient) is supplied when
    /// declared.
    SegmentColor,
    /// Paints one color over the zones a mask names, so a stream over it
    /// costs one write per distinct color. Must declare an argument marked
    /// [`ArgRole::Colors`](super::ArgRole::Colors) and one marked
    /// [`ArgRole::Zones`](super::ArgRole::Zones).
    SegmentColorMasked,
    /// Paints every LED at once and stores the result, so the colors stay
    /// after the sender stops and arm no channel. Must declare an argument
    /// marked [`ArgRole::Colors`](super::ArgRole::Colors) and one marked
    /// [`ArgRole::Zones`](super::ArgRole::Zones), which takes every LED.
    Draw,
    /// Sets zone interpolation, on a mode that carries it in a frame of its
    /// own. Must declare an argument marked
    /// [`ArgRole::Gradient`](super::ArgRole::Gradient).
    SegmentGradient,
    /// Wakes the Wi-Fi module before provisioning, and releases it after. Must
    /// declare an argument marked [`ArgRole::Enable`](super::ArgRole::Enable).
    WifiLink,
    /// Reports which endpoint the device asks provisioning for. Must capture a
    /// field marked [`ArgRole::ApiType`](super::ArgRole::ApiType). A file that
    /// declares none makes the SDK provision without an endpoint.
    WifiApiType,
    /// Carries the network credentials. Must declare arguments marked
    /// [`ArgRole::Network`](super::ArgRole::Network),
    /// [`ArgRole::Password`](super::ArgRole::Password),
    /// [`ArgRole::RunMode`](super::ArgRole::RunMode),
    /// [`ArgRole::TimezoneHours`](super::ArgRole::TimezoneHours),
    /// [`ArgRole::TimezoneMinutes`](super::ArgRole::TimezoneMinutes) and
    /// [`ArgRole::IotVersion`](super::ArgRole::IotVersion).
    WifiProvision,
    /// The same, plus an argument marked
    /// [`ArgRole::ApiUrl`](super::ArgRole::ApiUrl). The SDK sends this one when
    /// [`Role::WifiApiType`] reports a type.
    WifiProvisionWithApi,
    /// Plays an effect the device renders from its own microphone. Must
    /// declare an argument marked [`ArgRole::Effect`](super::ArgRole::Effect);
    /// the SDK fills the other music arguments where the entry declares them.
    Music,
    /// Uploads the parameters of a scene, then plays it. Must declare
    /// arguments marked [`ArgRole::SceneCode`](super::ArgRole::SceneCode),
    /// [`ArgRole::SceneParam`](super::ArgRole::SceneParam) and
    /// [`ArgRole::SceneType`](super::ArgRole::SceneType).
    Scene,
    /// Plays a scene that the firmware stores, by its code alone. Must
    /// declare an argument marked
    /// [`ArgRole::SceneCode`](super::ArgRole::SceneCode).
    SceneBuiltin,
}

impl Role {
    /// Every role. `Display` gives the name that a device file writes.
    pub const CLAIMABLE: [Self; 17] = [
        Self::Status,
        Self::Power,
        Self::Brightness,
        Self::Color,
        Self::ColorTemp,
        Self::SegmentEnable,
        Self::SegmentColor,
        Self::SegmentColorMasked,
        Self::Draw,
        Self::SegmentGradient,
        Self::WifiLink,
        Self::WifiApiType,
        Self::WifiProvision,
        Self::WifiProvisionWithApi,
        Self::Music,
        Self::Scene,
        Self::SceneBuiltin,
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
            Self::Draw => "draw",
            Self::SegmentGradient => "segment_gradient",
            Self::WifiLink => "wifi_link",
            Self::WifiApiType => "wifi_api_type",
            Self::WifiProvision => "wifi_provision",
            Self::WifiProvisionWithApi => "wifi_provision_with_api",
            Self::Music => "music",
            Self::Scene => "scene",
            Self::SceneBuiltin => "scene_builtin",
        })
    }
}
