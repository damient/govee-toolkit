//! The command surface.
//!
//! Two kinds of subcommand live here. `send` names a device file entry and
//! carries no command name in this crate. The verbs — `on`, `brightness`,
//! `segment` — name one, and reach the device file through a `role:` where the
//! schema declares one, so that Node and Python get the same verb from the
//! core rather than a second implementation.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use govee_toolkit::codec::Mode;

/// The whole invocation.
#[derive(Debug, Parser)]
#[command(name = "govee", version, about = "Control Govee devices. Unofficial.")]
pub(crate) struct Cli {
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

/// What every subcommand accepts.
#[derive(Debug, clap::Args)]
pub(crate) struct Global {
    /// Write JSON instead of text. Errors are JSON too.
    #[arg(long, global = true)]
    pub json: bool,

    /// Restrict the run to one mode. It never enables a mode the
    /// configuration leaves out, and it never falls back to another.
    #[arg(long, global = true, value_enum, value_name = "MODE")]
    pub mode: Option<ModeArg>,

    /// Read the configuration from this file instead of the default path.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// The mode names accepted on the command line.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ModeArg {
    /// UDP on the local network.
    Lan,
    /// Bluetooth Low Energy.
    Ble,
    /// Govee's cloud API.
    Cloud,
}

impl From<ModeArg> for Mode {
    fn from(arg: ModeArg) -> Self {
        match arg {
            ModeArg::Lan => Self::Lan,
            ModeArg::Ble => Self::Ble,
            ModeArg::Cloud => Self::Cloud,
        }
    }
}

/// One subcommand.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Discover devices and report what answered.
    Scan {
        /// How long to wait for answers.
        #[arg(long, default_value_t = 3000, value_name = "MS")]
        timeout_ms: u64,
    },

    /// List the devices already known, without touching the network.
    Devices,

    /// Report what a device file declares: modes, capabilities, commands and
    /// arguments. Reads no hardware.
    Describe {
        /// The device identity, or the SKU.
        target: String,
    },

    /// Send one device file entry by name.
    Send {
        /// The device identity.
        device: String,
        /// The entry in `commands.<mode>` of the device file.
        command: String,
        /// One argument, as `name=value`. Repeat for each.
        #[arg(long = "arg", value_name = "NAME=VALUE")]
        args: Vec<String>,
    },

    /// Read the device's state.
    Status {
        /// The device identity.
        device: String,
    },

    /// Turn the device on.
    On {
        /// The device identity.
        device: String,
    },

    /// Turn the device off.
    Off {
        /// The device identity.
        device: String,
    },

    /// Set the brightness, in the unit the device file declares.
    Brightness {
        /// The device identity.
        device: String,
        /// The value. Out of range is an error, never a clamp.
        value: i64,
    },

    /// Set one color over the whole device.
    Color {
        /// The device identity.
        device: String,
        /// `#RRGGBB`.
        color: String,
    },

    /// Paint addressable zones.
    Segment {
        /// The device identity.
        device: String,
        /// Zone indices, zero-based and comma-separated. Every zone when
        /// absent.
        #[arg(long, value_name = "LIST")]
        zones: Option<String>,
        /// `#RRGGBB`.
        color: String,
    },
}
