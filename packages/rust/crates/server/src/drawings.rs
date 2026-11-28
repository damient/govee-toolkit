//! The last drawing that the server sent to each light, so that a client that
//! opens later can show it. The light keeps playing a drawing after the
//! request, and answers no read of it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use govee_toolkit::DeviceId;
use serde_json::{Value, json};

use crate::studio::Hex;

/// Lost when the server stops. Another controller can replace the drawing
/// without the server knowing.
#[derive(Debug, Default)]
pub(crate) struct Drawings(Mutex<HashMap<String, Arc<[Hex]>>>);

impl Drawings {
    pub(crate) fn set(&self, id: &DeviceId, colors: &Arc<[Hex]>) {
        self.lock().insert(id.to_string(), Arc::clone(colors));
    }

    pub(crate) fn clear(&self, id: &DeviceId) {
        self.lock().remove(&id.to_string());
    }

    /// `#rrggbb` for each LED, or `null` with no drawing.
    pub(crate) fn get(&self, id: &DeviceId) -> Value {
        self.lock()
            .get(&id.to_string())
            .map_or(Value::Null, |colors| json!(&colors[..]))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<[Hex]>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
