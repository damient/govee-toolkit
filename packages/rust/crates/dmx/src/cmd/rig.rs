//! What every subcommand that reaches a device does first: read the
//! configuration, and join the patch to the devices the SDK knows.

use std::collections::BTreeMap;
use std::path::Path;

use govee_toolkit::codec::Device;
use govee_toolkit::{Config, DeviceId, Govee, Mode};
use govee_toolkit_dmx::patch::{Patch, Rig};

use super::{CONFIG, Failure, UNREACHABLE};

/// The patch, joined to every device the SDK knows.
///
/// A device the cache carries counts here, and a device that misses one scan
/// therefore starts the run. The backoff of the send path is what reports it
/// where it stays silent — `docs/dmx.md`.
///
/// # Errors
///
/// [`CONFIG`] for every fault of the patch, and [`UNREACHABLE`] where a
/// patched device enables no `lan` mode.
pub(crate) fn resolve(govee: &Govee, patch: &Patch) -> Result<Rig, Failure> {
    let found = govee.devices();
    let mut known: BTreeMap<DeviceId, &Device> = BTreeMap::new();
    for device in &found {
        if let Ok(file) = govee.catalog().device(&device.sku) {
            known.insert(device.id.clone(), file);
        }
    }
    let catalog = govee.catalog();
    let rig = patch
        .resolve(|id| known.get(id).copied(), |sku| catalog.device(sku).ok())
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

/// The configuration the command line names, or the one `govee` reads.
///
/// # Errors
///
/// [`CONFIG`] where the file cannot be read or cannot be applied.
pub(crate) fn configure(path: Option<&Path>) -> Result<Config, Failure> {
    let config = match path {
        Some(path) => Config::load_from(path),
        None => Config::load(),
    };
    config.map_err(|e| Failure::new(e.to_string(), CONFIG))
}

/// Every fault of a patch, one per line: an operator corrects the whole patch
/// once.
pub(crate) fn lines<E: ToString>(errors: &[E]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
