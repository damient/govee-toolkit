//! `GET /api/devices/{id}/status`: what the home view of Studio reads. The
//! call goes over the modes that `config.yaml` enables for the device, and
//! fails when none reaches it.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{DeviceId, Supported};
use serde_json::json;

use crate::devices::{Sdk, failed};
use crate::{apply, refuse};

/// `GET /api/devices/{id}/status`: ask the device for its state. The record
/// adds `brightness_range`, the bounds of `brightness` in `POST /api/apply`
/// on the mode that answered, or `null`, and `features`, what
/// [`apply::features`] gives for that mode. The first call for a device that
/// no transport knows scans for it, until the device answers.
pub(crate) async fn status(State(sdk): State<Arc<Sdk>>, Path(id): Path<String>) -> Response {
    let govee = match sdk.get().await {
        Ok(govee) => govee,
        Err(error) => return failed(&error),
    };
    let answer = async {
        let handle = govee.device(&DeviceId::new(&id), None)?;
        handle.ensure_known().await?;
        let call = handle.resolve()?;
        let supported = Supported::of(call.spec(), call.mode());
        Ok::<_, govee_toolkit::Error>((call.status().await?, supported))
    }
    .await;
    match answer {
        Ok((status, supported)) => {
            let mut record = status.to_json();
            if let Some(fields) = record.as_object_mut() {
                fields.insert("brightness_range".to_owned(), json!(supported.brightness));
                fields.insert("features".to_owned(), apply::features(&supported));
            }
            Json(record).into_response()
        }
        Err(error) => refuse(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}
