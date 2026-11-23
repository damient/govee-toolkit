//! `/api/settings`. Every client that the server admits can change the
//! settings. `server` is read-only.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::studio::{Store, StudioSettings};
use crate::{refuse, storage};

fn answer(settings: StudioSettings) -> Response {
    let mut value = serde_json::to_value(settings).unwrap_or_default();
    if let Value::Object(fields) = &mut value {
        fields.insert(
            "server".to_owned(),
            json!({ "version": env!("CARGO_PKG_VERSION"), "sdk": govee_toolkit::VERSION }),
        );
    }
    Json(value).into_response()
}

pub(crate) async fn get(State(store): State<Arc<Store>>) -> Response {
    answer(store.get().studio.settings)
}

pub(crate) async fn put(State(store): State<Arc<Store>>, Json(body): Json<Value>) -> Response {
    let Value::Object(changes) = body else {
        return refuse(StatusCode::BAD_REQUEST, "expected an object");
    };
    let mut merged = match serde_json::to_value(store.get().studio.settings) {
        Ok(Value::Object(current)) => current,
        _ => serde_json::Map::new(),
    };
    merged.extend(changes);
    let prefs: StudioSettings = match serde_json::from_value(Value::Object(merged)) {
        Ok(prefs) => prefs,
        Err(e) => return refuse(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    let saved = storage::blocking(move || store.update(|studio| studio.studio.settings = prefs));
    match saved.await {
        Ok(studio) => answer(studio.studio.settings),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
