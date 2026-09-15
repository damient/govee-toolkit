//! What a device file declares, as the record every surface prints.
//!
//! The serialized [`Device`] is the file itself. This is the reader's view of
//! it: the modes in one place, the commands under the mode that carries them,
//! and each argument's type, role and bound. Reads no hardware.

use serde_json::{Map, Value, json};

use crate::codec::{ArgSpec, Bounds, Command, Device, Mode, ModeSupport};
use crate::stream::reach;

/// One device file, as the record `govee describe --json` prints and every
/// binding hands a caller.
#[must_use]
pub fn describe(device: &Device) -> Value {
    json!({
        "sku": device.sku,
        "name": device.name,
        "family": device.family,
        "aliases": device.aliases,
        "candidate_aliases": device.candidate_aliases,
        "capabilities": device.capabilities.names().collect::<Vec<_>>(),
        "segments": {
            "count": device.capabilities.segment_count(),
            "native_pixels": device.capabilities.native_pixels(),
            "refines_at": device.measurements.resolution_changepoints,
        },
        "modes": by_mode(|mode| mode_json(device, mode)),
        "commands": by_mode(|mode| commands_json(device, mode)),
        "verified": {
            "by": device.verified.by,
            "firmware": device.verified.firmware,
            "date": device.verified.date,
            "notes": device.verified.notes,
        },
    })
}

/// One entry per mode, so a reader finds every mode whether or not the file
/// declares anything for it.
fn by_mode(of: impl Fn(Mode) -> Value) -> Value {
    Mode::ALL
        .iter()
        .map(|mode| (mode.to_string(), of(*mode)))
        .collect::<Map<_, _>>()
        .into()
}

fn mode_json(device: &Device, mode: Mode) -> Value {
    let support: &ModeSupport = device.modes.get(mode);
    json!({
        "support": support.support.to_string(),
        "capabilities": support.capabilities.resolve(&device.capabilities),
        "segments": reach(device, mode).map(|reach| json!({
            "zones": reach.zones,
            "native": reach.native,
        })),
        "unreachable": support
            .unreachable
            .iter()
            .map(|(name, reason)| (name.clone(), Value::from(reason.to_string())))
            .collect::<Map<_, _>>(),
        "notes": support.notes,
    })
}

fn commands_json(device: &Device, mode: Mode) -> Value {
    device
        .commands
        .get(mode)
        .iter()
        .map(|(name, command)| (name.clone(), command_json(command)))
        .collect::<Map<_, _>>()
        .into()
}

fn command_json(command: &Command) -> Value {
    json!({
        "cmd": command.cmd,
        "role": command.role.map(|role| role.to_string()),
        "notes": command.notes,
        "args": command
            .args
            .iter()
            .map(|(name, spec)| (name.clone(), arg_json(spec)))
            .collect::<Map<_, _>>(),
    })
}

fn arg_json(spec: &ArgSpec) -> Value {
    json!({
        "type": spec.kind(),
        "role": spec.role().map(|role| role.to_string()),
        "bound": bound(spec),
    })
}

/// What the file bounds this argument by: the pair an `int` takes, the zones a
/// mask counts, or the length the others cap.
fn bound(spec: &ArgSpec) -> Value {
    match spec {
        ArgSpec::Int { range, .. } => json!(range.as_ref().and_then(Bounds::pair)),
        ArgSpec::Zones { count, .. } => json!(count),
        ArgSpec::RgbList { max_len, .. }
        | ArgSpec::String { max_len, .. }
        | ArgSpec::Bytes { max_len, .. } => json!(max_len),
    }
}
