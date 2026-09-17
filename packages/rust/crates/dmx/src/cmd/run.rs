//! `run`: receive Art-Net, and drive the patched devices.
//!
//! `--dry-run` stops before the send path. It prints every frame and what each
//! fixture reads out of it, and it writes to no device: that is what debugs a
//! patch before any device is at risk.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use govee_toolkit::codec::Device;
use govee_toolkit::{Config, DeviceId, Govee, Mode};
use govee_toolkit_dmx::input::artnet::PORT;
use govee_toolkit_dmx::input::socket::Listener;
use govee_toolkit_dmx::node::Node;
use govee_toolkit_dmx::patch::{Patch, Rig};

use super::observe::Printer;
use super::{CONFIG, Failure, INTERNAL, UNREACHABLE};

pub(crate) fn start(
    file: &Path,
    dry_run: bool,
    config: Option<&Path>,
    as_json: bool,
) -> Result<(), Failure> {
    let patch = Patch::load(file).map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    // One socket and a timer per fixture: the work never saturates a core, and
    // a worker pool costs the spawns at startup.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    runtime.block_on(run(&patch, dry_run, config, as_json))
}

async fn run(
    patch: &Patch,
    dry_run: bool,
    config: Option<&Path>,
    as_json: bool,
) -> Result<(), Failure> {
    let govee = Govee::start(configure(config)?)
        .await
        .map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    // No mode keeps a record across runs, so the patch is joined to what this
    // run found.
    govee
        .scan()
        .await
        .map_err(|e| Failure::new(e.to_string(), UNREACHABLE))?;
    let rig = resolve(&govee, patch)?;

    let address = SocketAddr::new(patch.node.bind, PORT);
    let listener = Listener::bind(address).map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    let mut node = if dry_run {
        Node::dry_run(rig)
    } else {
        Node::live(rig, &govee, Duration::from_secs(patch.node.refresh_secs))
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

fn configure(path: Option<&Path>) -> Result<Config, Failure> {
    let config = match path {
        Some(path) => Config::load_from(path),
        None => Config::load(),
    };
    config.map_err(|e| Failure::new(e.to_string(), CONFIG))
}

/// The patch, joined to the devices this run found.
fn resolve(govee: &Govee, patch: &Patch) -> Result<Rig, Failure> {
    let found = govee.devices();
    let mut known: BTreeMap<DeviceId, &Device> = BTreeMap::new();
    for device in &found {
        if let Ok(file) = govee.catalog().device(&device.sku) {
            known.insert(device.id.clone(), file);
        }
    }
    let rig = patch
        .resolve(|id| known.get(id).copied())
        .map_err(|errors| Failure::new(lines(&errors), CONFIG))?;
    lan_enabled(govee, &rig)?;
    Ok(rig)
}

/// Every patched device must have `lan` enabled. The bridge reaches a device
/// over `lan` and substitutes no other mode, so this fails at the start rather
/// than at the first frame.
fn lan_enabled(govee: &Govee, rig: &Rig) -> Result<(), Failure> {
    let without: Vec<String> = rig
        .fixtures()
        .iter()
        .map(|fixture| &fixture.entry.device)
        .filter(|id| !govee.device(id).modes().contains(&Mode::Lan))
        .map(ToString::to_string)
        .collect();
    if without.is_empty() {
        return Ok(());
    }
    Err(Failure::new(
        format!(
            "these devices enable no `lan` mode: {}; the bridge drives a device over `lan` alone",
            without.join(", ")
        ),
        UNREACHABLE,
    ))
}

/// Every fault of a patch, one per line: an operator corrects the whole patch
/// once.
fn lines<E: ToString>(errors: &[E]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
