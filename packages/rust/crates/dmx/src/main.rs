//! `govee-dmx` — the Art-Net node over `govee-toolkit`.
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
use govee_toolkit::Identify;
use govee_toolkit_dmx::patch::{Layout, MAX_PORT_ADDRESS, PortAddress};
use govee_toolkit_dmx::profile::Personality;

mod cmd;

/// The personality `--personality widest` names: the widest one the device
/// serves, which a long strip can take a whole universe for.
const WIDEST: &str = "widest";

#[derive(Debug, Parser)]
#[command(
    name = "govee-dmx",
    version,
    about = "Drive Govee devices from a lighting desk, over Art-Net. Unofficial.",
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
    Identify {
        /// The patch file. The default is `patch.yaml` beside `config.yaml`.
        patch: Option<PathBuf>,
        /// The color each fixture shows, as `#RRGGBB`.
        #[arg(long, default_value = "#00ff00", value_name = "COLOR")]
        color: String,
        /// How long the walk waits between two steps: after the rig goes
        /// off, and after each fixture lights.
        #[arg(long, default_value_t = 1000, value_name = "MS")]
        wait_ms: u64,
        /// How long the last fixture holds the color before every fixture
        /// goes off.
        #[arg(long, default_value_t = 5000, value_name = "MS")]
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
    /// Whether this invocation asked for the machine form.
    const fn as_json(&self) -> bool {
        match self {
            Self::Profile { json, .. }
            | Self::Patch { json, .. }
            | Self::Identify { json, .. }
            | Self::Run { json, .. } => *json,
        }
    }
}

/// What a scan asks of the patch file.
fn options(
    personality: &str,
    universe: u16,
    reset: bool,
    dry_run: bool,
) -> Result<cmd::patch::Options, cmd::Failure> {
    let layout = if personality == WIDEST {
        Layout::Widest
    } else {
        Personality::parse(personality)
            .map(Layout::Fixed)
            .ok_or_else(|| {
                cmd::Failure::new(
                    format!(
                        "unknown personality `{personality}`; expected {}",
                        spellings()
                    ),
                    cmd::USAGE,
                )
            })?
    };
    let first = PortAddress::new(universe).ok_or_else(|| {
        cmd::Failure::new(
            format!("universe {universe} is over the {MAX_PORT_ADDRESS} Art-Net holds"),
            cmd::USAGE,
        )
    })?;
    Ok(cmd::patch::Options {
        layout,
        first,
        reset,
        dry_run,
    })
}

/// What the command line asks of one identify walk.
fn walk(
    color: &str,
    wait_ms: u64,
    hold_ms: u64,
    keep: bool,
) -> Result<cmd::identify::Walk, cmd::Failure> {
    Ok(cmd::identify::Walk {
        pass: Identify {
            color: rgb(color)?,
            ..Identify::default()
        },
        wait: Duration::from_millis(wait_ms),
        hold: Duration::from_millis(hold_ms),
        keep,
    })
}

/// One `#RRGGBB`, as a person types it on a desk.
fn rgb(text: &str) -> Result<[u8; 3], cmd::Failure> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    let bytes = (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
        .ok_or_else(|| cmd::Failure::new(format!("`{text}` is no `#RRGGBB` color"), cmd::USAGE))?;
    Ok([
        u8::try_from((bytes >> 16) & 0xff).unwrap_or(0),
        u8::try_from((bytes >> 8) & 0xff).unwrap_or(0),
        u8::try_from(bytes & 0xff).unwrap_or(0),
    ])
}

fn spellings() -> String {
    Personality::ALL
        .iter()
        .map(|personality| format!("`{personality}`"))
        .chain(std::iter::once(format!("`{WIDEST}`")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn main() -> ExitCode {
    let Cli { command } = Cli::parse();
    let as_json = command.as_json();
    let outcome = match command {
        Command::Profile {
            sku,
            personality,
            json,
        } => cmd::profile::run(&sku, personality.as_deref(), json),
        Command::Patch {
            patch,
            personality,
            universe,
            reset,
            dry_run,
            config,
            json,
        } => match options(&personality, universe, reset, dry_run) {
            Ok(options) => cmd::patch::start(patch.as_deref(), options, config.as_deref(), json),
            Err(failure) => Err(failure),
        },
        Command::Identify {
            patch,
            color,
            wait_ms,
            hold_ms,
            keep,
            config,
            json,
        } => match walk(&color, wait_ms, hold_ms, keep) {
            Ok(walk) => cmd::identify::start(patch.as_deref(), walk, config.as_deref(), json),
            Err(failure) => Err(failure),
        },
        Command::Run {
            patch,
            scan,
            personality,
            universe,
            dry_run,
            no_reset,
            debug,
            config,
            json,
        } => match options(&personality, universe, false, false) {
            Ok(options) => {
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
                    json,
                )
            }
            Err(failure) => Err(failure),
        },
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(as_json),
    }
}
