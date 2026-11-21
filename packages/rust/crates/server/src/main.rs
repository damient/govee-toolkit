//! `govee-server` — Studio, served from this machine.

// A binary reports and exits.
#![allow(clippy::print_stdout)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit_server::{DEFAULT_LISTEN, Options, Web};

/// Serve Studio: its API and its web UI.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    /// The address to listen on. Use `0.0.0.0:<port>` to reach Studio
    /// from another device on the network. The API has no authentication.
    #[arg(long, default_value = DEFAULT_LISTEN)]
    listen: SocketAddr,
    /// Serve the web UI from this directory, such as `apps/studio/dist`,
    /// and not from the copy built into the binary.
    #[arg(long)]
    web: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let writer = Writer::new(false);
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return writer.failure(&Failure::internal(e.to_string())),
    };
    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => writer.failure(&Failure::internal(e.to_string())),
    }
}

async fn run(cli: Cli) -> std::io::Result<()> {
    let listener = std::net::TcpListener::bind(cli.listen)?;
    println!("govee-server: http://{}", listener.local_addr()?);
    let options = Options {
        web: cli.web.map_or(Web::Embedded, Web::Dir),
    };
    tokio::select! {
        served = govee_toolkit_server::serve(listener, &options) => served,
        _ = tokio::signal::ctrl_c() => Ok(()),
    }
}
