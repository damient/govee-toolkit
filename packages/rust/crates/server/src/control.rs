use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use govee_toolkit::{DeviceId, Supported};
use serde_json::json;

use crate::apply;
use crate::devices::{MODE, Sdk};
use crate::fault::{failed, offline};

/// `brightness_range` is `null` when the mode that answered has none. The first
/// call for a device that no transport knows scans until the device answers.
/// A device that does not answer gets `200` with `online: false`, `reason` and
/// `code`: that is a state of the light, not a fault of the request.
pub(crate) async fn status(State(sdk): State<Arc<Sdk>>, Path(id): Path<String>) -> Response {
    let govee = match sdk.get().await {
        Ok(govee) => govee,
        Err(error) => return failed(&error),
    };
    let answer = async {
        let handle = govee.device(&DeviceId::new(&id), Some(MODE))?;
        handle.ensure_known().await?;
        let call = handle.resolve()?;
        let supported = Supported::of(call.spec(), call.mode());
        let features = apply::features(&supported, call.spec(), call.mode());
        Ok::<_, govee_toolkit::Error>((call.status().await?, supported, features))
    }
    .await;
    match answer {
        Ok((status, supported, features)) => {
            let mut record = status.to_json();
            if let Some(fields) = record.as_object_mut() {
                fields.insert("online".to_owned(), json!(true));
                fields.insert("brightness_range".to_owned(), json!(supported.brightness));
                fields.insert("features".to_owned(), features);
                fields.insert(
                    "drawing".to_owned(),
                    sdk.drawings().get(&DeviceId::new(&id)),
                );
            }
            Json(record).into_response()
        }
        Err(error) if offline(&error) => Json(json!({
            "online": false,
            "reason": error.to_string(),
            "code": error.code(),
        }))
        .into_response(),
        Err(error) => failed(&error),
    }
}
