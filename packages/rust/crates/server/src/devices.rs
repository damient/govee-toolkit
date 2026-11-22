//! `GET /api/devices`: the devices that answer a `lan` scan.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Config, Govee, Mode, Walk};
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

    pub(crate) fn storage(&self) -> Option<&Storage> {
        self.storage.as_ref()
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

fn failed(error: &govee_toolkit::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": error.to_string() })),
    )
        .into_response()
}

/// A scanned device, with the family of its device file for the icon of
/// Studio, and the name that `config.yaml` gives it now: a rename does not
/// restart the SDK.
fn record(govee: &Govee, device: &govee_toolkit::Device, config: Option<&Config>) -> Value {
    let mut value = device.to_json(Some(Mode::Lan));
    let family = govee
        .catalog()
        .device(&device.sku)
        .ok()
        .map(|file| file.family.clone());
    if let Some(fields) = value.as_object_mut() {
        fields.insert("family".to_owned(), json!(family));
        if let Some(config) = config {
            fields.insert("name".to_owned(), json!(config.name_for(&device.id)));
        }
    }
    value
}

pub(crate) async fn list(State(sdk): State<Arc<Sdk>>) -> Response {
    let found = async {
        let govee = sdk.get().await?;
        let devices = govee.scan(Some(&[Mode::Lan])).await?;
        let config = sdk.storage.as_ref().map(Storage::sdk_config).transpose()?;
        let records: Vec<Value> = devices
            .iter()
            .map(|device| record(&govee, device, config.as_ref()))
            .collect();
        govee_toolkit::Result::Ok(records)
    }
    .await;
    match found {
        Ok(devices) => Json(json!({ "devices": devices })).into_response(),
        Err(error) => failed(&error),
    }
}

/// `POST /api/devices/{id}/identify`: the walk of `govee identify` over
/// `lan`, for one device. The device goes off, shows green, and goes off
/// again, so the answer comes after some seconds.
pub(crate) async fn identify(State(sdk): State<Arc<Sdk>>, Path(id): Path<String>) -> Response {
    let report = async {
        let govee = sdk.get().await?;
        govee.identify(Some(&[id]), &Walk::default(), &()).await
    }
    .await;
    match report {
        Ok(report) => match report.summary("light") {
            None => Json(json!({})).into_response(),
            Some(line) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": line }))).into_response(),
        },
        Err(error) => failed(&error),
    }
}
