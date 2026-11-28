use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::codec::Device;
use govee_toolkit::{
    Applied, Args, DeviceId, Devices, Drawing, Error, Filter, Outcome, Painting, Resolution,
    Supported, Verb,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::devices::{MODE, Sdk};
use crate::fault::failed;
use crate::refuse;
use crate::studio::Hex;

pub(crate) fn features(supported: &Supported, spec: &Device) -> Value {
    let segments = supported.segments.map(|segments| segments.zones);
    json!({
        "color": supported.color,
        "color_temp": supported.color_temp,
        "segments": segments,
        "gradient": segments.is_some() && supported.gradient,
        "draw": supported.draw.map(|cells| draw(spec, cells)),
    })
}

/// The LED count of the `draw` entry, and `[columns, rows]` when the file
/// records a `measurements.segment_grid` of that many cells.
fn draw(spec: &Device, cells: usize) -> Value {
    let grid = spec
        .measurements
        .extra
        .get("segment_grid")
        .and_then(|grid| serde_json::from_value::<[usize; 2]>(grid.clone()).ok())
        .filter(|[columns, rows]| columns * rows == cells);
    json!({ "cells": cells, "grid": grid })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Zones {
    /// Zero-based.
    zones: Vec<u16>,
    color: Hex,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Apply {
    ids: Vec<String>,
    power: Option<bool>,
    brightness: Option<i64>,
    color: Option<Hex>,
    /// In kelvin.
    color_temp: Option<i64>,
    segment: Option<Zones>,
    /// With `segment`, the paint carries it, and `None` sets it off.
    gradient: Option<bool>,
    /// A name or a code of the cached scene library of each model.
    scene: Option<String>,
    /// One color per LED of the `draw` entry, sent after the other changes.
    draw: Option<Vec<Hex>>,
}

impl Apply {
    fn verbs(self) -> Vec<Verb> {
        let mut verbs = Vec::new();
        verbs.extend(self.power.map(Verb::Power));
        verbs.extend(self.brightness.map(Verb::Brightness));
        verbs.extend(self.color.map(|hex| Verb::Color(hex.rgb())));
        verbs.extend(self.color_temp.map(Verb::ColorTemp));
        match self.segment {
            Some(segment) => verbs.push(Verb::Segment(Painting {
                zones: Some(segment.zones),
                colors: vec![segment.color.rgb()],
                resolution: Resolution::default(),
                gradient: self.gradient.unwrap_or(false),
            })),
            None => verbs.extend(self.gradient.map(Verb::Gradient)),
        }
        verbs.extend(self.scene.map(Verb::Scene));
        verbs
    }
}

fn outcome(id: &DeviceId, error: Option<&Error>) -> Value {
    json!({
        "id": id.to_string(),
        "ok": error.is_none(),
        "error": error.map(|error| json!({ "code": error.code(), "message": error.to_string() })),
    })
}

/// One outcome per device, in the order of `ids`. When a device fails, the
/// others go ahead.
pub(crate) async fn apply(State(sdk): State<Arc<Sdk>>, Json(mut body): Json<Apply>) -> Response {
    if body.ids.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "`ids` is empty");
    }
    if body
        .segment
        .as_ref()
        .is_some_and(|segment| segment.zones.is_empty())
    {
        return refuse(StatusCode::BAD_REQUEST, "`segment.zones` is empty");
    }
    let ids: Vec<DeviceId> = body.ids.iter().map(DeviceId::new).collect();
    if body.draw.as_ref().is_some_and(Vec::is_empty) {
        return refuse(StatusCode::BAD_REQUEST, "`draw` is empty");
    }
    let colors: Option<Vec<[u8; 3]>> = body
        .draw
        .take()
        .map(|draw| draw.iter().map(Hex::rgb).collect());
    let solid = body.color.is_some()
        || body.color_temp.is_some()
        || body.scene.is_some()
        || body.segment.is_some();
    let verbs = body.verbs();
    if verbs.is_empty() && colors.is_none() {
        return refuse(StatusCode::BAD_REQUEST, "the request changes nothing");
    }
    let applied = async {
        let govee = sdk.get().await?;
        let devices = govee.devices(Filter::ids(ids.clone()), Some(MODE)).await?;
        let applied = devices.apply(verbs).await;
        let drawn = match &colors {
            Some(colors) => draw_each(&devices, &applied, colors).await,
            None => Vec::new(),
        };
        govee_toolkit::Result::Ok((applied, drawn))
    }
    .await;
    let (applied, drawn) = match applied {
        Ok(answer) => answer,
        Err(error) => return failed(&error),
    };
    let drawn = drawn
        .iter()
        .filter_map(|done| done.result.as_ref().err().map(|error| (&done.id, error)));
    let errors: Vec<(&DeviceId, &Error)> = applied.failures().chain(drawn).collect();
    let devices: Vec<Value> = ids
        .iter()
        .map(|id| {
            let error = errors
                .iter()
                .find(|(failed, _)| *failed == id)
                .map(|(_, error)| *error);
            if error.is_none() {
                match &colors {
                    Some(colors) => sdk.drawings().set(id, colors),
                    None if solid => sdk.drawings().clear(id),
                    None => {}
                }
            }
            outcome(id, error)
        })
        .collect();
    Json(json!({ "devices": devices })).into_response()
}

/// Draw `colors` on every member that took every verb. A member that failed
/// before gets `Ok`, since its failure is already in `applied`.
async fn draw_each(
    devices: &Devices<'_>,
    applied: &Applied,
    colors: &[[u8; 3]],
) -> Vec<Outcome<()>> {
    let cells: Vec<u16> = (0..u16::try_from(colors.len()).unwrap_or(u16::MAX)).collect();
    let args = Args::new();
    let drawing = Drawing {
        cells: Some(&cells),
        colors,
        resolution: Resolution::default(),
        args: &args,
    };
    let skip: Vec<&DeviceId> = applied.failures().map(|(id, _)| id).collect();
    devices
        .each(|handle| {
            let skipped = skip.contains(&handle.id());
            async move {
                if skipped {
                    return Ok(());
                }
                handle.draw(&drawing).await.map(drop)
            }
        })
        .await
}
