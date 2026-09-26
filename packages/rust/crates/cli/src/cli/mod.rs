//! The command surface. The verbs are [`Verb`], flattened into [`Command`],
//! so `govee on <device>` parses as one of them.

use std::path::PathBuf;

use clap::builder::{PossibleValuesParser, TypedValueParser as _};
use clap::{Parser, Subcommand};
use govee_toolkit::codec::{Mode, coerce};
use govee_toolkit::transport::millis;
use govee_toolkit::{IDENTIFY_COLOR, IDENTIFY_HOLD, IDENTIFY_WAIT, Resolution};

mod verbs;

pub(crate) use verbs::Verb;

#[derive(Debug, Parser)]
#[command(name = "govee", version, about = "Control Govee devices. Unofficial.")]
pub(crate) struct Cli {
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Global {
    /// Write JSON instead of text. Errors are JSON too.
    #[arg(long, global = true)]
    pub json: bool,

    /// Restrict the run to one mode. It never enables a mode the
    /// configuration leaves out, and it never falls back to another.
    ///
    /// The names are the crate's own.
    #[arg(
        long,
        global = true,
        value_name = "MODE",
        value_parser = PossibleValuesParser::new(Mode::NAMES)
            .try_map(|name| name.parse::<Mode>()),
    )]
    pub mode: Option<Mode>,

    /// Read the configuration from this file instead of the default path.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Read the `GOVEE_*` variables from this file instead of the `.env` the
    /// search finds. A file that is absent is an error here.
    #[arg(long, global = true, value_name = "PATH", conflicts_with = "no_env")]
    pub env_file: Option<PathBuf>,

    /// Read no `.env`. The process environment supplies the variables alone.
    #[arg(long, global = true)]
    pub no_env: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Discover devices and report what answered.
    Scan {
        /// How long to wait for answers.
        #[arg(long, default_value_t = 3000, value_name = "MS")]
        timeout_ms: u64,
    },

    /// List the devices already known, without touching the network.
    Devices {
        /// The devices to list: an identity, a SKU, a name, or a group. Every
        /// known device when absent. See `identify` for the whole grammar.
        #[arg(value_name = "TARGET")]
        targets: Vec<String>,
    },

    /// Report what a device file declares: modes, capabilities, commands and
    /// arguments. Reads no hardware.
    Describe {
        /// The device identity, or the SKU.
        target: String,
    },

    /// Send one device file entry by name.
    Send {
        /// The device: its identity, or a name the configuration gives.
        device: String,
        /// The entry in `commands.<mode>` of the device file.
        command: String,
        /// One argument, as `name=value`. Repeat for each.
        #[arg(long = "arg", value_name = "NAME=VALUE")]
        args: Vec<String>,
    },

    /// Read the device's state.
    Status {
        /// The device: its identity, or a name the configuration gives.
        device: String,
    },

    #[command(flatten)]
    Verb(Verb),

    /// Light each device in turn, so a person sees which identity drives
    /// which fixture.
    ///
    /// Every device goes off at once first. One device at a time then comes
    /// back on in one color. Every device goes off again at the end.
    ///
    /// The walk drives `lan`, and the mode `--mode` names where it names one.
    /// It substitutes no other mode.
    ///
    /// A target names an identity (`1C:8B:…`), a SKU (`H6159`), a name the
    /// configuration gives a device (`kitchen`), or a group the configuration
    /// gives (`ambient`). `id:`, `sku:`, `name:` and `group:` state the kind
    /// where the target alone does not.
    Identify {
        /// The devices, in the order to light them. Every device a scan finds
        /// when absent.
        #[arg(value_name = "TARGET")]
        targets: Vec<String>,
        /// The color each device shows, as `#RRGGBB`.
        #[arg(long, default_value_t = coerce::hex(IDENTIFY_COLOR), value_name = "COLOR")]
        color: String,
        /// How long the walk waits between two steps: after the rig goes
        /// off, and after each device lights.
        #[arg(long, default_value_t = millis(IDENTIFY_WAIT), value_name = "MS")]
        wait_ms: u64,
        /// How long the last device holds the color before every device goes
        /// off.
        #[arg(long, default_value_t = millis(IDENTIFY_HOLD), value_name = "MS")]
        hold_ms: u64,
        /// Leave every device on and lit at the end.
        #[arg(long, conflicts_with = "hold_ms")]
        keep: bool,
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
    /// comma-separated. The stream closes at end of input, which disarms the
    /// segment channel: the device goes back to the color it showed before.
    /// Use `segment` to paint colors that stay.
    Stream {
        /// The device: its identity, or a name the configuration gives.
        device: String,
        /// How many zones every frame states: `app`, `native`, `groups`, or a
        /// count. A count the unit renders as a smaller one is refused.
        #[arg(long, default_value_t = Resolution::default().to_string(), value_name = "RESOLUTION")]
        resolution: String,
        /// Frames per second. The measured rate for this unit when absent.
        #[arg(long, value_name = "HZ")]
        rate: Option<f64>,
        /// Interpolate between zones, and wrap from the last back to the
        /// first. Refused where the device file can carry the setting
        /// nowhere.
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
        /// The device: its identity, or a name the configuration gives.
        device: String,
        /// The network name. `GOVEE_WIFI_SSID` supplies it when absent, from
        /// the environment or from `.env`.
        /// 2.4 GHz: no Govee device joins a 5 GHz network.
        #[arg(long, value_name = "SSID")]
        ssid: Option<String>,
        /// The password. `GOVEE_WIFI_PASSWORD` supplies it when absent, from
        /// the environment or from `.env`.
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

impl Command {
    /// The target of a command that drives one device.
    pub(crate) fn device(&self) -> Option<&str> {
        match self {
            Self::Send { device, .. } | Self::Status { device } | Self::Stream { device, .. } => {
                Some(device)
            }
            #[cfg(feature = "ble")]
            Self::Provision { device, .. } => Some(device),
            Self::Scan { .. }
            | Self::Verb(_)
            | Self::Identify { .. }
            | Self::Devices { .. }
            | Self::Doctor
            | Self::Describe { .. }
            | Self::Watch { .. } => None,
        }
    }

    /// The target of a command that drives one device or one group.
    pub(crate) fn members(&self) -> Option<&str> {
        match self {
            Self::Verb(verb) => Some(verb.device()),
            _ => None,
        }
    }
}
