//! `GET /api/devices`: the devices that answer a `lan` scan.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Govee, Mode};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::storage::Storage;

/// Started on the first request, so a UI-only server opens no device socket.
#[derive(Debug, Default)]
pub(crate) struct Sdk {
    /// `None` reads the configuration of the system.
    storage: Option<Storage>,
    govee: Mutex<Option<Arc<Govee>>>,
}

impl Sdk {
    pub(crate) fn new(storage: Option<Storage>) -> Self {
        Self {
            storage,
            govee: Mutex::default(),
        }
    }

    async fn get(&self) -> govee_toolkit::Result<Arc<Govee>> {
        let mut govee = self.govee.lock().await;
        if let Some(govee) = &*govee {
            return Ok(Arc::clone(govee));
        }
        let config = self.storage.as_ref().map(Storage::sdk_config).transpose()?;
        let started = Arc::new(Govee::start_or_defaults(config, None).await?);
        *govee = Some(Arc::clone(&started));
        Ok(started)
    }

    /// Close the SDK. The next request starts it with the files as they are.
    pub(crate) async fn reset(&self) {
        if let Some(govee) = self.govee.lock().await.take() {
            // A close that fails leaves nothing to undo: the next start binds
            // again.
            let _ = govee.close().await;
        }
    }
}

pub(crate) async fn list(State(sdk): State<Arc<Sdk>>) -> Response {
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
