//! `run`: receive Art-Net, and drive the patched devices.
//!
//! `--dry-run` stops before the send path. It prints every frame and what each
//! fixture reads out of it, and it writes to no device: that is what debugs a
//! patch before any device is at risk.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceHandle, Govee, Mode};
use govee_toolkit_dmx::apply::Timing;
use govee_toolkit_dmx::input::artnet::PORT;
use govee_toolkit_dmx::input::socket::Listener;
use govee_toolkit_dmx::node::{Node, Observer};
use govee_toolkit_dmx::patch::{Patch, Rig};

use super::observe::Printer;
use super::patch as patcher;
use super::rig::{configure, resolve};

/// The color the start pass stores on every fixture.
const BLACK: [u8; 3] = [0, 0, 0];

/// What the command line asks of one run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flags {
    /// Write to no device, and print every packet instead.
    pub(crate) dry_run: bool,
    /// Store no black on the devices at start.
    pub(crate) no_reset: bool,
    /// Print a line for each answered poll.
    pub(crate) debug: bool,
}

pub(crate) fn start(
    file: Option<&Path>,
    scan: Option<patcher::Options>,
    flags: Flags,
    config: Option<&Path>,
    writer: Writer,
) -> Result<(), Failure> {
    let path = patcher::file(file);
    // A file the operator named and misspelled must fail now, not after a
    // scan. `--scan` is the form that writes the file, so it checks nothing
    // here.
    if scan.is_none() {
        Patch::load(&path).map_err(|e| Failure::config(e.to_string()))?;
    }
    // One socket and a timer per fixture: the work never saturates a core, and
    // a worker pool costs the spawns at startup.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::internal(e.to_string()))?;
    runtime.block_on(run(&path, scan, flags, config, writer))
}

async fn run(
    file: &Path,
    scan: Option<patcher::Options>,
    flags: Flags,
    config: Option<&Path>,
    writer: Writer,
) -> Result<(), Failure> {
    let govee = Govee::start(configure(config)?)
        .await
        .map_err(|e| Failure::config(e.to_string()))?;
    let found = super::rig::scan(&govee).await?;
    if let Some(options) = scan {
        let written = patcher::update(&govee, &found, file, options)?;
        patcher::report(&written, file, writer);
    }
    let patch = Patch::load(file).map_err(|e| Failure::config(e.to_string()))?;
    let patch = &patch;
    let rig = resolve(&govee, patch)?;

    let address = SocketAddr::new(patch.node.bind, PORT);
    let listener = Listener::bind(address).map_err(|e| Failure::internal(e.to_string()))?;
    let timing = Timing {
        refresh: Duration::from_secs(patch.node.refresh_secs),
        silence: Duration::from_secs(patch.node.signal_loss_secs),
        off_delay: Duration::from_secs(patch.node.off_delay_secs),
    };
    let mut node = if flags.dry_run {
        Node::dry_run(rig)
    } else {
        Node::live(rig, &govee, timing)
    }
    .named(&patch.node.name);

    let mut printer = Printer::new(writer, flags.dry_run, flags.debug);
    printer.started(listener.local_addr(), patch, node.rig());
    if !flags.dry_run && !flags.no_reset {
        reset(&govee, node.rig(), &mut printer).await;
    }
    let outcome = node.run(&listener, shutdown(), &mut printer).await;
    printer.ended(&node.close().await);
    let released = govee
        .shutdown()
        .await
        .map_err(|e| Failure::internal(e.to_string()));
    outcome.map_err(|e| Failure::internal(e.to_string()))?;
    released
}

/// Store black on every driven fixture, and leave the fixture off.
///
/// A device shows the color that it held when it next comes on. A frame that
/// raises the dimmer powers the device on before the color of the frame
/// reaches it, so a device that holds a color from an earlier run shows that
/// color for a few milliseconds. Black shows nothing.
///
/// One task per fixture. A fixture that refuses the pass stops no other one:
/// the node reports it again on the first frame.
async fn reset(govee: &Govee, rig: &Rig, printer: &mut Printer) {
    let mut passes = Vec::with_capacity(rig.fixtures().len());
    for fixture in rig.fixtures() {
        let (govee, id) = (govee.clone(), fixture.entry.device.clone());
        passes.push(tokio::spawn(async move {
            let outcome = blackout(&govee.device_on(&id, Mode::Lan)).await;
            (id, outcome)
        }));
    }
    for pass in passes {
        if let Ok((id, Err(error))) = pass.await {
            printer.failed(&id, &error.to_string());
        }
    }
}

/// Power the device on, paint black, and power the device off. The device
/// takes a color while it is on, and it holds the last one it took.
async fn blackout(device: &DeviceHandle<'_>) -> govee_toolkit::Result<()> {
    let gap = device.command_gap()?;
    device.power(true).await?;
    tokio::time::sleep(gap).await;
    device.color(BLACK).await?;
    tokio::time::sleep(gap).await;
    device.power(false).await?;
    Ok(())
}

/// Wait for the operator to stop the node. A host with no signal handler
/// leaves the node running until it is killed, which is what a show wants.
async fn shutdown() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}
