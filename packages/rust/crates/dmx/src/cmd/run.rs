//! `run`: receive Art-Net, and drive the patched devices.
//!
//! `--dry-run` stops before the send path. It prints every frame and what each
//! fixture reads out of it, and it writes to no device: that is what debugs a
//! patch before any device is at risk.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use govee_toolkit::Govee;
use govee_toolkit_dmx::apply::Timing;
use govee_toolkit_dmx::input::artnet::PORT;
use govee_toolkit_dmx::input::socket::Listener;
use govee_toolkit_dmx::node::Node;
use govee_toolkit_dmx::patch::Patch;

use super::observe::Printer;
use super::rig::{configure, resolve};
use super::{CONFIG, Failure, INTERNAL, UNREACHABLE, patch as writer};

pub(crate) fn start(
    file: Option<&Path>,
    scan: Option<writer::Options>,
    dry_run: bool,
    config: Option<&Path>,
    as_json: bool,
) -> Result<(), Failure> {
    let path = writer::file(file);
    // A file the operator named and misspelled must fail now, not after a
    // scan. `--scan` is the form that writes the file, so it checks nothing
    // here.
    if scan.is_none() {
        Patch::load(&path).map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    }
    // One socket and a timer per fixture: the work never saturates a core, and
    // a worker pool costs the spawns at startup.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    runtime.block_on(run(&path, scan, dry_run, config, as_json))
}

async fn run(
    file: &Path,
    scan: Option<writer::Options>,
    dry_run: bool,
    config: Option<&Path>,
    as_json: bool,
) -> Result<(), Failure> {
    let govee = Govee::start(configure(config)?)
        .await
        .map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    let found = govee
        .scan()
        .await
        .map_err(|e| Failure::new(e.to_string(), UNREACHABLE))?;
    if let Some(options) = scan {
        let written = writer::update(&govee, &found, file, options)?;
        writer::report(&written, file, as_json);
    }
    let patch = Patch::load(file).map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    let patch = &patch;
    let rig = resolve(&govee, patch)?;

    let address = SocketAddr::new(patch.node.bind, PORT);
    let listener = Listener::bind(address).map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    let timing = Timing {
        refresh: Duration::from_secs(patch.node.refresh_secs),
        silence: Duration::from_secs(patch.node.signal_loss_secs),
    };
    let mut node = if dry_run {
        Node::dry_run(rig)
    } else {
        Node::live(rig, &govee, timing)
    }
    .named(&patch.node.name);

    let mut printer = Printer::new(as_json, dry_run);
    printer.started(listener.local_addr(), patch, node.rig());
    let outcome = node.run(&listener, shutdown(), &mut printer).await;
    printer.ended(&node.close().await);
    let released = govee
        .shutdown()
        .await
        .map_err(|e| Failure::new(e.to_string(), INTERNAL));
    outcome.map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    released
}

/// Wait for the operator to stop the node. A host with no signal handler
/// leaves the node running until it is killed, which is what a show wants.
async fn shutdown() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}
