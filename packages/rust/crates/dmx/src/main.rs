//! `govee-dmx` — the Art-Net node over `govee-toolkit`.

// A binary reports and exits; the no-panic rule that protects a host
// application from a library does not apply here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "govee-dmx",
    version,
    about = "Drive Govee devices from a lighting desk, over Art-Net. Unofficial.",
    arg_required_else_help = true
)]
struct Cli {}

fn main() {
    let Cli {} = Cli::parse();
}
