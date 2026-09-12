//! `scan` and `devices`: what answered, and what is already known.

use govee_toolkit::codec::Mode;
use govee_toolkit::{Device, Govee};
use serde_json::{Value, json};

use crate::output::{Failure, Writer};

/// Discover devices and report them.
pub(super) async fn scan(
    govee: &Govee,
    writer: &Writer,
    restrict: Option<Mode>,
) -> Result<(), Failure> {
    // `--mode` restricts the wire the scan touches, and not only what is
    // printed: a scan over another mode would send frames the caller ruled
    // out.
    let modes = restrict.map_or_else(|| govee.modes(), |mode| vec![mode]);
    let found = govee.scan_on(&modes).await?;
    for device in &found {
        // A device that answered is on the air, whatever the configuration
        // says about it. Printing only the ones it enables would hide a device
        // the scan just heard: `ble` reports a device under the handle the
        // platform gives the peripheral, and that handle is in no
        // configuration until somebody puts it there.
        let enabled = device.modes.iter().any(|mode| modes.contains(mode));
        writer.emit(
            &heard_json(device, restrict, enabled),
            &heard_text(device, &modes, restrict, enabled),
        );
    }
    Ok(())
}

/// One device a scan heard, and whether the configuration enables the mode it
/// answered over.
fn heard_json(device: &Device, restrict: Option<Mode>, enabled: bool) -> Value {
    let mut json = as_json(device, restrict);
    if let Value::Object(fields) = &mut json {
        fields.insert("enabled".to_owned(), Value::Bool(enabled));
    }
    json
}

fn heard_text(device: &Device, scanned: &[Mode], restrict: Option<Mode>, enabled: bool) -> String {
    if enabled {
        return as_text(device, restrict);
    }
    let over: Vec<String> = scanned.iter().map(ToString::to_string).collect();
    format!(
        "{}  {}  {}  answered over {}, which the configuration does not enable for it",
        device.id,
        device.sku,
        device.name.as_deref().unwrap_or("-"),
        over.join(", ")
    )
}

/// Report the devices already known, without touching the network.
pub(super) fn list(govee: &Govee, writer: &Writer, restrict: Option<Mode>) {
    report(&govee.devices(), writer, restrict);
}

/// What `devices` reports: the devices a command can go to, so one that does
/// not enable the mode is left out. A scan reports what answered instead.
fn report(devices: &[Device], writer: &Writer, restrict: Option<Mode>) {
    for device in devices {
        if restrict.is_some_and(|only| !device.modes.contains(&only)) {
            continue;
        }
        writer.emit(&as_json(device, restrict), &as_text(device, restrict));
    }
}

fn as_json(device: &Device, restrict: Option<Mode>) -> Value {
    let modes: Vec<Value> = modes(device, restrict)
        .map(|mode| {
            json!({
                "mode": mode.to_string(),
                "health": device.health.get(&mode).map(|health| json!({
                    "state": health.state.to_string(),
                    "failures": health.failures,
                    "available": health.available,
                })),
            })
        })
        .collect();
    json!({
        "id": device.id.to_string(),
        "sku": device.sku,
        "name": device.name,
        "modes": modes,
    })
}

fn as_text(device: &Device, restrict: Option<Mode>) -> String {
    let modes: Vec<String> = modes(device, restrict)
        .map(|mode| match device.health.get(&mode) {
            Some(health) => format!("{mode}={}", health.state),
            None => mode.to_string(),
        })
        .collect();
    format!(
        "{}  {}  {}  [{}]",
        device.id,
        device.sku,
        device.name.as_deref().unwrap_or("-"),
        modes.join(" ")
    )
}

// `--mode` narrows what is reported to one mode. It never adds a mode the
// configuration leaves out.
fn modes(device: &Device, restrict: Option<Mode>) -> impl Iterator<Item = Mode> + '_ {
    device
        .modes
        .iter()
        .copied()
        .filter(move |mode| restrict.is_none_or(|only| only == *mode))
}

/// Report everything wrong with the configuration. Reads no hardware.
pub(super) fn doctor(govee: &Govee, writer: &Writer) {
    let problems = govee.problems();
    let json = json!({
        "problems": problems
            .iter()
            .map(|problem| json!({
                "device": problem.device.as_ref().map(ToString::to_string),
                "message": problem.message,
            }))
            .collect::<Vec<_>>(),
    });
    let text = if problems.is_empty() {
        "no problem found".to_owned()
    } else {
        problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    };
    writer.emit(&json, &text);
}
