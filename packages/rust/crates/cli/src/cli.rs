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
        /// absent. A subset needs a mode that paints by zone mask.
        #[arg(long, value_name = "LIST")]
        zones: Option<String>,
        /// `#RRGGBB`.
        color: String,
        /// Interpolate between zones, and wrap from the last back to the
        /// first. Refused where the device file can carry the setting
        /// nowhere.
        #[arg(long)]
        gradient: bool,
    },

    /// Report everything wrong with the configuration. Reads no hardware.
    Doctor,

    /// Print events as they arrive, until the process is interrupted.
    Watch {
        /// Scan again every this many milliseconds. `0` scans once, at the
        /// start, and then only listens.
        #[arg(long, default_value_t = 0, value_name = "MS")]
        rescan_ms: u64,
    },

    /// Stream colors to the segment channel, one frame per line of stdin.
    ///
    /// A line is one `#RRGGBB`, which fills every zone, or one per zone,
    /// comma-separated. The stream closes at end of input.
    Stream {
        /// The device identity.
        device: String,
        /// How many zones to carry: `app`, `native`, or a count.
        #[arg(long, default_value = "app", value_name = "ZONES")]
        zones: String,
        /// Frames per second. The measured rate for this unit when absent.
        #[arg(long, value_name = "HZ")]
        rate: Option<f64>,
        /// Interpolate between zones, and wrap from the last back to the
        /// first.
        #[arg(long)]
        gradient: bool,
    },

    /// Put a device on a Wi-Fi network over `ble`.
    ///
    /// The password travels in plaintext, with no key exchange: anything in
    /// Bluetooth range while this runs reads it. Close the vendor app first,
    /// which holds the one connection the radio accepts.
    #[cfg(feature = "ble")]
    Provision {
        /// The device identity.
        device: String,
        /// The network name. 2.4 GHz: no Govee device joins a 5 GHz network.
        #[arg(long, value_name = "SSID")]
        ssid: String,
        /// The password. `GOVEE_WIFI_PASSWORD` supplies it when absent.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// Join a network that has no password.
        #[arg(long)]
        open: bool,
        /// Whole hours of the device's UTC offset.
        #[arg(long, default_value_t = 0, value_name = "HOURS")]
        utc_offset_hours: u8,
        /// The remaining minutes of that offset.
        #[arg(long, default_value_t = 0, value_name = "MINUTES")]
        utc_offset_minutes: u8,
    },
}
