//! `status`: what the device reports about itself.

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, DeviceStatus, Govee};
use serde_json::{Value, json};

use crate::output::{Failure, Writer, option};

/// Ask the device for its state and report the answer.
pub(super) async fn run(govee: &Govee, writer: &Writer, id: &DeviceId) -> Result<(), Failure> {
    let handle = govee.device(id);
    let mode = handle.serving_mode()?;
    let status = handle.status().await?;
    writer.emit(&as_json(&status, mode), &as_text(&status, mode));
    Ok(())
}

fn as_json(status: &DeviceStatus, mode: Mode) -> Value {
    json!({
        "id": status.id.to_string(),
        "mode": mode.to_string(),
        "on": status.on,
        "brightness": status.brightness,
        "color": status.color.map(hex),
        "color_temp_kelvin": status.color_temp_kelvin,
        "raw": status.raw,
    })
}

fn as_text(status: &DeviceStatus, mode: Mode) -> String {
    let field = |name: &str, value: Option<String>| format!("{name}={}", option(value));
    format!(
        "{}  {}  {}  {}  {}  {}",
        status.id,
        mode,
        field("on", status.on.map(|on| on.to_string())),
        field("brightness", status.brightness.map(|v| v.to_string())),
        field("color", status.color.map(hex)),
        field("kelvin", status.color_temp_kelvin.map(|v| v.to_string())),
    )
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
