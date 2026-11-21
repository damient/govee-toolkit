//! `GET /api/devices`: the devices that answer a `lan` scan.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Govee, Mode};
use serde_json::{Value, json};
use tokio::sync::OnceCell;

/// Started on the first request, so a UI-only server opens no device socket.
#[derive(Debug, Default)]
pub(crate) struct Sdk(OnceCell<Govee>);

impl Sdk {
    async fn get(&self) -> govee_toolkit::Result<&Govee> {
        self.0
            .get_or_try_init(|| Govee::start_or_defaults(None, None))
            .await
    }
}

pub(crate) async fn list(State(sdk): State<std::sync::Arc<Sdk>>) -> Response {
    let found = async { sdk.get().await?.scan(Some(&[Mode::Lan])).await }.await;
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
