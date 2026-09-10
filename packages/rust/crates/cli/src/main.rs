//! `govee` — the command line over `govee-toolkit`.
//!
//! ```text
//! govee scan
//! govee devices
//! govee send <device> <command> --arg brightness=50
//! ```
//!
//! Every subcommand takes `--json`, which is the form a script and a model
//! read. The text form is for a person and its layout is not stable.

// A binary reports and exits; the no-panic rule that protects a host
// application from a library does not apply here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use clap::Parser;

mod cli;
mod output;
mod run;

use crate::cli::Cli;
use crate::output::{Failure, Writer};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let writer = Writer::new(cli.global.json);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return writer.failure(&Failure::internal(e.to_string())),
    };

    match runtime.block_on(run::dispatch(&cli, &writer)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(failure) => writer.failure(&failure),
    }
}
