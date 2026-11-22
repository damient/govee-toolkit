//! `rooms.json` and `/api/rooms`: the rooms of the lights.
//!
//! A room can be empty, and a device can be in more than one room. The file
//! belongs to the server, not to the preferences of Studio, so that every
//! client reads the same rooms.

use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::storage;

/// The version of the shape of `rooms.json` that this server writes.
pub(crate) const VERSION: &str = "0.1.0";
/// The longest name, in characters.
const NAME_MAX: usize = 64;

/// The content of `rooms.json`, and the rooms part of a backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rooms {
    version: String,
    rooms: Vec<Room>,
}

impl Default for Rooms {
    fn default() -> Self {
        Self {
            version: VERSION.to_owned(),
            rooms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Room {
    /// Kept across a rename. A request that gives none gets a new one.
    #[serde(default)]
    id: String,
    name: String,
    /// Device IDs, in the order of the room.
    #[serde(default)]
    devices: Vec<String>,
}

impl Rooms {
    /// Check the content of a file or of a backup.
    pub(crate) fn checked(self) -> Result<Self, String> {
        if self.version != VERSION {
            return Err(format!(
                "the rooms have the version {}, which this server does not read: update govee-server",
                self.version
            ));
        }
        if self.rooms.iter().any(|room| room.id.is_empty()) {
            return Err("the rooms are not valid: a room has no id".to_owned());
        }
        Ok(Self {
            rooms: valid(self.rooms)?,
            ..Self::default()
        })
    }

    /// Remove `ids` from every room. Answer `true` when a room changed.
    fn forget(&mut self, ids: &[String]) -> bool {
        let mut changed = false;
        for room in &mut self.rooms {
            let before = room.devices.len();
            room.devices
                .retain(|id| !ids.iter().any(|gone| gone.eq_ignore_ascii_case(id)));
            changed |= room.devices.len() != before;
        }
        changed
    }
}

/// Trim the names and the IDs, then refuse a blank or long name, a name or
/// an ID that two rooms share, and a device that a room lists twice. A room
/// with no id gets `room-<n>`, above every number in use.
fn valid(rooms: Vec<Room>) -> Result<Vec<Room>, String> {
    let mut next = rooms
        .iter()
        .filter_map(|room| room.id.strip_prefix("room-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    let mut out: Vec<Room> = Vec::with_capacity(rooms.len());
    for room in rooms {
        let name = room.name.trim().to_owned();
        if name.is_empty() {
            return Err("a room has no name".to_owned());
        }
        if name.chars().count() > NAME_MAX {
            return Err(format!("a room name is longer than {NAME_MAX} characters"));
        }
        if out.iter().any(|kept| kept.name.eq_ignore_ascii_case(&name)) {
            return Err(format!("two rooms have the name `{name}`"));
        }
        let mut id = room.id.trim().to_owned();
        if id.is_empty() {
            next += 1;
            id = format!("room-{next}");
        }
        if out.iter().any(|kept| kept.id == id) {
            return Err(format!("two rooms have the id `{id}`"));
        }
        let mut devices: Vec<String> = Vec::with_capacity(room.devices.len());
        for device in room.devices {
            let device = device.trim().to_owned();
            if device.is_empty() {
                return Err(format!("`{name}` lists a blank device ID"));
            }
            if devices
                .iter()
                .any(|kept| kept.eq_ignore_ascii_case(&device))
            {
                return Err(format!("`{name}` lists {device} twice"));
            }
            devices.push(device);
        }
        out.push(Room { id, name, devices });
    }
    Ok(out)
}

/// `rooms.json` in memory, and the file that keeps it.
#[derive(Debug, Default)]
pub(crate) struct Store {
    /// `None` keeps the rooms in memory alone.
    path: Option<PathBuf>,
    data: Mutex<Rooms>,
}

impl Store {
    /// Read the file at `path`. A file that does not exist holds no room.
    pub(crate) fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let rooms = match &path {
            Some(path) => read(path)?.unwrap_or_default(),
            None => Rooms::default(),
        };
        Ok(Self {
            path,
            data: Mutex::new(rooms),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Rooms> {
        self.data.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn get(&self) -> Rooms {
        self.lock().clone()
    }

    /// Replace the rooms and the file. `None` deletes the file. `keep` makes
    /// a `.import.bak` copy of the file first. The rooms stay as they were
    /// when the write fails.
    pub(crate) fn set(&self, rooms: Option<Rooms>, keep: bool) -> io::Result<()> {
        let mut data = self.lock();
        if let Some(path) = &self.path {
            if keep {
                storage::keep(path)?;
            }
            match &rooms {
                Some(rooms) => {
                    let mut text = serde_json::to_string_pretty(rooms).map_err(storage::invalid)?;
                    text.push('\n');
                    storage::write(path, text.as_bytes(), false)?;
                }
                None => storage::remove(path)?,
            }
        }
        *data = rooms.unwrap_or_default();
        Ok(())
    }

    /// Remove `ids` from every room, and write the file when a room changed.
    pub(crate) fn forget(&self, ids: &[String]) -> io::Result<()> {
        let mut next = self.get();
        if next.forget(ids) {
            self.set(Some(next), false)?;
        }
        Ok(())
    }
}

/// The checked rooms of the file at `path`, or `None` when it does not exist.
pub(crate) fn read(path: &std::path::Path) -> io::Result<Option<Rooms>> {
    let Some(text) = storage::read(path)? else {
        return Ok(None);
    };
    let invalid = |e: String| storage::invalid(format!("{}: {e}", path.display()));
    let rooms: Rooms = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
    rooms.checked().map(Some).map_err(invalid)
}

fn answer(rooms: &Rooms) -> Response {
    Json(json!({ "rooms": rooms.rooms })).into_response()
}

/// `GET /api/rooms`: `{ "rooms": [{ "id", "name", "devices" }] }`.
pub(crate) async fn get(State(store): State<Arc<Store>>) -> Response {
    answer(&store.get())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Put {
    rooms: Vec<Room>,
}

/// `PUT /api/rooms` with every room: the same answer as `GET`. A room with
/// no `id` gets one.
pub(crate) async fn put(State(store): State<Arc<Store>>, Json(body): Json<Put>) -> Response {
    let rooms = match valid(body.rooms) {
        Ok(rooms) => Rooms {
            rooms,
            ..Rooms::default()
        },
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    };
    match store.set(Some(rooms.clone()), false) {
        Ok(()) => answer(&rooms),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn room(id: &str, name: &str, devices: &[&str]) -> Room {
        Room {
            id: id.to_owned(),
            name: name.to_owned(),
            devices: devices.iter().map(|d| (*d).to_owned()).collect(),
        }
    }

    #[test]
    fn a_new_room_gets_a_number_above_the_others() {
        let rooms = valid(vec![
            room("room-4", "Kitchen", &[]),
            room("", " Desk ", &["AA"]),
        ])
        .unwrap();
        assert_eq!(rooms[1].id, "room-5");
        assert_eq!(rooms[1].name, "Desk");
    }

    #[test]
    fn a_device_can_be_in_two_rooms_but_not_twice_in_one() {
        assert!(valid(vec![room("", "A", &["AA"]), room("", "B", &["AA"])]).is_ok());
        assert!(valid(vec![room("", "A", &["AA", "aa"])]).is_err());
    }

    #[test]
    fn a_blank_long_or_shared_name_is_refused() {
        assert!(valid(vec![room("", " ", &[])]).is_err());
        assert!(valid(vec![room("", &"x".repeat(65), &[])]).is_err());
        assert!(valid(vec![room("", "Desk", &[]), room("", "desk", &[])]).is_err());
        assert!(valid(vec![room("r", "A", &[]), room("r", "B", &[])]).is_err());
    }

    #[test]
    fn forget_removes_a_device_from_every_room() {
        let mut rooms = Rooms {
            rooms: vec![
                room("room-1", "A", &["AA", "BB"]),
                room("room-2", "B", &["aa"]),
            ],
            ..Rooms::default()
        };
        assert!(rooms.forget(&["AA".to_owned()]));
        assert_eq!(rooms.rooms[0].devices, ["BB"]);
        assert_eq!(rooms.rooms[1].devices, Vec::<String>::new());
        assert!(!rooms.forget(&["CC".to_owned()]));
    }

    #[test]
    fn another_version_is_refused() {
        let rooms = Rooms {
            version: "9.0.0".to_owned(),
            rooms: Vec::new(),
        };
        assert!(rooms.checked().is_err());
    }
}
