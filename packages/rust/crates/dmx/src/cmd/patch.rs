//! `patch`: scan the LAN, and write the rig the node drives.
//!
//! The command appends. It never moves an address that is already in the
//! file, because that address is patched on a desk too. `--reset` is the one
//! form that starts over, and it keeps the file it replaces under `.bak`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{Device, DeviceId, Govee, Mode};
use govee_toolkit_dmx::patch::{self, Candidate, Layout, Patch, Plan, PortAddress, Skip, stamp};
use serde_json::{Value, json};

use super::rig::{configure, lines, scan};

/// What the command line asks of one scan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options {
    /// The personality a new entry takes.
    pub(crate) layout: Layout,
    /// The lowest port-address a new entry lands on.
    pub(crate) first: PortAddress,
    /// Start the file over, at the cost of every address in it.
    pub(crate) reset: bool,
    /// Report the entries and write nothing.
    pub(crate) dry_run: bool,
}

/// What one scan did to the file.
#[derive(Debug)]
pub(crate) struct Written {
    plan: Plan,
    /// The entries whose `enabled:` the scan changed, and what it wrote.
    flipped: Vec<(DeviceId, bool)>,
    /// The entries `hold:` kept, and the state the scan would have written.
    held: Vec<(DeviceId, bool)>,
    /// The instant the scan ran.
    scanned: String,
    /// The devices the bridge drives through nothing: no `lan` mode, or no
    /// device file.
    ignored: Vec<Skip>,
    /// Whether the file changed.
    wrote: bool,
}

/// The file the command line names, or [`patch::default_path`].
pub(crate) fn file(named: Option<&Path>) -> PathBuf {
    named.map_or_else(patch::default_path, Path::to_path_buf)
}

pub(crate) fn start(
    named: Option<&Path>,
    options: Options,
    config: Option<&Path>,
    writer: Writer,
) -> Result<(), Failure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::internal(e.to_string()))?;
    let path = file(named);
    runtime.block_on(async {
        let govee = Govee::start(configure(config)?)
            .await
            .map_err(|e| Failure::config(e.to_string()))?;
        let found = scan(&govee).await?;
        let outcome = update(&govee, &found, &path, options);
        govee
            .shutdown()
            .await
            .map_err(|e| Failure::internal(e.to_string()))?;
        report(&outcome?, &path, writer);
        Ok(())
    })
}

/// Write what `found` answered into `path`, and answer what changed.
///
/// `found` is what this scan reached, not what `govee` knows: a device cache
/// holds a device that was on the network last week, and a patch built from
/// that would carry a fixture nothing can drive.
///
/// # Errors
///
/// [`Failure::config`] where the file cannot be read or written, does not
/// parse, or states no width for an entry whose device did not answer.
pub(crate) fn update(
    govee: &Govee,
    found: &[Device],
    path: &Path,
    options: Options,
) -> Result<Written, Failure> {
    let text = read(path, options.reset)?;
    let current = Patch::parse(&text, &path.display().to_string())
        .map_err(|e| Failure::config(e.to_string()))?;
    let mut ignored = Vec::new();
    let mut candidates = Vec::new();
    for device in found {
        if !device.modes.contains(&Mode::Lan) {
            ignored.push(skip(device, "enables no `lan` mode"));
            continue;
        }
        match govee.catalog().device(&device.sku) {
            Ok(file) => candidates.push(Candidate {
                id: &device.id,
                device: file,
            }),
            Err(error) => ignored.push(skip(device, &error.to_string())),
        }
    }
    let catalog = govee.catalog();
    let plan = current
        .plan(&candidates, options.layout, options.first, |sku| {
            catalog.device(sku).ok()
        })
        .map_err(|errors| Failure::config(lines(&errors)))?;
    let answered: BTreeSet<DeviceId> = found.iter().map(|device| device.id.clone()).collect();
    let states: BTreeMap<DeviceId, bool> = current
        .patch
        .iter()
        .filter(|entry| !entry.hold)
        .map(|entry| (entry.device.clone(), answered.contains(&entry.device)))
        .collect();
    let flipped = flipped(&current, &states);
    let held = held(&current, &answered);
    let scanned = stamp::now();
    let wrote = !options.dry_run;
    if wrote {
        let text = patch::update(&text, &plan.added, &states, &scanned);
        write(path, &text, options.reset)?;
    }
    Ok(Written {
        plan,
        flipped,
        held,
        scanned,
        ignored,
        wrote,
    })
}

/// The entries `hold:` keeps, and the state a scan would have written on each.
///
/// An entry that already carries that state is not held against anything, and
/// is left out.
fn held(current: &Patch, answered: &BTreeSet<DeviceId>) -> Vec<(DeviceId, bool)> {
    current
        .patch
        .iter()
        .filter(|entry| entry.hold)
        .map(|entry| (entry.device.clone(), answered.contains(&entry.device)))
        .filter(|(device, state)| {
            current
                .patch
                .iter()
                .any(|entry| &entry.device == device && entry.enabled != *state)
        })
        .collect()
}

