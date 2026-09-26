//! `describe`: what a device file declares. Reads no hardware.
//!
//! The record is the crate's own, so a binding prints the same one. The text
//! form is this file's, and its layout can change at any release.

use govee_toolkit::codec::{ArgBound, ArgSpec, Command, Device, Geometry, Mode};
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::stream::reach;
use govee_toolkit::{DeviceId, Govee, describe};

pub(super) fn run(govee: &Govee, writer: Writer, target: &str) -> Result<(), Failure> {
    let device = resolve(govee, target)?;
    writer.emit(&describe(device), &as_text(device));
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
    match device.geometry {
        Some(Geometry::Line { length_m }) => lines.push(format!("geometry: {length_m} m long")),
        Some(Geometry::Surface { width_m, height_m }) => {
            lines.push(format!("geometry: {width_m} m wide, {height_m} m high"));
        }
        _ => {}
    }
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
                lines.push(format!("    {arg}: {}{}", spec.kind(), bound_of(spec)));
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
    match spec.bound() {
        ArgBound::Range([min, max]) => format!(" [{min}, {max}]"),
        ArgBound::Zones(count) => format!(" ({count} zones)"),
        ArgBound::MaxLen(len) => format!(" (at most {len})"),
        ArgBound::None => String::new(),
    }
}

fn join(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}
