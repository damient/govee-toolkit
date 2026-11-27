//! The last drawing that the server sent to each light, so that a client that
//! opens later can show it. The light keeps playing a drawing after the
//! request, and answers no read of it.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use govee_toolkit::DeviceId;
use serde_json::{Value, json};

/// Lost when the server stops. Another controller can replace the drawing
/// without the server knowing.
#[derive(Debug, Default)]
pub(crate) struct Drawings(Mutex<HashMap<String, Vec<[u8; 3]>>>);

impl Drawings {
    pub(crate) fn set(&self, id: &DeviceId, colors: &[[u8; 3]]) {
        self.lock().insert(id.to_string(), colors.to_vec());
    }

    pub(crate) fn clear(&self, id: &DeviceId) {
        self.lock().remove(&id.to_string());
    }

    /// `#rrggbb` for each LED, or `null` with no drawing.
    pub(crate) fn get(&self, id: &DeviceId) -> Value {
        self.lock()
            .get(&id.to_string())
            .map_or(Value::Null, |colors| {
                let hex: Vec<String> = colors
                    .iter()
                    .map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"))
                    .collect();
                json!(hex)
            })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<[u8; 3]>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
