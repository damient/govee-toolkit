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
    let found = govee.scan().await?;
    report(&found, writer, restrict);
    Ok(())
}

/// Report the devices already known, without touching the network.
pub(super) fn list(govee: &Govee, writer: &Writer, restrict: Option<Mode>) {
    report(&govee.devices(), writer, restrict);
}

fn report(devices: &[Device], writer: &Writer, restrict: Option<Mode>) {
    for device in devices {
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
