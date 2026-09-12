//! `describe`: what a device file declares. Reads no hardware.

use govee_toolkit::codec::{ArgSpec, Command, Device, Mode, ModeSupport};
use govee_toolkit::stream::reach;
use govee_toolkit::{DeviceId, Govee};
use serde_json::{Value, json};

use crate::output::{Failure, Writer};
use crate::run::args::kind;

pub(super) fn run(govee: &Govee, writer: &Writer, target: &str) -> Result<(), Failure> {
    let device = resolve(govee, target)?;
    writer.emit(&as_json(device), &as_text(device));
    Ok(())
}

fn resolve<'a>(govee: &'a Govee, target: &str) -> Result<&'a Device, Failure> {
    let id = DeviceId::new(target);
    let sku = govee
        .devices()
        .into_iter()
        .find(|device| device.id == id)
        .map_or_else(|| target.to_owned(), |device| device.sku);
    Ok(govee.catalog().device(&sku)?)
}

fn as_json(device: &Device) -> Value {
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
        "modes": Mode::ALL
            .iter()
            .map(|mode| (mode.to_string(), mode_json(device, *mode)))
            .collect::<serde_json::Map<_, _>>(),
        "commands": Mode::ALL
            .iter()
            .map(|mode| (mode.to_string(), commands_json(device, *mode)))
            .collect::<serde_json::Map<_, _>>(),
        "verified": {
            "by": device.verified.by,
            "firmware": device.verified.firmware,
            "date": device.verified.date,
            "notes": device.verified.notes,
        },
    })
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
            .collect::<serde_json::Map<_, _>>(),
        "notes": support.notes,
    })
}

fn commands_json(device: &Device, mode: Mode) -> Value {
    device
        .commands
        .get(mode)
        .iter()
        .map(|(name, command)| (name.clone(), command_json(command)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

fn command_json(command: &Command) -> Value {
    json!({
        "cmd": command.cmd,
        "documented": command.documented,
        "role": command.role.map(|role| role.to_string()),
        "notes": command.notes,
        "args": command
            .args
            .iter()
            .map(|(name, spec)| (name.clone(), arg_json(spec)))
            .collect::<serde_json::Map<_, _>>(),
    })
}

fn arg_json(spec: &ArgSpec) -> Value {
    let mut value = json!({ "type": kind(spec), "role": spec.role().map(|role| role.to_string()) });
    let bound = match spec {
        ArgSpec::Int { range, .. } => json!(range),
        ArgSpec::Zones { count, .. } => json!(count),
        ArgSpec::RgbList { max_len, .. }
        | ArgSpec::String { max_len, .. }
        | ArgSpec::Bytes { max_len, .. } => json!(max_len),
    };
    if let Value::Object(fields) = &mut value {
        fields.insert("bound".to_owned(), bound);
    }
    value
}

fn as_text(device: &Device) -> String {
    let mut lines = vec![
        format!("{}  {}  family: {}", device.sku, device.name, device.family),
        format!("aliases: {}", join(&device.aliases)),
        format!("candidate aliases: {}", join(&device.candidate_aliases)),
        format!(
            "capabilities: {}",
            join(
                &device
                    .capabilities
                    .names()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            )
        ),
    ];
    if let Some(count) = device.capabilities.segment_count() {
        let pixels = device
            .capabilities
            .native_pixels()
            .map_or_else(|| "?".to_owned(), |value| value.to_string());
        lines.push(format!(
            "segments: {count} zones, {pixels} addressable LEDs"
        ));
        let points = &device.measurements.resolution_changepoints;
        if !points.is_empty() {
            lines.push(format!(
                "refines at: {}",
                points
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    for mode in Mode::ALL {
        let support = device.modes.get(mode);
        lines.push(format!("{mode}: {}", support.support));
        if let Some(reach) = reach(device, mode) {
            let native = if reach.native {
                "reaches every addressable LED"
            } else {
                "reaches no LED of its own"
            };
            lines.push(format!("  paints up to {} zones, {native}", reach.zones));
        }
        for (name, command) in device.commands.get(mode) {
            lines.push(format!("  {name}{}", role_of(command)));
            for (arg, spec) in &command.args {
                lines.push(format!("    {arg}: {}{}", kind(spec), bound_of(spec)));
            }
        }
    }
    lines.join("\n")
}

fn role_of(command: &Command) -> String {
    command
        .role
        .map_or_else(String::new, |role| format!("  role: {role}"))
}

fn bound_of(spec: &ArgSpec) -> String {
    match spec {
        ArgSpec::Int { range, .. } => format!(" [{}, {}]", range[0], range[1]),
        ArgSpec::Zones { count, .. } => count.map_or_else(String::new, |c| format!(" ({c} zones)")),
        ArgSpec::RgbList { max_len, .. }
        | ArgSpec::String { max_len, .. }
        | ArgSpec::Bytes { max_len, .. } => {
            max_len.map_or_else(String::new, |len| format!(" (at most {len})"))
        }
    }
}

fn join(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}
