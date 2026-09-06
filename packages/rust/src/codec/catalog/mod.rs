//! The device catalog: `devices/*.yaml`, deserialized.
//!
//! These types mirror `devices/schema.yaml` field for field. Nothing here is
//! SKU-specific: the catalog is data, and the codec in [`crate::codec::frame`]
//! and [`crate::codec::command`] interprets it generically.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::codec::capabilities::{Capabilities, ModeCapabilities, Reason};
use crate::codec::chunk::Chunk;
use crate::codec::measurements::Measurements;

mod spec;

pub use spec::{ArgRole, ArgSpec, Role};

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

/// One entry of a device file's `modes:` table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ModeSupport {
    /// Support level.
    pub support: Support,
    /// Capabilities reachable in this mode.
    pub capabilities: ModeCapabilities,
    /// Capabilities the hardware has that this mode does not reach, each with
    /// the reason it does not. Together with `capabilities` this covers the
    /// hardware's whole set, which `crate::codec::validate` checks.
    pub unreachable: BTreeMap<String, Reason>,
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

/// One command, in one mode.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Command {
    /// The value sent in `msg.cmd`, for `lan` and `cloud`. Empty where the mode
    /// puts the frame on the wire with no envelope around it.
    pub cmd: String,
    /// `false` marks a command found through reverse engineering.
    pub documented: bool,
    /// The `msg.data` template. Placeholders are whole strings, `"${name}"`.
    pub payload: serde_json::Value,
    /// The byte layout of a raw-channel frame. See [`crate::codec::frame`].
    pub frame: Option<String>,
    /// The byte layout of a payload too long for one frame, split by `chunk`.
    /// Mutually exclusive with `frame`.
    pub body: Option<String>,
    /// How to cut `body` into frames. See [`Chunk`].
    pub chunk: Option<Chunk>,
    /// Declared arguments.
    pub args: BTreeMap<String, ArgSpec>,
    /// Behavior worth knowing before calling it.
    pub notes: String,
    /// Path to a real capture, relative to the repository root.
    pub capture: String,
    /// What the SDK may use this command for on its own. See [`Role`].
    pub role: Option<Role>,

    /// `frame`, tokenized on first use. The layout is fixed by the device file,
    /// so the send path parses it once rather than once per command.
    #[serde(skip)]
    pub(crate) parsed_frame: std::sync::OnceLock<crate::codec::Frame>,
    /// `body` and the three `chunk` layouts, tokenized on first use.
    #[serde(skip)]
    pub(crate) parsed_chunk: std::sync::OnceLock<crate::codec::chunk::Layout>,
}

impl Command {
    /// The argument declared with `role`, by name.
    ///
    /// Returns the name the file gave it, which the caller puts in [`Args`]
    /// like any other. `None` means the command declares no such argument.
    ///
    /// [`Args`]: crate::codec::Args
    #[must_use]
    pub fn arg_for(&self, role: ArgRole) -> Option<&str> {
        self.args
            .iter()
            .find(|(_, spec)| spec.role() == Some(role))
            .map(|(name, _)| name.as_str())
    }
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
        self.commands
            .get(mode)
            .iter()
            .find(|(_, command)| command.role == Some(role))
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
