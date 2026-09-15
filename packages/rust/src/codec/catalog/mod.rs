//! The device catalog: `devices/*.yaml`, deserialized.
//!
//! These types mirror `devices/schema.yaml` field for field. Nothing here is
//! SKU-specific: the catalog is data, and the codec in [`crate::codec::frame`]
//! and [`crate::codec::command`] interprets it generically.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::codec::capabilities::{Capabilities, ModeCapabilities, Reason};
use crate::codec::chunk::Chunk;
use crate::codec::cloud::{Capability, Read};
use crate::codec::exchange::{Exchanges, Step};
use crate::codec::measurements::Measurements;

mod bounds;
mod mode;
mod overrides;
mod spec;

pub use bounds::{Bounds, CapabilityRef, resolve as resolve_bounds};
pub use mode::{Mode, UnknownMode};
pub use overrides::{ArgOverride, Override, Overrides, apply as apply_overrides};
pub use spec::{ArgRole, ArgSpec, Role};

/// How much of a device's capability set a mode reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Support {
    /// Every capability the hardware has.
    Full,
    /// Every capability the transport carries, which is less than the hardware
    /// has. A boundary of the transport, not work left on this device.
    Capped,
    /// A subset, listed in [`ModeSupport::capabilities`]. At least one
    /// capability out of reach is work left.
    Partial,
    /// The hardware does not do this mode. A claim, so set it only when
    /// somebody established it; not probed is [`Support::Unknown`].
    None,
    /// Nobody has probed this mode on this device. The default.
    ///
    /// A failed probe and an unimplemented feature look identical from
    /// outside, so the mode can still be enabled: that is how it gets
    /// probed.
    #[default]
    Unknown,
}

impl fmt::Display for Support {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full => "full",
            Self::Capped => "capped",
            Self::Partial => "partial",
            Self::None => "none",
            Self::Unknown => "unknown",
        })
    }
}

/// One entry of a device file's `modes:` table.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModeSupport {
    /// Support level.
    pub support: Support,
    /// Capabilities reachable in this mode.
    pub capabilities: ModeCapabilities,
    /// Capabilities this mode does not reach, each with a reason. With
    /// `capabilities` it must cover the hardware's whole set.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub unreachable: BTreeMap<String, Reason>,
    /// Free-form notes.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// A device file's `modes:` table.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Command {
    /// The value sent in `msg.cmd`, for `lan` and `cloud`. Empty where the mode
    /// puts the frame on the wire with no envelope around it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cmd: String,
    /// The `msg.data` template. Placeholders are whole strings, `"${name}"`.
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub payload: serde_json::Value,
    /// The byte layout of a raw-channel frame. See [`crate::codec::frame`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    /// The layout of the reply `frame` expects. See [`crate::codec::reply`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
    /// Several send/reply exchanges, issued in order. Mutually exclusive with
    /// `frame` and `body`. See [`crate::codec::exchange`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<Step>,
    /// The byte layout of a payload too long for one frame, split by `chunk`.
    /// Mutually exclusive with `frame`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// How to cut `body` into frames. See [`Chunk`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<Chunk>,
    /// Declared arguments.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, ArgSpec>,
    /// Behavior worth knowing before calling it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// What the SDK may use this command for on its own. See [`Role`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// The capability a `cloud` command writes. See
    /// [`crate::codec::cloud`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<Capability>,
    /// Which capability answers into which argument, on a `cloud` command that
    /// reads a status.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<Read>,

    /// The exchanges, tokenized on first use, so the send path parses a
    /// layout once and not once per command.
    #[serde(skip)]
    pub(crate) parsed_exchanges: std::sync::OnceLock<Option<Exchanges>>,
    /// `body` and the three `chunk` layouts, tokenized on first use.
    #[serde(skip)]
    pub(crate) parsed_chunk: std::sync::OnceLock<crate::codec::chunk::Layout>,
}

impl Command {
    /// Whether the entry declares an answer to read back, in any of the four
    /// shapes that can carry one.
    #[must_use]
    pub fn answers(&self) -> bool {
        self.reply.is_some()
            || !self.frames.is_empty()
            || !self.reads.is_empty()
            || self
                .chunk
                .as_ref()
                .is_some_and(|chunk| chunk.reply.is_some())
    }

