//! `govee-dmx` — the DMX bridge over `govee-toolkit`.
//!
//! Every subcommand takes `--json`, which is the form a script and a model
//! read. The text form is for a person and its layout is not stable. The
//! design is `docs/dmx.md`.

// A binary reports and exits; the no-panic rule that protects a host
// application from a library does not apply here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use govee_toolkit::codec::{Mode, coerce};
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::transport::millis;
use govee_toolkit::{IDENTIFY_COLOR, IDENTIFY_HOLD, IDENTIFY_WAIT, Identify, Walk};
use govee_toolkit_dmx::patch::Layout;
use govee_toolkit_dmx::profile::{Personality, UNIVERSE};

mod cmd;

/// The spelling for the widest layout the device serves.
const WIDEST: &str = "widest";

#[derive(Debug, Parser)]
#[command(
    name = "govee-dmx",
    version,
    about = "Drive Govee devices from a lighting desk, over DMX.",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the channel table of one device, to patch a desk with.
    Profile {
        /// The SKU, or a verified alias of one.
        sku: String,
        /// Print this personality alone: `full`, `segment` or `pixel`.
        #[arg(long)]
        personality: Option<String>,
        /// Print the record a machine reads instead of the table.
        #[arg(long)]
        json: bool,
    },
    /// Scan the LAN, and write the patch the node runs from.
    ///
    /// The command appends: it adds an entry for each device that has none,
    /// on the lowest free channels, and it moves no entry the file already
    /// carries. Set `enabled: false` on an entry to take a fixture out of the
    /// rig and keep its channels.
    Patch {
        /// The patch file. The default is `patch.yaml` beside `config.yaml`.
        #[arg(long, value_name = "FILE")]
        patch: Option<PathBuf>,
        /// The layout a new entry takes: `full`, `segment`, `pixel`, or
        /// `widest` for the widest the device serves.
        #[arg(long, default_value = "full")]
        personality: String,
        /// The lowest universe a new entry lands on.
        #[arg(long, default_value_t = 0)]
        universe: u16,
        /// Write the file from the scan alone, and keep the current one as
        /// `.yaml.bak`. Every address in it is set again.
        #[arg(long)]
        reset: bool,
        /// Print what the scan would add, and write nothing.
        #[arg(long)]
        dry_run: bool,
        /// The configuration file. The default is the one `govee` reads.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Print the records a machine reads instead of the lines a person
        /// reads.
        #[arg(long)]
        json: bool,
    },
    /// Light each patched fixture in turn, so a person sees which entry
    /// drives which fixture.
    ///
    /// Every fixture goes off at once first. One fixture at a time then
    /// comes back on in one color. Every fixture goes off again at the end.
    ///
    /// A target names fixtures by their devices: an identity (`1C:8B:…`), a
    /// SKU (`H6159`), a name (`kitchen`), or a group (`bar`). A name and a
    /// group read the patch first, and `config.yaml` after it. A group lights
    /// in patch order. Write `name:` or `group:` where a target reads as
    /// both. `--universe` and `--address` name fixtures by the channels they
    /// answer to. Every fixture the patch enables when the command line names
    /// none.
    Identify {
        /// The devices or groups to light. Every enabled fixture when absent.
        #[arg(value_name = "TARGET")]
        targets: Vec<String>,
        /// The port-address to light. With `--address`, the universe that
        /// channel sits on; the default is 0.
        #[arg(long, value_name = "UNIVERSE")]
        universe: Option<u16>,
        /// The DMX channel to light, 1 to 512. It lights the fixture that
        /// answers to that channel, and not only the one that starts there.
        #[arg(long, value_name = "CHANNEL")]
        address: Option<u16>,
        /// The patch file. The default is `patch.yaml` beside `config.yaml`.
        #[arg(long, value_name = "FILE")]
        patch: Option<PathBuf>,
        /// The color each fixture shows, as `#RRGGBB`.
        #[arg(long, default_value_t = coerce::hex(IDENTIFY_COLOR), value_name = "COLOR")]
        color: String,
        /// How long the walk waits between two steps: after the rig goes
        /// off, and after each fixture lights.
        #[arg(long, default_value_t = millis(IDENTIFY_WAIT), value_name = "MS")]
        wait_ms: u64,
        /// How long the last fixture holds the color before every fixture
        /// goes off.
        #[arg(long, default_value_t = millis(IDENTIFY_HOLD), value_name = "MS")]
        hold_ms: u64,
        /// Leave every fixture on and lit at the end.
        #[arg(long, conflicts_with = "hold_ms")]
        keep: bool,
        /// The configuration file. The default is the one `govee` reads.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Print the records a machine reads instead of the lines a person
        /// reads.
        #[arg(long)]
        json: bool,
    },

    /// Receive Art-Net on port 6454 and drive the patched devices.
    ///
    /// The run stores black on every patched device at start, and leaves the
    /// device off. A device shows the color that it held when it next comes
    /// on, so a color from an earlier run would flash on the first frame that
    /// raises the dimmer.
    Run {
        /// The patch file, which says which device answers which channels.
        /// The default is `patch.yaml` beside `config.yaml`.
        #[arg(long, value_name = "FILE")]
        patch: Option<PathBuf>,
        /// Scan the LAN first, and add an entry for each device that has
        /// none. See the `patch` command.
        #[arg(long)]
        scan: bool,
        /// The layout `--scan` gives a new entry. See the `patch` command.
        #[arg(long, default_value = "full", requires = "scan")]
        personality: String,
        /// The lowest universe `--scan` lands a new entry on.
        #[arg(long, default_value_t = 0, requires = "scan")]
        universe: u16,
        /// Print every packet and what each fixture reads out of it, and
        /// write to no device.
        #[arg(long)]
        dry_run: bool,
        /// Leave every device as it is at start, and store no black on it.
        #[arg(long)]
        no_reset: bool,
        /// Print a line for each answered poll, and the library traces at
        /// `debug`. `RUST_LOG` wins over it. Both go to stderr.
        #[arg(long)]
        debug: bool,
        /// The configuration file. The default is the one `govee` reads.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Print the records a machine reads instead of the lines a person
        /// reads.
        #[arg(long)]
        json: bool,
    },
}

