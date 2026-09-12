//! Each verb calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name reaches this file.

use govee_toolkit::stream::Resolution;
use govee_toolkit::{DeviceId, Govee, Music, Paint, Served};
use serde_json::json;

use crate::output::{Failure, Writer};

pub(super) enum Verb {
    Power(bool),
    Brightness(i64),
    Color([u8; 3]),
    ColorTemp(i64),
    Segment {
        zones: Option<Vec<u16>>,
        colors: Vec<[u8; 3]>,
        resolution: Resolution,
        gradient: bool,
    },
    Gradient(bool),
    Music(Music),
}

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
            colors,
            resolution,
            gradient,
        } => {
            handle
                .segment(&Paint {
                    zones: zones.as_deref(),
                    colors: &colors,
                    resolution,
                    gradient,
                })
                .await
        }
        Verb::Gradient(on) => handle.gradient(on).await,
        Verb::Music(music) => handle.music(&music).await,
    }?;
    report(writer, &served);
    Ok(())
}

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
