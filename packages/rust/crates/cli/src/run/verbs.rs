//! Each verb calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name reaches this file.
//!
//! A member that fails stops no other one: its line goes to stderr, and the
//! run exits with the code of the first failure.

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::stream::Resolution;
use govee_toolkit::{DeviceId, Govee, GroupHandle, Music, Outcome, Paint, Served};
use serde_json::json;

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
    writer: Writer,
    members: &[DeviceId],
    restrict: Option<Mode>,
    verb: Verb,
) -> Result<(), Failure> {
    let mut failures: Vec<(DeviceId, Failure)> = Vec::new();
    let mut reached = Vec::new();
    for outcome in govee.group_maybe_on(members, restrict).ensure_known().await {
        match outcome.result {
            Ok(_) => reached.push(outcome.id),
            Err(error) => failures.push((outcome.id, Failure::from(error))),
        }
    }
    for outcome in play(&govee.group_maybe_on(&reached, restrict), verb).await {
        match outcome.result {
            Ok(served) => report(writer, &served),
            Err(error) => failures.push((outcome.id, Failure::from(error))),
        }
    }
    verdict(writer, members.len(), failures)
}

async fn play(group: &GroupHandle<'_>, verb: Verb) -> Vec<Outcome> {
    match verb {
        Verb::Power(on) => group.power(on).await,
        Verb::Brightness(level) => group.brightness(level).await,
        Verb::Color(rgb) => group.color(rgb).await,
        Verb::ColorTemp(kelvin) => group.color_temp(kelvin).await,
        Verb::Segment {
            zones,
            colors,
            resolution,
            gradient,
        } => {
            group
                .segment(&Paint {
                    zones: zones.as_deref(),
                    colors: &colors,
                    resolution,
                    gradient,
                })
                .await
        }
        Verb::Gradient(on) => group.gradient(on).await,
        Verb::Music(music) => group.music(&music).await,
    }
}

/// One device fails with its own failure, as a command on one device does.
fn verdict(
    writer: Writer,
    count: usize,
    mut failures: Vec<(DeviceId, Failure)>,
) -> Result<(), Failure> {
    if count == 1 {
        return failures.pop().map_or(Ok(()), |(_, failure)| Err(failure));
    }
    for (id, failure) in &failures {
        writer.warn(
            &json!({
                "id": id.to_string(),
                "error": { "kind": failure.kind(), "message": failure.message() },
            }),
            &format!("{id} failed: {}", failure.message()),
        );
    }
    let names: Vec<String> = failures.iter().map(|(id, _)| id.to_string()).collect();
    let failed = failures.len();
    match failures.into_iter().next() {
        None => Ok(()),
        Some((_, first)) => Err(first.with_message(format!(
            "{failed} of {count} devices failed: {}",
            names.join(", ")
        ))),
    }
}

pub(super) fn report(writer: Writer, served: &Served) {
    writer.emit(
        &json!({
            "id": served.id.to_string(),
            "mode": served.mode.to_string(),
            "command": served.command,
        }),
        &format!("{} {} served by {}", served.id, served.command, served.mode),
    );
}
