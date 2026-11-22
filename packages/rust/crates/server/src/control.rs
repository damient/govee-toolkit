//! `GET /api/devices/{id}/status`: what the home view of Studio reads. The
//! call goes over the modes that `config.yaml` enables for the device, and
//! fails when none reaches it.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::DeviceId;
use govee_toolkit::codec::{ArgBound, ArgRole, Device, Mode, Role};
use serde_json::json;

use crate::apply;
use crate::devices::{Sdk, failed};

fn unreached(error: &govee_toolkit::Error) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({ "error": error.to_string() })),
    )
        .into_response()
}

/// The bounds of the argument `role: brightness` of the entry `role:
/// brightness` on `mode`, which `POST /api/apply` takes. `None` when the device
/// file declares no such entry.
fn brightness_range(device: &Device, mode: Mode) -> Option<[i64; 2]> {
    let (_, command) = device.entry_for(mode, Role::Brightness)?;
    let spec = command.args.get(command.arg_for(ArgRole::Brightness)?)?;
    match spec.bound() {
        ArgBound::Range(pair) => Some(pair),
        _ => None,
    }
}

/// `GET /api/devices/{id}/status`: ask the device for its state. The record
/// adds `brightness_range`, the bounds of `brightness` in `POST /api/apply`
/// on the mode that answered, or `null`, and `features`, what
/// [`apply::features`] gives for that mode. The first call for a device scans
/// for it, so the answer can take the scan window.
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
        let features = apply::features(call.spec(), call.mode());
        Ok::<_, govee_toolkit::Error>((call.status().await?, range, features))
    }
    .await;
    match answer {
        Ok((status, range, features)) => {
            let mut record = status.to_json();
            if let Some(fields) = record.as_object_mut() {
                fields.insert("brightness_range".to_owned(), json!(range));
                fields.insert("features".to_owned(), features);
            }
            Json(record).into_response()
        }
        Err(error) => unreached(&error),
    }
}
