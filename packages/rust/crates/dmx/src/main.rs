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

use clap::{Parser, Subcommand};

mod cmd;

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
        /// Print this personality alone: `basic`, `full`, `pixel` or
        /// `pixel-native`.
        #[arg(long)]
        personality: Option<String>,
        /// Print the record a machine reads instead of the table.
        #[arg(long)]
        json: bool,
    },
    /// Receive Art-Net on port 6454 and drive the patched devices.
    Run {
        /// The patch file, which says which device answers which channels.
        patch: PathBuf,
        /// Print every packet and what each fixture reads out of it, and
        /// write to no device.
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
}

impl Command {
    /// Whether this invocation asked for the machine form.
    const fn as_json(&self) -> bool {
        match self {
            Self::Profile { json, .. } | Self::Run { json, .. } => *json,
        }
    }
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
        Command::Run {
            patch,
            dry_run,
            config,
            json,
        } => cmd::run::start(&patch, dry_run, config.as_deref(), json),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(as_json),
    }
}
