//! `GET /api/devices`: the devices that answer a `lan` scan.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Govee, Mode};
use serde_json::{Value, json};
use tokio::sync::OnceCell;

/// The SDK, started on the first request: a server that only serves the UI
/// opens no device socket.
#[derive(Debug, Default)]
pub(crate) struct Sdk(OnceCell<Govee>);

impl Sdk {
    async fn get(&self) -> govee_toolkit::Result<&Govee> {
        self.0
            .get_or_try_init(|| Govee::start_or_defaults(None, None))
            .await
    }
}

/// Each request runs a scan, so the answer takes one `lan` scan window.
pub(crate) async fn list(State(sdk): State<std::sync::Arc<Sdk>>) -> Response {
    let found = match sdk.get().await {
        Ok(govee) => govee.scan(Some(&[Mode::Lan])).await,
        Err(error) => Err(error),
    };
    match found {
        Ok(devices) => {
            let devices: Vec<Value> = devices
                .iter()
                .map(|device| device.to_json(Some(Mode::Lan)))
                .collect();
            Json(json!({ "devices": devices })).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}
