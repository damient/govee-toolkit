//! `presets.json` and `/api/presets`. A preset is a look saved on one model: a
//! solid color, a white, or one color per zone or LED, with a brightness.
//! Studio shows a color or a white for every model that can play it. The server
//! keeps the file so that every client reads the same presets. Each route
//! changes one preset in the list that the server holds, so a client with an
//! old list loses no preset.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::studio::Hex;
use crate::{refuse, storage};

pub(crate) const VERSION: &str = "0.1.0";
const PRESETS: usize = 256;
/// In characters.
const NAME_MAX: usize = 64;
const SKU_MAX: usize = 32;
/// More than the LED count of a device file.
const CELLS_MAX: usize = 1024;
/// Wider than the white range of a device file.
const KELVIN: std::ops::RangeInclusive<u16> = 1000..=12000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Presets {
    version: String,
    presets: Vec<Preset>,
}

impl Default for Presets {
    fn default() -> Self {
        Self {
            version: VERSION.to_owned(),
            presets: Vec::new(),
        }
    }
}

/// One of `color`, `kelvin` and `cells`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Preset {
    /// Kept across a rename.
    #[serde(default)]
    id: String,
    name: String,
    sku: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<Hex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kelvin: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cells: Option<Vec<Hex>>,
    /// In percent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    brightness: Option<u8>,
}

impl Presets {
    pub(crate) fn checked(self) -> Result<Self, String> {
        if self.version != VERSION {
            return Err(format!(
                "the presets have the version {}, which this server does not read: update govee-server",
                self.version
            ));
        }
        if self.presets.iter().any(|preset| preset.id.is_empty()) {
            return Err("the presets are not valid: a preset has no id".to_owned());
        }
        Ok(Self {
            presets: valid(self.presets)?,
            ..Self::default()
        })
    }
}

