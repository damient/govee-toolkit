//! `/api/settings`: the Studio settings, which every client that the server
//! admits can read and change.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::studio::{Store, StudioSettings};

pub(crate) fn refuse(status: StatusCode, error: &str) -> Response {
    (status, Json(json!({ "error": error }))).into_response()
}

/// `GET /api/settings`: `{ "theme", "language", "advanced" }`.
pub(crate) async fn get(State(store): State<Arc<Store>>) -> Json<StudioSettings> {
    Json(store.get().studio.settings)
}

/// `PUT /api/settings` with the keys to change: the same answer as `GET`.
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
    match store.update(|studio| studio.studio.settings = prefs) {
        Ok(studio) => Json(studio.studio.settings).into_response(),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
