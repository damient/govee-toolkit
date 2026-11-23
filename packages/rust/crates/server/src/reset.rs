//! `POST /api/reset`: put the Studio settings back to the defaults, remove
//! the device entries of `config.yaml` and the rooms, or both. The other keys
//! of the file stay, and so do the API key and the sharing switch.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use govee_toolkit::config::DeviceEdit;
use serde::Deserialize;
use serde_json::json;

use crate::backup::Backup;
use crate::share::Local;
use crate::studio::StudioSettings;
use crate::{refuse, refuse_io, storage};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reset {
    /// Theme, language, the `advanced` switch and the favorite colors.
    #[serde(default)]
    prefs: bool,
    /// The `devices:` key of `config.yaml` (names, modes and groups), and
    /// `rooms.json`.
    #[serde(default)]
    devices: bool,
}

/// Only a client on this machine can reset, as for an import.
pub(crate) async fn reset(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
    Json(body): Json<Reset>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    if !body.prefs && !body.devices {
        return refuse(StatusCode::BAD_REQUEST, "choose what to reset");
    }
    let path = storage.config.clone();
    let rooms = Arc::clone(&backup.rooms);
    let store = Arc::clone(&backup.store);
    let Reset { prefs, devices } = body;
    let written = storage::blocking(move || {
        if devices {
            storage::edit_config(&path, &[DeviceEdit::RemoveAll])?;
            rooms.set(None, false)?;
        }
        if prefs {
            store.update(|studio| studio.studio.settings = StudioSettings::default())?;
        }
        Ok(())
    })
    .await;
    if let Err(error) = written {
        return refuse_io(&error);
    }
    if devices {
        backup.sdk.reload().await;
    }
    Json(json!({ "prefs": prefs, "devices": devices })).into_response()
}