fn valid(presets: Vec<Preset>) -> Result<Vec<Preset>, String> {
    if presets.len() > PRESETS {
        return Err(format!("there are more than {PRESETS} presets"));
    }
    let mut next = presets
        .iter()
        .filter_map(|preset| preset.id.strip_prefix("preset-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    let mut out: Vec<Preset> = Vec::with_capacity(presets.len());
    for preset in presets {
        let name = preset.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control)
        {
            return Err(format!("a preset name has 1 to {NAME_MAX} characters"));
        }
        look(&preset, &name)?;
        let sku = preset.sku;
        let plain = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
        if !(1..=SKU_MAX).contains(&sku.len()) || !sku.bytes().all(plain) {
            return Err(format!("`{sku}` is not a model"));
        }
        let mut id = preset.id.trim().to_owned();
        if id.is_empty() {
            next += 1;
            id = format!("preset-{next}");
        }
        if out.iter().any(|kept| kept.id == id) {
            return Err(format!("two presets have the id `{id}`"));
        }
        out.push(Preset {
            id,
            name,
            sku,
            color: preset.color,
            kelvin: preset.kelvin,
            cells: preset.cells,
            brightness: preset.brightness,
        });
    }
    Ok(out)
}

fn look(preset: &Preset, name: &str) -> Result<(), String> {
    match (&preset.color, preset.kelvin, &preset.cells) {
        (Some(_), None, None) => {}
        (None, Some(kelvin), None) if KELVIN.contains(&kelvin) => {}
        (None, Some(_), None) => {
            return Err(format!("`{name}` has a white outside 1000 to 12000 K"));
        }
        (None, None, Some(cells)) if (1..=CELLS_MAX).contains(&cells.len()) => {}
        (None, None, Some(_)) => {
            return Err(format!("`{name}` has more than {CELLS_MAX} cells or none"));
        }
        _ => {
            return Err(format!(
                "`{name}` must have one of `color`, `kelvin` and `cells`"
            ));
        }
    }
    if preset.brightness.is_some_and(|level| level > 100) {
        return Err(format!("`{name}` has a brightness above 100 %"));
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(crate) struct Store {
    /// `None` keeps the presets in memory alone.
    path: Option<PathBuf>,
    data: Mutex<Presets>,
}

impl Store {
    pub(crate) fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let presets = match &path {
            Some(path) => read(path)?.unwrap_or_default(),
            None => Presets::default(),
        };
        Ok(Self {
            path,
            data: Mutex::new(presets),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Presets> {
        self.data.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn get(&self) -> Presets {
        self.lock().clone()
    }

    /// `None` deletes the file. `keep` copies it to `.import.bak` first. A
    /// failed write leaves the presets as they were.
    pub(crate) fn set(&self, presets: Option<Presets>, keep: bool) -> io::Result<()> {
        let mut data = self.lock();
        if let Some(path) = &self.path {
            let text = presets.as_ref().map(storage::json_text).transpose()?;
            storage::put(path, text.as_deref(), false, keep)?;
        }
        *data = presets.unwrap_or_default();
        Ok(())
    }
}

/// The status and the message of a change that the server refuses.
type Refusal = (StatusCode, String);

impl Store {
    /// Apply `edit` to the list, check it, then write it. A failed check or
    /// write leaves the presets as they were.
    fn edit(
        &self,
        edit: impl FnOnce(&mut Vec<Preset>) -> Result<(), Refusal>,
    ) -> Result<Presets, Refusal> {
        let mut data = self.lock();
        let mut list = data.presets.clone();
        edit(&mut list)?;
        let presets = Presets {
            presets: valid(list).map_err(|e| (StatusCode::BAD_REQUEST, e))?,
            ..Presets::default()
        };
        if let Some(path) = &self.path {
            let fault = |e: io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
            let text = storage::json_text(&presets).map_err(fault)?;
            storage::put(path, Some(&text), false, false).map_err(fault)?;
        }
        *data = presets.clone();
        Ok(presets)
    }
}

fn place(list: &[Preset], id: &str) -> Result<usize, Refusal> {
    list.iter()
        .position(|preset| preset.id == id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("there is no preset `{id}`")))
}

pub(crate) fn read(path: &Path) -> io::Result<Option<Presets>> {
    let Some(text) = storage::read(path)? else {
        return Ok(None);
    };
    let invalid = |e: String| storage::invalid(format!("{}: {e}", path.display()));
    let presets: Presets = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
    presets.checked().map(Some).map_err(invalid)
}

fn answer(presets: &Presets) -> Response {
    Json(json!({ "presets": presets.presets })).into_response()
}

pub(crate) async fn get(State(store): State<Arc<Store>>) -> Response {
    answer(&store.get())
}

async fn change(
    store: Arc<Store>,
    edit: impl FnOnce(&mut Vec<Preset>) -> Result<(), Refusal> + Send + 'static,
) -> Response {
    let edited = storage::blocking(move || Ok(store.edit(edit))).await;
    match edited {
        Ok(Ok(presets)) => answer(&presets),
        Ok(Err((status, error))) => refuse(status, &error),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// `POST /api/presets`: one preset, which the server adds to the end of its
/// list.
pub(crate) async fn add(
    State(store): State<Arc<Store>>,
    Json(mut preset): Json<Preset>,
) -> Response {
    preset.id = String::new();
    change(store, move |list| {
        list.push(preset);
        Ok(())
    })
    .await
}

/// `PUT /api/presets/{id}`: the preset replaces the one with this `id`, in its
/// place.
pub(crate) async fn replace(
    State(store): State<Arc<Store>>,
    AxumPath(id): AxumPath<String>,
    Json(mut preset): Json<Preset>,
) -> Response {
    change(store, move |list| {
        let at = place(list, &id)?;
        preset.id = id;
        if let Some(slot) = list.get_mut(at) {
            *slot = preset;
        }
        Ok(())
    })
    .await
}

/// `DELETE /api/presets/{id}`.
pub(crate) async fn remove(
    State(store): State<Arc<Store>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    change(store, move |list| {
        let at = place(list, &id)?;
        list.remove(at);
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests;
