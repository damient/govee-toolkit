//! `GET /api/devices/{id}/status`, `POST /api/devices/{id}/brightness` and
//! `POST /api/power`: what the home view of Studio reads and sends. Each call
//! goes over the modes that `config.yaml` enables for the device, and fails
//! when none reaches it.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::codec::{ArgBound, ArgRole, Device, Mode, Role};
use govee_toolkit::{DeviceId, Filter};
use serde::Deserialize;
use serde_json::json;

use crate::devices::{Sdk, failed};

fn unreached(error: &govee_toolkit::Error) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({ "error": error.to_string() })),
    )
        .into_response()
}

/// The bounds of the argument `role: brightness` of the entry `role:
/// brightness` on `mode`. `None` when the device file declares no such entry.
fn brightness_range(device: &Device, mode: Mode) -> Option<[i64; 2]> {
    let (_, command) = device.entry_for(mode, Role::Brightness)?;
    let spec = command.args.get(command.arg_for(ArgRole::Brightness)?)?;
    match spec.bound() {
        ArgBound::Range(pair) => Some(pair),
        _ => None,
    }
}

/// `GET /api/devices/{id}/status`: ask the device for its state. The record
/// adds `brightness_range`, the bounds that `POST
/// /api/devices/{id}/brightness` takes on the mode that answered, or `null`.
/// The first call for a device scans for it, so the answer can take the scan
/// window.
pub(crate) async fn status(State(sdk): State<Arc<Sdk>>, Path(id): Path<String>) -> Response {
    let govee = match sdk.get().await {
        Ok(govee) => govee,
        Err(error) => return failed(&error),
    };
    let answer = async {
        let handle = govee.device(&DeviceId::new(&id), None)?;
        handle.ensure_known().await?;
        let call = handle.resolve()?;
        let range = brightness_range(call.spec(), call.mode());
        Ok::<_, govee_toolkit::Error>((call.status().await?, range))
    }
    .await;
    match answer {
        Ok((status, range)) => {
            let mut record = status.to_json();
            if let Some(fields) = record.as_object_mut() {
                fields.insert("brightness_range".to_owned(), json!(range));
            }
            Json(record).into_response()
        }
        Err(error) => unreached(&error),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Brightness {
    level: i64,
}

/// `POST /api/devices/{id}/brightness`: set the brightness, in the bounds
/// that `status` gives. A value out of the bounds is an error, never a clamp.
pub(crate) async fn brightness(
    State(sdk): State<Arc<Sdk>>,
    Path(id): Path<String>,
    Json(body): Json<Brightness>,
) -> Response {
    let govee = match sdk.get().await {
        Ok(govee) => govee,
        Err(error) => return failed(&error),
    };
    let sent = async {
        let handle = govee.device(&DeviceId::new(&id), None)?;
        handle.ensure_known().await?;
        handle.brightness(body.level).await
    }
    .await;
    match sent {
        Ok(_) => Json(json!({})).into_response(),
        Err(error) => unreached(&error),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Power {
    ids: Vec<String>,
    on: bool,
}

/// `POST /api/power`: turn every device of `ids` on or off at once. The
/// answer gives one outcome per device. When a device fails, the others go
/// ahead.
pub(crate) async fn power(State(sdk): State<Arc<Sdk>>, Json(body): Json<Power>) -> Response {
    let outcomes = async {
        let govee = sdk.get().await?;
        let devices = govee
            .devices(Filter::ids(body.ids.iter().map(DeviceId::new)), None)
            .await?;
        let known = devices.ensure_known().await;
        let outcomes = devices.power(body.on).await;
        // A device that no scan found reports the scan error, which names the
        // cause, before the send error.
        govee_toolkit::Result::Ok(
            outcomes
                .iter()
                .zip(&known)
                .map(|(sent, found)| match found.error() {
                    Some(error) => json!({
                        "id": sent.id.to_string(),
                        "ok": false,
                        "error": { "code": error.code(), "message": error.to_string() },
                    }),
                    None => sent.to_json(),
                })
                .collect::<Vec<_>>(),
        )
    }
    .await;
    match outcomes {
        Ok(devices) => Json(json!({ "devices": devices })).into_response(),
        Err(error) => failed(&error),
    }
}
