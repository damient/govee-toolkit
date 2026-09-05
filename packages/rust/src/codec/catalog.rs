//! The device catalog: `devices/*.yaml`, deserialized.
//!
//! These types mirror `devices/schema.yaml` field for field. Nothing here is
//! SKU-specific: the catalog is data, and the codec in [`crate::codec::frame`]
//! and [`crate::codec::command`] interprets it generically.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::codec::measurements::Measurements;

/// A way of talking to a device. Not a fallback chain — see `docs/modes.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// UDP on the local network. The default, and the only mode that never
    /// leaves it.
    Lan,
    /// Bluetooth Low Energy.
    Ble,
    /// Govee's cloud API.
    Cloud,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Lan => "lan",
            Self::Ble => "ble",
            Self::Cloud => "cloud",
        })
    }
}

/// How much of a device's capability set a mode reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Support {
    /// Every capability the hardware has.
    Full,
    /// A subset, listed in [`ModeSupport::capabilities`].
    Partial,
    /// Reachable by no command: the hardware does not do this mode.
    ///
    /// A claim about the hardware, and only correct when somebody established
    /// it. Not probed is [`Support::Unknown`].
    None,
    /// Nobody has probed this mode on this device.
    ///
    /// The default, and the honest answer for a mode nobody tried: a failed
    /// probe and an unimplemented feature look identical from outside
    /// (`docs/protocol/lan.md`). Enabling the mode is allowed — that is how it
    /// gets probed — and a command it does not carry still fails explicitly,
    /// as [`Error::UnknownCommand`](crate::codec::Error::UnknownCommand).
    #[default]
    Unknown,
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

/// One entry of a device file's `modes:` table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ModeSupport {
    /// Support level.
    pub support: Support,
    /// Capabilities reachable in this mode.
    pub capabilities: ModeCapabilities,
    /// Free-form notes.
    pub notes: String,
}

/// A device file's `modes:` table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Modes {
    /// `lan` support.
    pub lan: ModeSupport,
    /// `ble` support.
    pub ble: ModeSupport,
    /// `cloud` support.
    pub cloud: ModeSupport,
}

impl Modes {
    /// The entry for `mode`.
    #[must_use]
    pub fn get(&self, mode: Mode) -> &ModeSupport {
        match mode {
            Mode::Lan => &self.lan,
            Mode::Ble => &self.ble,
            Mode::Cloud => &self.cloud,
        }
    }
}

/// What the hardware can do, regardless of mode.
// One flag per capability, mirroring `devices/schema.yaml`. Grouping them into
// a bitfield would only make the device files harder to read.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    /// On / off.
    pub power: bool,
    /// Brightness.
    pub brightness: bool,
    /// RGB color.
    pub color: bool,
    /// White color temperature.
    pub colortemp: bool,
    /// Manufacturer scenes.
    pub scenes: bool,
    /// Addressable zones.
    pub segments: bool,
    /// Sensor telemetry.
    pub sensors: bool,
    /// Accepted brightness bounds, inclusive.
    pub brightness_range: [i64; 2],
    /// Accepted color temperature bounds, in kelvin, inclusive.
    pub colortemp_range_kelvin: [i64; 2],
    /// Zones the Govee app exposes.
    pub segment_count: u32,
    /// Individually addressable LEDs, if measured on a physical unit. `0` means
    /// not measured — never extrapolated from another unit.
    pub native_pixels: u32,
}

/// How an argument may be supplied.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArgSpec {
    /// A whole number, bounded inclusively.
    Int {
        /// `[min, max]`, both inclusive.
        range: [i64; 2],
    },
    /// A list of RGB triples, for a frame repeat group.
    RgbList {
        /// Optional cap on the number of triples.
        #[serde(default)]
        max_len: Option<usize>,
    },
}

impl ArgSpec {
    /// The name used in error messages.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int { .. } => crate::codec::args::INT,
            Self::RgbList { .. } => crate::codec::args::RGB_LIST,
        }
    }
}

/// What a command is for, when the SDK has to pick one without being told.
///
/// Only the device file names commands. A role lets it say which entry serves a
/// purpose the SDK has of its own, so no command name has to live in code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Nothing beyond the name the caller asks for. The default.
    #[default]
    None,
    /// Reports the device's state. This is what fire-and-verify sends after a
    /// command, and what a `status()` call encodes.
    Status,
    /// Arms and disarms the raw segment channel. Must declare an int argument
    /// named `on`.
    SegmentEnable,
    /// Paints every zone at once. Must declare an `rgb_list` argument named
    /// `colors`; an int argument named `gradient` is supplied when declared.
    SegmentColor,
}