/// The entries whose state the scan changes.
///
/// The scan owns `enabled:`, in both directions: a device that stops answering
/// leaves the rig and keeps its channels, and the same device takes them up
/// again once it answers.
fn flipped(current: &Patch, states: &BTreeMap<DeviceId, bool>) -> Vec<(DeviceId, bool)> {
    current
        .patch
        .iter()
        .filter_map(|entry| {
            let state = states.get(&entry.device).copied()?;
            (state != entry.enabled).then(|| (entry.device.clone(), state))
        })
        .collect()
}

fn skip(device: &Device, reason: &str) -> Skip {
    Skip {
        device: device.id.clone(),
        sku: device.sku.clone(),
        reason: reason.to_owned(),
    }
}

/// The file as it stands, and nothing where it is missing or reset.
fn read(path: &Path, reset: bool) -> Result<String, Failure> {
    if reset || !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(path)
        .map_err(|e| Failure::config(format!("cannot read the patch `{}`: {e}", path.display())))
}

/// Write the file, and keep the one a reset replaces.
fn write(path: &Path, text: &str, reset: bool) -> Result<(), Failure> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| Failure::config(format!("cannot create `{}`: {e}", parent.display())))?;
    }
    if reset && path.exists() {
        let backup = path.with_extension("yaml.bak");
        std::fs::rename(path, &backup).map_err(|e| {
            Failure::config(format!(
                "cannot keep the patch as `{}`: {e}",
                backup.display()
            ))
        })?;
    }
    std::fs::write(path, text)
        .map_err(|e| Failure::config(format!("cannot write the patch `{}`: {e}", path.display())))
}

/// What the scan did, for a person and for a machine.
pub(crate) fn report(written: &Written, path: &Path, writer: Writer) {
    writer.emit(&record(written, path), &text(written, path));
}

/// What the scan did, as one record.
fn record(written: &Written, path: &Path) -> Value {
    let plan = &written.plan;
    let added: Vec<Value> = plan
        .added
        .iter()
        .map(|placement| {
            json!({
                "device": placement.device.to_string(),
                "sku": placement.sku,
                "personality": placement.personality.as_str(),
                "universe": placement.universe.get(),
                "address": placement.address,
                "width": placement.width,
            })
        })
        .collect();
    json!({
        "event": "patched",
        "file": path.display().to_string(),
        "written": written.wrote,
        "scanned": written.scanned,
        "kept": plan.kept,
        "added": added,
        "enabled": states(&written.flipped, true),
        "disabled": states(&written.flipped, false),
        "held": states(&written.held, true)
            .into_iter()
            .chain(states(&written.held, false))
            .collect::<Vec<_>>(),
        "skipped": records(&plan.skipped),
        "ignored": records(&written.ignored),
    })
}

/// What the scan did, as the lines an operator reads.
fn text(written: &Written, path: &Path) -> String {
    let plan = &written.plan;
    let mut lines = vec![
        format!("patch {}", path.display()),
        format!("scanned {}", written.scanned),
        format!("kept {} entries", plan.kept),
    ];
    for (device, state) in &written.held {
        lines.push(format!(
            "held {device}  `hold:` keeps it, and the scan reads it {}",
            answer(*state)
        ));
    }
    for (device, state) in &written.flipped {
        if *state {
            lines.push(format!("enabled {device}  it answers again"));
        } else {
            lines.push(format!(
                "disabled {device}  it did not answer, and keeps its channels"
            ));
        }
    }
    for placement in &plan.added {
        lines.push(format!(
            "added {}  {}  {}  universe {}  channels {} to {}",
            placement.device,
            placement.sku,
            placement.personality,
            placement.universe,
            placement.address,
            placement.address + placement.width - 1
        ));
    }
    for skipped in plan.skipped.iter().chain(&written.ignored) {
        lines.push(format!(
            "skipped {}  {}: {}",
            skipped.device, skipped.sku, skipped.reason
        ));
    }
    if plan.added.is_empty() && written.flipped.is_empty() && written.held.is_empty() {
        lines.push("nothing to change".to_owned());
    }
    if !written.wrote {
        lines.push("dry run: the file is unchanged".to_owned());
    }
    lines.join("\n")
}

/// How the scan read a device.
const fn answer(answered: bool) -> &'static str {
    if answered { "on the network" } else { "absent" }
}

/// The devices the scan flipped one way.
fn states(flipped: &[(DeviceId, bool)], wanted: bool) -> Vec<Value> {
    flipped
        .iter()
        .filter(|(_, state)| *state == wanted)
        .map(|(device, _)| Value::String(device.to_string()))
        .collect()
}

fn records(skips: &[Skip]) -> Vec<Value> {
    skips
        .iter()
        .map(|skip| {
            json!({
                "device": skip.device.to_string(),
                "sku": skip.sku,
                "reason": skip.reason,
            })
        })
        .collect()
}
