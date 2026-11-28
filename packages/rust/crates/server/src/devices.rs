//! `GET /api/devices` reads `config.yaml` and answers at once. `GET /api/scan`
//! answers when the `lan` scan window ends.
//!
//! The server drives every light over [`MODE`] alone. `config.yaml` is shared
//! with the CLI, so it can hold an entry that this mode does not reach.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Config, DeviceConfig, DeviceId, Govee, Mode, Walk};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::drawings::Drawings;
use crate::fault::failed;
use crate::storage::Storage;
use crate::{refuse, rooms};

/// The one mode of the server. A light that the configuration does not enable
/// for it stays out of `GET /api/devices`.
pub(crate) const MODE: Mode = Mode::Lan;

/// Starts on the first request, so that a UI-only server opens no socket.
#[derive(Debug, Default)]
pub(crate) struct Sdk {
    /// `None` reads the configuration of the system.
    storage: Option<Storage>,
    rooms: Arc<rooms::Store>,
    govee: Mutex<Option<Arc<Govee>>>,
    drawings: Drawings,
}

impl Sdk {
    pub(crate) fn new(storage: Option<Storage>, rooms: Arc<rooms::Store>) -> Self {
        Self {
            storage,
            rooms,
            govee: Mutex::default(),
            drawings: Drawings::default(),
        }
    }

    pub(crate) async fn get(&self) -> govee_toolkit::Result<Arc<Govee>> {
        let mut govee = self.govee.lock().await;
        if let Some(govee) = &*govee {
            return Ok(Arc::clone(govee));
        }
        let config = self.read_config().await?;
        let started = Arc::new(Govee::start_or_defaults(config, None).await?);
        *govee = Some(Arc::clone(&started));
        Ok(started)
    }

    /// Current, because every write of the server calls [`Sdk::reload`].
    fn config<'a>(&self, govee: &'a Govee) -> Option<&'a Config> {
        self.storage.is_some().then(|| govee.config())
    }

    pub(crate) fn storage(&self) -> Option<&Storage> {
        self.storage.as_ref()
    }

    pub(crate) fn drawings(&self) -> &Drawings {
        &self.drawings
    }

    pub(crate) fn rooms(&self) -> Arc<rooms::Store> {
        Arc::clone(&self.rooms)
    }

    async fn read_config(&self) -> govee_toolkit::Result<Option<Config>> {
        let Some(storage) = self.storage.clone() else {
            return Ok(None);
        };
        let path = storage.config.display().to_string();
        let read = tokio::task::spawn_blocking(move || storage.sdk_config()).await;
        read.map_err(|e| govee_toolkit::Error::Config {
            path,
            reason: e.to_string(),
        })?
        .map(Some)
    }

    /// When [`Govee::reconfigure`] fails, the SDK closes and the next request
    /// starts it again.
    pub(crate) async fn reload(&self) {
        let mut govee = self.govee.lock().await;
        let Some(current) = govee.take() else {
            return;
        };
        let next = match self.read_config().await {
            Ok(Some(config)) => current.reconfigure(config).ok(),
            _ => None,
        };
        if next.is_none() {
            // A failed close leaves nothing to undo.
            let _ = current.close().await;
        }
        *govee = next.map(Arc::new);
    }
}

fn family(govee: &Govee, sku: Option<&str>) -> Option<String> {
    let file = govee.catalog().device(sku?).ok()?;
    Some(file.family.clone())
}

fn scanned(govee: &Govee, device: &govee_toolkit::Device, config: Option<&Config>) -> Value {
    let mut value = device.to_json(Some(MODE));
    if let Some(fields) = value.as_object_mut() {
        fields.insert("family".to_owned(), json!(family(govee, Some(&device.sku))));
        let name = config.and_then(|config| config.name_for(&device.id));
        let configured = config.is_some_and(|config| config.devices.contains_key(&device.id));
        fields.insert("name".to_owned(), json!(name));
        fields.insert("configured".to_owned(), json!(configured));
    }
    value
}

fn entry(govee: &Govee, id: &DeviceId, entry: &DeviceConfig) -> Value {
    json!({
        "id": id.to_string(),
        "sku": entry.sku,
        "name": entry.name,
        "family": family(govee, entry.sku.as_deref()),
    })
}

/// The entries that [`MODE`] reaches.
fn reached(config: &Config) -> impl Iterator<Item = (&DeviceId, &DeviceConfig)> {
    config
        .devices
        .iter()
        .filter(|(id, _)| config.modes_for(id).contains(&MODE))
}

pub(crate) async fn list(State(sdk): State<Arc<Sdk>>) -> Response {
    let found = async {
        let govee = sdk.get().await?;
        govee_toolkit::Result::Ok(
            sdk.config(&govee)
                .into_iter()
                .flat_map(reached)
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

pub(crate) async fn scan(State(sdk): State<Arc<Sdk>>) -> Response {
    let found = async {
        let govee = sdk.get().await?;
        let devices = govee.scan(Some(&[MODE])).await?;
        let config = sdk.config(&govee);
        govee_toolkit::Result::Ok(
            devices
                .iter()
                .map(|device| scanned(&govee, device, config))
                .collect::<Vec<_>>(),
        )
    }
    .await;
    match found {
        Ok(devices) => Json(json!({ "devices": devices })).into_response(),
        Err(error) => failed(&error),
    }
}

/// Answers when the walk of `govee identify` ends, after some seconds.
pub(crate) async fn identify(State(sdk): State<Arc<Sdk>>, Path(id): Path<String>) -> Response {
    let report = async {
        let govee = sdk.get().await?;
        govee.identify(Some(&[id]), &Walk::default(), &()).await
    }
    .await;
    match report {
        Ok(report) => match report.summary("light") {
            None => Json(json!({})).into_response(),
            Some(line) => refuse(StatusCode::SERVICE_UNAVAILABLE, &line),
        },
        Err(error) => failed(&error),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const CONFIG: &str = "
defaults:
  modes: [lan]
devices:
  AA:BB:CC:DD:EE:01: {sku: H6008}
  AA:BB:CC:DD:EE:02: {sku: H6008, modes: [lan, cloud]}
  5B05DEE1-F68B-26C3-785D-04752C1A0333: {sku: H6008, modes: [ble]}
";

    #[test]
    fn an_entry_that_the_mode_does_not_reach_stays_out_of_the_list() {
        let config: Config = serde_norway::from_str(CONFIG).unwrap();
        let mut ids: Vec<String> = reached(&config).map(|(id, _)| id.to_string()).collect();
        ids.sort();
        assert_eq!(ids, ["AA:BB:CC:DD:EE:01", "AA:BB:CC:DD:EE:02"]);
    }
}