impl Command {
    const fn as_json(&self) -> bool {
        match self {
            Self::Profile { json, .. }
            | Self::Patch { json, .. }
            | Self::Identify { json, .. }
            | Self::Run { json, .. } => *json,
        }
    }
}

fn options(
    personality: &str,
    universe: u16,
    reset: bool,
    dry_run: bool,
) -> Result<cmd::patch::Options, Failure> {
    let layout = if personality == WIDEST {
        Layout::Widest
    } else {
        Personality::parse(personality)
            .map(Layout::Fixed)
            .ok_or_else(|| {
                Failure::usage(format!(
                    "unknown personality `{personality}`; expected {}",
                    spellings()
                ))
            })?
    };
    let first = cmd::port_address(universe)?;
    Ok(cmd::patch::Options {
        layout,
        first,
        reset,
        dry_run,
    })
}

fn walk(color: &str, wait_ms: u64, hold_ms: u64, keep: bool) -> Result<Walk, Failure> {
    Ok(Walk {
        pass: Identify {
            color: rgb(color)?,
            ..Identify::default()
        },
        wait: Duration::from_millis(wait_ms),
        hold: Duration::from_millis(hold_ms),
        keep,
        // The bridge drives a device over `lan` alone.
        mode: Mode::Lan,
    })
}

/// `--address` without `--universe` reads universe 0.
fn chosen(
    targets: Vec<String>,
    universe: Option<u16>,
    address: Option<u16>,
) -> Result<cmd::identify::Chosen, Failure> {
    let port = match (universe, address) {
        (None, None) => None,
        (universe, _) => Some(cmd::port_address(universe.unwrap_or(0))?),
    };
    if let Some(channel) = address
        && (channel == 0 || channel > UNIVERSE)
    {
        return Err(Failure::usage(format!(
            "channel {channel} is outside the 1 to {UNIVERSE} a universe carries"
        )));
    }
    Ok(cmd::identify::Chosen {
        targets,
        universe: port,
        address,
    })
}

fn rgb(text: &str) -> Result<[u8; 3], Failure> {
    coerce::rgb(text)
        .ok_or_else(|| Failure::usage(format!("`{text}` is not a color; write `#RRGGBB`")))
}

/// Every personality spelling, `widest` included.
fn spellings() -> String {
    format!("{}, `{WIDEST}`", cmd::spellings())
}

fn main() -> ExitCode {
    let Cli { command } = Cli::parse();
    let writer = Writer::new(command.as_json());
    match dispatch(command, writer) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => writer.failure(&failure),
    }
}

fn dispatch(command: Command, writer: Writer) -> Result<(), Failure> {
    match command {
        Command::Profile {
            sku,
            personality,
            json: _,
        } => cmd::profile::run(&sku, personality.as_deref(), writer),
        Command::Patch {
            patch,
            personality,
            universe,
            reset,
            dry_run,
            config,
            json: _,
        } => {
            let options = options(&personality, universe, reset, dry_run)?;
            cmd::patch::start(patch.as_deref(), options, config.as_deref(), writer)
        }
        Command::Identify {
            targets,
            universe,
            address,
            patch,
            color,
            wait_ms,
            hold_ms,
            keep,
            config,
            json: _,
        } => {
            let walk = walk(&color, wait_ms, hold_ms, keep)?;
            let chosen = chosen(targets, universe, address)?;
            cmd::identify::start(patch.as_deref(), &chosen, walk, config.as_deref(), writer)
        }
        Command::Run {
            patch,
            scan,
            personality,
            universe,
            dry_run,
            no_reset,
            debug,
            config,
            json: _,
        } => {
            let options = options(&personality, universe, false, false)?;
            cmd::trace(debug);
            cmd::run::start(
                patch.as_deref(),
                scan.then_some(options),
                cmd::run::Flags {
                    dry_run,
                    no_reset,
                    debug,
                },
                config.as_deref(),
                writer,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use super::*;

    /// The walk `Walk::default()` documents, over `lan` as the bridge drives.
    #[test]
    fn identify_with_no_option_runs_the_default_walk() {
        let cli = Cli::try_parse_from(["govee-dmx", "identify"]).expect("parses");
        let Command::Identify {
            color,
            wait_ms,
            hold_ms,
            keep,
            ..
        } = cli.command
        else {
            panic!("parses as identify");
        };
        let walk = walk(&color, wait_ms, hold_ms, keep).expect("the default color reads");
        assert_eq!(walk, Walk::default());
    }
}
