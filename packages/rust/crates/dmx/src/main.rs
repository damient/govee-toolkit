//! `govee-dmx` — the Art-Net node over `govee-toolkit`.

// A binary reports and exits; the no-panic rule that protects a host
// application from a library does not apply here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use govee_toolkit::codec::Catalog;
use govee_toolkit_dmx::profile::{self, Personality, Profile};
use govee_toolkit_dmx::report;
use serde_json::json;

/// The command line is wrong, or it names something no device file carries.
/// clap reports its own with the same code.
const USAGE: u8 = 2;
/// The device serves no such channel table. Nothing was printed.
const REFUSED: u8 = 5;

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
}

/// What failed, and with which exit code.
struct Failure {
    message: String,
    code: u8,
}

fn main() -> ExitCode {
    let Cli { command } = Cli::parse();
    let Command::Profile {
        sku,
        personality,
        json,
    } = command;
    match run(&sku, personality.as_deref(), json) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            if json {
                eprintln!("{}", json!({ "error": { "message": failure.message } }));
            } else {
                eprintln!("error: {}", failure.message);
            }
            ExitCode::from(failure.code)
        }
    }
}

fn run(sku: &str, personality: Option<&str>, as_json: bool) -> Result<(), Failure> {
    let catalog = Catalog::embedded().map_err(|error| Failure {
        message: error.to_string(),
        code: USAGE,
    })?;
    let device = catalog.device(sku).map_err(|error| Failure {
        message: error.to_string(),
        code: USAGE,
    })?;
    let tables = tables(device, personality)?;
    if as_json {
        println!("{}", report::json(device, &tables));
    } else {
        println!("{}", report::text(device, &tables));
    }
    Ok(())
}

/// The tables to print: the one asked for, or every personality the device
/// serves. A personality wider than one universe stays in the list and
/// carries its error, because the operator has to see that it is the width
/// that refused it.
fn tables(
    device: &govee_toolkit::codec::Device,
    personality: Option<&str>,
) -> Result<Vec<Result<Profile, profile::Error>>, Failure> {
    let Some(name) = personality else {
        return Ok(profile::served(device)
            .into_iter()
            .map(|personality| Profile::of(device, personality))
            .collect());
    };
    let personality = Personality::parse(name).ok_or_else(|| Failure {
        message: format!("unknown personality `{name}`; expected {}", spellings()),
        code: USAGE,
    })?;
    let table = Profile::of(device, personality).map_err(|error| Failure {
        message: error.to_string(),
        code: REFUSED,
    })?;
    Ok(vec![Ok(table)])
}

fn spellings() -> String {
    Personality::ALL
        .iter()
        .map(|personality| format!("`{personality}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
