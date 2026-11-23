//! `POST /api/apply`: change the state of a list of devices in one call. The
//! SDK sends power on first and power off last, and a device that fails takes
//! no later step. Each step goes over the mode that `config.yaml` enables for
//! the device, and fails when that mode does not carry the role.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{DeviceId, Error, Filter, Painting, Resolution, Supported, Verb};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::devices::{Sdk, failed};
use crate::refuse;
use crate::studio::Hex;

/// What `POST /api/apply` can send on a mode, for the `features` of
/// `GET /api/devices/{id}/status`. A mode that arms a segment channel gets
/// `segments: null`: Studio paints zones in the normal mode of the device
/// only. `gradient` needs `segments`.
pub(crate) fn features(supported: &Supported) -> Value {
    let segments = supported
        .segments
        .filter(|segments| !segments.armed)
        .map(|segments| segments.zones);
    json!({
        "color": supported.color,
        "color_temp": supported.color_temp,
        "segments": segments,
        "gradient": segments.is_some() && supported.gradient,
    })
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
    /// In kelvin, in the bounds of `features.color_temp`.
    color_temp: Option<i64>,
    segment: Option<Zones>,
    /// With `segment`, the paint carries it. A paint with no `gradient`
    /// sets it off.
    gradient: Option<bool>,
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

/// `POST /api/apply` with `{ "ids", "power", "brightness", "color",
/// "color_temp", "segment", "gradient" }`. Each key but `ids` is optional. The
/// answer gives one outcome per device, in the order of `ids`, with the first
/// error of the device. When a device fails, the others go ahead.
pub(crate) async fn apply(State(sdk): State<Arc<Sdk>>, Json(body): Json<Apply>) -> Response {
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
    let verbs = body.verbs();
    if verbs.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "the request changes nothing");
    }
    let applied = async {
        let govee = sdk.get().await?;
        let devices = govee.devices(Filter::ids(ids.clone()), None).await?;
        govee_toolkit::Result::Ok(devices.apply(verbs).await)
    }
    .await;
    let applied = match applied {
        Ok(applied) => applied,
        Err(error) => return failed(&error),
    };
    let devices: Vec<Value> = ids
        .iter()
        .map(|id| {
            let error = applied
                .failures()
                .find(|(failed, _)| *failed == id)
                .map(|(_, error)| error);
            outcome(id, error)
        })
        .collect();
    Json(json!({ "devices": devices })).into_response()
}