    /// The arguments this entry declares, comma-separated, or `none`. What an
    /// error message names when a caller supplies an argument the entry does
    /// not declare.
    #[must_use]
    pub fn declared(&self) -> String {
        if self.args.is_empty() {
            return "none".to_owned();
        }
        self.args
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The name the file gave the argument declared with `role`. `None` if
    /// the command declares none.
    #[must_use]
    pub fn arg_for(&self, role: ArgRole) -> Option<&str> {
        self.args
            .iter()
            .find(|(_, spec)| spec.role() == Some(role))
            .map(|(name, _)| name.as_str())
    }
}

/// A device file's `commands:` table, one map per mode.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Commands {
    /// `lan` commands.
    #[serde(deserialize_with = "null_as_default")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lan: BTreeMap<String, Command>,
    /// `ble` commands.
    #[serde(deserialize_with = "null_as_default")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub ble: BTreeMap<String, Command>,
    /// `cloud` commands.
    #[serde(deserialize_with = "null_as_default")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub cloud: BTreeMap<String, Command>,
}

impl Commands {
    pub(crate) fn get_mut(&mut self, mode: Mode) -> &mut BTreeMap<String, Command> {
        match mode {
            Mode::Lan => &mut self.lan,
            Mode::Ble => &mut self.ble,
            Mode::Cloud => &mut self.cloud,
        }
    }

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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Verified {
    /// Who tested it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub by: String,
    /// Firmware versions tested against.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub firmware: String,
    /// `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    /// What was exercised, and what was not.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// One `devices/families/<name>.yaml` file: commands several SKUs share.
///
/// A fragment carries no SKU and no capability: a dialect is a property of a
/// family, and what one unit answered stays in that unit's file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Family {
    /// Schema revision the fragment was written against.
    pub schema_version: u32,
    /// What a device file names in its `include:`.
    pub family: String,
    /// What the fragment covers.
    #[serde(default)]
    pub description: String,
    /// The command tables it contributes.
    #[serde(default)]
    pub commands: Commands,
}

/// One `devices/<SKU>.yaml` file.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// SKUs that look like the same product but have not been verified. These
    /// deliberately do **not** resolve: a lookup for one is an unknown SKU.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub candidate_aliases: Vec<String>,
    /// Shared command tables to pull in, by the `family:` each declares. The
    /// commands they carry are merged into [`Device::commands`] on load, so
    /// nothing downstream can tell an included entry from a local one.
    ///
    /// Not serialized: the generated catalog is flat.
    #[serde(default, skip_serializing)]
    pub include: Vec<String>,
    /// What the hardware can do.
    pub capabilities: Capabilities,
    /// Which modes the hardware supports.
    #[serde(default)]
    pub modes: Modes,
    /// The command tables.
    #[serde(default)]
    pub commands: Commands,
    /// What this file changes in a command an `include:` brought in. Applied
    /// on load, so nothing downstream reads it. See [`Overrides`].
    #[serde(default, skip_serializing)]
    pub overrides: Overrides,
    /// Numbers taken from one physical unit.
    #[serde(default)]
    pub measurements: Measurements,
    /// Verification record.
    #[serde(default)]
    pub verified: Verified,
}

impl Device {
    /// The entry in `commands.<mode>` that claims `role`.
    ///
    /// The entry's name, not its `cmd`. `None` means the file claims that
    /// role for nothing here, and the caller must do without.
    #[must_use]
    pub fn command_for(&self, mode: Mode, role: Role) -> Option<&str> {
        self.entry_for(mode, role).map(|(name, _)| name)
    }

    /// The name and the declaration of the entry that claims `role`.
    ///
    /// A caller that reads the entry's arguments takes both here, so it looks
    /// the name up once. See [`Device::command_for`].
    #[must_use]
    pub fn entry_for(&self, mode: Mode, role: Role) -> Option<(&str, &Command)> {
        self.commands
            .get(mode)
            .iter()
            .find(|(_, command)| command.role == Some(role))
            .map(|(name, command)| (name.as_str(), command))
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
