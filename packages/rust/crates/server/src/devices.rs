//! `GET /api/devices`: the devices of `config.yaml`. `GET /api/scan`: the
//! devices that answer a `lan` scan. The first answers at once, the second
//! after the scan window.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Config, DeviceConfig, DeviceId, Govee, Mode, Walk};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::rooms;
use crate::storage::Storage;

/// Started on the first request, so a UI-only server opens no device socket.
#[derive(Debug, Default)]
pub(crate) struct Sdk {
    /// `None` reads the configuration of the system.
    storage: Option<Storage>,
    /// A device that leaves `config.yaml` leaves its rooms too.
    rooms: Arc<rooms::Store>,
    govee: Mutex<Option<Arc<Govee>>>,
}

impl Sdk {
    pub(crate) fn new(storage: Option<Storage>, rooms: Arc<rooms::Store>) -> Self {
        Self {
            storage,
            rooms,
            govee: Mutex::default(),
        }
    }

    pub(crate) async fn get(&self) -> govee_toolkit::Result<Arc<Govee>> {
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

    pub(crate) fn rooms(&self) -> &rooms::Store {
        &self.rooms
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

pub(crate) fn failed(error: &govee_toolkit::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": error.to_string() })),
    )
        .into_response()
}

fn family(govee: &Govee, sku: Option<&str>) -> Option<String> {
    let file = govee.catalog().device(sku?).ok()?;
    Some(file.family.clone())
}

/// A device that answered the scan, with the family of its device file for
/// the icon of Studio, and its entry in `config.yaml`. The file is read on
/// each call: a write does not restart the SDK.
fn scanned(govee: &Govee, device: &govee_toolkit::Device, config: Option<&Config>) -> Value {
    let mut value = device.to_json(Some(Mode::Lan));
    if let Some(fields) = value.as_object_mut() {
        fields.insert("family".to_owned(), json!(family(govee, Some(&device.sku))));
        let name = config.and_then(|config| config.name_for(&device.id));
        let configured = config.is_some_and(|config| config.devices.contains_key(&device.id));
        fields.insert("name".to_owned(), json!(name));
        fields.insert("configured".to_owned(), json!(configured));
    }
    value
}

/// A device of `config.yaml`. `sku` and `family` are `null` when the entry
/// gives no SKU.
fn entry(govee: &Govee, id: &DeviceId, entry: &DeviceConfig) -> Value {
    json!({
        "id": id.to_string(),
        "sku": entry.sku,
        "name": entry.name,
        "family": family(govee, entry.sku.as_deref()),
    })
}

/// `GET /api/devices`: the devices of `config.yaml`, with no scan.
pub(crate) async fn list(State(sdk): State<Arc<Sdk>>) -> Response {
    let found = async {
        let govee = sdk.get().await?;
        let config = sdk.storage.as_ref().map(Storage::sdk_config).transpose()?;
        let records = config.iter().flat_map(|config| &config.devices);
        govee_toolkit::Result::Ok(
            records
                .map(|(id, record)| entry(&govee, id, record))
                .collect::<Vec<_>>(),
        )
    }
    .await;
    match found {
        Ok(devices) => Json(json!({ "devices": devices })).into_response(),
        Err(error) => failed(&error),
    }
}

/// `GET /api/scan`: the devices that answer a `lan` scan. The answer comes
/// when the scan window ends.
pub(crate) async fn scan(State(sdk): State<Arc<Sdk>>) -> Response {
    let found = async {
        let govee = sdk.get().await?;
        let devices = govee.scan(Some(&[Mode::Lan])).await?;
        let config = sdk.storage.as_ref().map(Storage::sdk_config).transpose()?;
        govee_toolkit::Result::Ok(
            devices
                .iter()
                .map(|device| scanned(&govee, device, config.as_ref()))
                .collect::<Vec<_>>(),
        )
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