impl Role {
    /// Every role the SDK picks a command by, so a caller can iterate them.
    pub(crate) const CLAIMABLE: [Self; 3] = [Self::Status, Self::SegmentEnable, Self::SegmentColor];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Status => "status",
            Self::SegmentEnable => "segment_enable",
            Self::SegmentColor => "segment_color",
        })
    }
}

/// One command, in one mode.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Command {
    /// The value sent in `msg.cmd`, for `lan` and `cloud`.
    pub cmd: String,
    /// `false` marks a command found through reverse engineering.
    pub documented: bool,
    /// The `msg.data` template. Placeholders are whole strings, `"${name}"`.
    pub payload: serde_json::Value,
    /// The byte layout of a raw-channel frame. See [`crate::codec::frame`].
    pub frame: Option<String>,
    /// Declared arguments.
    pub args: BTreeMap<String, ArgSpec>,
    /// Behavior worth knowing before calling it.
    pub notes: String,
    /// Path to a real capture, relative to the repository root.
    pub capture: String,
    /// What the SDK may use this command for on its own. See [`Role`].
    pub role: Role,

    /// `frame`, tokenized on first use. The layout is fixed by the device file,
    /// so the send path parses it once rather than once per command.
    #[serde(skip)]
    pub(crate) parsed_frame: std::sync::OnceLock<crate::codec::Frame>,
}

/// A device file's `commands:` table, one map per mode.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Commands {
    /// `lan` commands.
    #[serde(deserialize_with = "null_as_default")]
    pub lan: BTreeMap<String, Command>,
    /// `ble` commands.
    #[serde(deserialize_with = "null_as_default")]
    pub ble: BTreeMap<String, Command>,
    /// `cloud` commands.
    #[serde(deserialize_with = "null_as_default")]
    pub cloud: BTreeMap<String, Command>,
}

impl Commands {
    /// The command table for `mode`.
    #[must_use]
    pub fn get(&self, mode: Mode) -> &BTreeMap<String, Command> {
        match mode {
            Mode::Lan => &self.lan,
            Mode::Ble => &self.ble,
            Mode::Cloud => &self.cloud,
        }
    }
}

/// Who tested the device, against which firmware.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Verified {
    /// Who tested it.
    pub by: String,
    /// Firmware versions tested against.
    pub firmware: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// What was exercised, and what was not.
    pub notes: String,
}

/// One `devices/<SKU>.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    /// Schema revision the file was written against.
    pub schema_version: u32,
    /// The SKU this file describes.
    pub sku: String,
    /// Product family.
    pub family: String,
    /// Human-readable model name.
    pub name: String,
    /// SKUs **verified** to behave identically. These resolve to this file.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// SKUs that look like the same product but have not been verified. These
    /// deliberately do **not** resolve: a lookup for one is an unknown SKU.
    #[serde(default)]
    pub candidate_aliases: Vec<String>,
    /// What the hardware can do.
    pub capabilities: Capabilities,
    /// Which modes the hardware supports.
    #[serde(default)]
    pub modes: Modes,
    /// The command tables.
    #[serde(default)]
    pub commands: Commands,
    /// Numbers taken from one physical unit.
    #[serde(default)]
    pub measurements: Measurements,
    /// Verification record.
    #[serde(default)]
    pub verified: Verified,
}

impl Device {
    /// The entry in `commands.<mode>` that claims `role`, if the file names
    /// one.
    ///
    /// Returns the entry's name, not its `cmd`: the caller encodes it like any
    /// other command. `None` means the file claims that role for nothing in
    /// this mode, and callers do without rather than guessing a name.
    #[must_use]
    pub fn command_for(&self, mode: Mode, role: Role) -> Option<&str> {
        if role == Role::None {
            return None;
        }
        self.commands
            .get(mode)
            .iter()
            .find(|(_, command)| command.role == role)
            .map(|(name, _)| name.as_str())
    }

    /// The entry in `commands.<mode>` that reports state. See
    /// [`Device::command_for`].
    #[must_use]
    pub fn status_command(&self, mode: Mode) -> Option<&str> {
        self.command_for(mode, Role::Status)
    }
}

fn null_as_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(de)?.unwrap_or_default())
}
