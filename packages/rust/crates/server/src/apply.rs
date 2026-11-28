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

/// A `segment`: one color over a list of zones, or one color per zone.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Zones {
    Listed(Listed),
    Every(Every),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Listed {
    /// Zero-based.
    zones: Vec<u16>,
    color: Hex,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Every {
    /// As many as `features.segments`.
    colors: Vec<Hex>,
}

impl Zones {
    fn is_empty(&self) -> bool {
        match self {
            Self::Listed(listed) => listed.zones.is_empty(),
            Self::Every(every) => every.colors.is_empty(),
        }
    }

    fn painting(self, gradient: bool) -> Painting {
        let (zones, colors, resolution) = match self {
            Self::Listed(Listed { zones, color }) => {
                (Some(zones), vec![color.rgb()], Resolution::default())
            }
            Self::Every(Every { colors }) => {
                // The count of the list, so that the mask takes one color
                // each and spreads nothing.
                let count = u16::try_from(colors.len()).unwrap_or(u16::MAX);
                let colors = colors.iter().map(Hex::rgb).collect();
                (None, colors, Resolution::Exact(count))
            }
        };
        Painting {
            zones,
            colors,
            resolution,
            gradient,
        }
    }
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
            Some(segment) => verbs.push(Verb::Segment(
                segment.painting(self.gradient.unwrap_or(false)),
            )),
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
    if body.segment.as_ref().is_some_and(Zones::is_empty) {
        return refuse(StatusCode::BAD_REQUEST, "`segment` names no zone");
    }
    let ids: Vec<DeviceId> = body.ids.iter().map(DeviceId::new).collect();
    if body.draw.as_ref().is_some_and(Vec::is_empty) {
        return refuse(StatusCode::BAD_REQUEST, "`draw` is empty");
    }
    let drawing: Option<Arc<[Hex]>> = body.draw.take().map(Into::into);
    let colors: Option<Vec<[u8; 3]>> = drawing
        .as_ref()
        .map(|draw| draw.iter().map(Hex::rgb).collect());
    let verbs = body.verbs();
    if verbs.is_empty() && colors.is_none() {
        return refuse(StatusCode::BAD_REQUEST, "the request changes nothing");
    }
    // A change that repaints every LED replaces the drawing.
    let solid = verbs.iter().any(|verb| {
        matches!(
            verb,
            Verb::Color(_) | Verb::ColorTemp(_) | Verb::Scene(_) | Verb::Segment(_)
        )
    });
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
                match &drawing {
                    Some(drawing) => sdk.drawings().set(id, drawing),
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
