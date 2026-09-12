//! The verbs a person types: `on`, `off`, `brightness`, `color`, `colortemp`,
//! `segment`, `music`.
//!
//! Each one calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name reaches this file.

use govee_toolkit::{DeviceId, Govee, Music, Served};
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
    /// Set the white temperature, in kelvin.
    ColorTemp(i64),
    /// Paint one color over zones. `None` paints every zone.
    Segment {
        /// The zones to paint, zero-based.
        zones: Option<Vec<u16>>,
        /// The color.
        rgb: [u8; 3],
        /// Ask the firmware to interpolate between zones.
        gradient: bool,
    },
    /// Play an effect the device renders from its own microphone.
    Music(Music),
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
        Verb::ColorTemp(kelvin) => handle.color_temp(kelvin).await,
        Verb::Segment {
            zones,
            rgb,
            gradient,
        } => handle.segment(zones.as_deref(), rgb, gradient).await,
        Verb::Music(music) => handle.music(&music).await,
    }?;
    report(writer, &served);
    Ok(())
}

/// Report one served command.
pub(super) fn report(writer: &Writer, served: &Served) {
    writer.emit(
        &json!({
            "id": served.id.to_string(),
            "mode": served.mode.to_string(),
            "command": served.command,
        }),
        &format!("{} {} served by {}", served.id, served.command, served.mode),
    );
}
