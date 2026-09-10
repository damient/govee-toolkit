//! The verbs a person types: `on`, `off`, `brightness`, `color`.
//!
//! Each one calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name reaches this file.

use govee_toolkit::{DeviceId, Govee, Served};
use serde_json::json;

use crate::output::{Failure, Writer};

/// What one verb does to one device.
pub(super) enum Verb {
    /// Turn on, or turn off.
    Power(bool),
    /// Set the brightness, in the unit the device file declares.
    Brightness(i64),
    /// Set one color over the whole device.
    Color([u8; 3]),
}

/// Run one verb and report the mode that served it.
pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    verb: Verb,
) -> Result<(), Failure> {
    let handle = govee.device(id);
    let served = match verb {
        Verb::Power(on) => handle.power(on).await,
        Verb::Brightness(level) => handle.brightness(level).await,
        Verb::Color(rgb) => handle.color(rgb).await,
    }?;
    report(writer, &served);
    Ok(())
}

/// Read `#RRGGBB`, or the same six digits with no `#`.
pub(super) fn rgb(text: &str) -> Result<[u8; 3], Failure> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    let bytes = (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
        .ok_or_else(|| Failure::usage(format!("`{text}` is not a color; write `#RRGGBB`")))?;
    Ok([
        u8::try_from(bytes >> 16 & 0xFF).unwrap_or_default(),
        u8::try_from(bytes >> 8 & 0xFF).unwrap_or_default(),
        u8::try_from(bytes & 0xFF).unwrap_or_default(),
    ])
}

fn report(writer: &Writer, served: &Served) {
    writer.emit(
        &json!({
            "id": served.id.to_string(),
            "mode": served.mode.to_string(),
            "command": served.command,
        }),
        &format!("{} {} served by {}", served.id, served.command, served.mode),
    );
}
