//! `PUT /api/devices`: the device entries of `config.yaml`, in one write. A
//! device that the request removes leaves its rooms too.
//!
//! [`config::edit_devices`] makes the edit, so the other keys of the file stay
//! as they are. The comments of the file do not survive a write.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::config::DeviceEdit;
use govee_toolkit::{DeviceId, Mode};
use serde::Deserialize;
use serde_json::json;

use crate::devices::Sdk;
use crate::rooms::NAME_MAX;
use crate::{refuse, refuse_io, storage};

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Entries {
    keep: Vec<Keep>,
    remove: Vec<DeviceId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Keep {
    id: DeviceId,
    /// Written in an entry that has no SKU. A SKU in the file stays.
    #[serde(default)]
    sku: Option<String>,
    /// `null` or a blank string removes the name.
    #[serde(default)]
    name: Option<String>,
}

impl Entries {
    /// A new entry gets `modes: [lan]`, because the device answered a `lan`
    /// scan. An entry with no SKU gets the SKU of the request, so that a read
    /// with no scan knows the device file.
    fn edits(&self) -> Vec<DeviceEdit> {
        let mut edits = Vec::new();
        for keep in &self.keep {
            edits.push(DeviceEdit::Add {
                id: keep.id.clone(),
                modes: vec![Mode::Lan],
            });
            if let Some(sku) = &keep.sku {
                edits.push(DeviceEdit::Sku {
                    id: keep.id.clone(),
                    sku: sku.clone(),
                });
            }
            edits.push(DeviceEdit::Name {
                id: keep.id.clone(),
                name: keep.name.clone(),
            });
        }
        edits.extend(self.remove.iter().cloned().map(DeviceEdit::Remove));
        edits
    }
}

/// Write the entries, then give them to the SDK. A request that changes no
/// entry writes nothing.
pub(crate) async fn put(State(sdk): State<Arc<Sdk>>, Json(entries): Json<Entries>) -> Response {
    let Some(storage) = sdk.storage() else {
        return refuse(
            StatusCode::NOT_FOUND,
            "this server keeps no configuration file",
        );
    };
    let long = entries
        .keep
        .iter()
        .filter_map(|keep| keep.name.as_deref())
        .any(|name| name.trim().chars().count() > NAME_MAX);
    if long {
        return refuse(
            StatusCode::BAD_REQUEST,
            &format!("a name is longer than {NAME_MAX} characters"),
        );
    }
    let path = storage.config.clone();
    let edits = entries.edits();
    match storage::blocking(move || storage::edit_config(&path, &edits)).await {
        Ok(true) => sdk.reload().await,
        Ok(false) => {}
        Err(error) => return refuse_io(&error),
    }
    let rooms = sdk.rooms();
    if let Err(error) = storage::blocking(move || rooms.forget(&entries.remove)).await {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    Json(json!({})).into_response()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use govee_toolkit::config;

    use super::*;

    #[test]
    fn a_keep_adds_the_entry_with_lan_and_its_sku() {
        let entries: Entries =
            serde_json::from_str(r#"{"keep":[{"id":"aa:bb","sku":"H6022","name":"desk"}]}"#)
                .unwrap();
        let text = config::edit_devices(Path::new("c"), "", &entries.edits())
            .unwrap()
            .unwrap();
        let saved: govee_toolkit::Config = serde_norway::from_str(&text).unwrap();
        let id = DeviceId::new("AA:BB");
        assert_eq!(saved.modes_for(&id), [Mode::Lan]);
        assert_eq!(saved.sku_for(&id), Some("H6022"));
        assert_eq!(saved.name_for(&id), Some("desk"));
    }
}
