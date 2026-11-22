//! `studio.json`: the settings of the server and the settings of Studio.
//!
//! Each part carries the version that gave it its shape:
//!
//! - `server.version`: the `govee-server` that wrote the file.
//! - `studio.version`: the Studio release that gave the Studio settings their
//!   shape. A change to that shape raises [`VERSION`] and adds a row to
//!   [`crate::compat::TABLE`].

use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{compat, storage};

/// The version of the Studio settings that this server writes.
pub(crate) const VERSION: &str = "0.1.0";

/// The content of `studio.json`, and the settings part of a backup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Studio {
    pub(crate) server: ServerPart,
    pub(crate) studio: StudioPart,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ServerPart {
    pub(crate) version: String,
    pub(crate) settings: ServerSettings,
}

impl Default for ServerPart {
    fn default() -> Self {
        Self {
            version: compat::SERVER.to_owned(),
            settings: ServerSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct StudioPart {
    pub(crate) version: String,
    pub(crate) settings: StudioSettings,
}

impl Default for StudioPart {
    fn default() -> Self {
        Self {
            version: VERSION.to_owned(),
            settings: StudioSettings::default(),
        }
    }
}

impl Studio {
    /// Read `value` and give it the versions that this server writes.
    ///
    /// Refuse a Studio version that [`crate::compat::TABLE`] does not hold:
    /// its keys can mean other things. A file from a newer server that does
    /// not parse gets an answer that says to update.
    pub(crate) fn from_value(value: Value) -> Result<Self, String> {
        let server = value
            .pointer("/server/version")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if let Some(version) = value.pointer("/studio/version").and_then(Value::as_str)
            && !compat::reads(version)
        {
            return Err(compat::refusal(version, server.as_deref()));
        }
        let mut studio: Self = serde_json::from_value(value).map_err(|e| {
            match server
                .as_deref()
                .filter(|server| compat::newer_server(server))
            {
                Some(server) => compat::refusal(VERSION, Some(server)),
                None => format!("the settings are not valid: {e}"),
            }
        })?;
        compat::SERVER.clone_into(&mut studio.server.version);
        VERSION.clone_into(&mut studio.studio.version);
        Ok(studio)
    }
}

/// What only a client on this machine can change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ServerSettings {
    /// The server opens its network listener at start.
    pub(crate) share: bool,
}

/// What every client that the server admits can change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct StudioSettings {
    pub(crate) theme: Theme,
    /// `None` until the user picks a language.
    pub(crate) language: Option<Language>,
    pub(crate) advanced: bool,
    /// The favorite colors, which every light of the panel offers.
    pub(crate) favorites: Favorites,
}

/// The most favorite colors that the settings keep.
const FAVORITES: usize = 32;

/// A list of at most [`FAVORITES`] colors, with no color twice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Hex>", into = "Vec<Hex>")]
pub(crate) struct Favorites(Vec<Hex>);

impl TryFrom<Vec<Hex>> for Favorites {
    type Error = String;

    fn try_from(list: Vec<Hex>) -> Result<Self, String> {
        if list.len() > FAVORITES {
            return Err(format!("there are more than {FAVORITES} favorite colors"));
        }
        if list
            .iter()
            .enumerate()
            .any(|(i, hex)| list.iter().take(i).any(|other| other == hex))
        {
            return Err("a favorite color is in the list twice".to_owned());
        }
        Ok(Self(list))
    }
}

impl From<Favorites> for Vec<Hex> {
    fn from(favorites: Favorites) -> Self {
        favorites.0
    }
}

/// A color as `#rrggbb`. The text is kept in lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Hex([u8; 3]);

impl Hex {
    pub(crate) fn rgb(&self) -> [u8; 3] {
        self.0
    }
}

impl TryFrom<String> for Hex {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let refused = || format!("`{text}` is not a color as #rrggbb");
        let digits = text
            .strip_prefix('#')
            .filter(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(refused)?;
        let mut rgb = [0; 3];
        for (i, channel) in rgb.iter_mut().enumerate() {
            let pair = digits.get(i * 2..i * 2 + 2).ok_or_else(refused)?;
            *channel = u8::from_str_radix(pair, 16).map_err(|_| refused())?;
        }
        Ok(Self(rgb))
    }
}

impl From<Hex> for String {
    fn from(hex: Hex) -> Self {
        let [r, g, b] = hex.0;
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Theme {
    #[default]
    Dark,
    Light,
    Auto,
}

/// A language code such as `en` or `pt-BR`. Studio decides which codes it
/// has a text for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Language(String);

impl TryFrom<String> for Language {
    type Error = String;

    fn try_from(code: String) -> Result<Self, String> {
        let valid = (2..=16).contains(&code.len())
            && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if valid {
            Ok(Self(code))
        } else {
            Err(format!("`{code}` is not a language code"))
        }
    }
}

impl From<Language> for String {
    fn from(language: Language) -> Self {
        language.0
    }
}

/// `studio.json` in memory, and the file that keeps it.
#[derive(Debug, Default)]
pub(crate) struct Store {
    /// `None` keeps the content in memory alone.
    path: Option<PathBuf>,
    data: Mutex<Studio>,
}

impl Store {
    /// Read the file at `path`. A file that does not exist is the default.
    pub(crate) fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let mut studio = Studio::default();
        if let Some(path) = &path
            && let Some(text) = storage::read(path)?
        {
            let value = serde_json::from_str(&text)
                .map_err(|e| storage::invalid(format!("{}: {e}", path.display())))?;
            studio = Studio::from_value(value)
                .map_err(|e| storage::invalid(format!("{}: {e}", path.display())))?;
        }
        Ok(Self {
            path,
            data: Mutex::new(studio),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Studio> {
        self.data.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn get(&self) -> Studio {
        self.lock().clone()
    }

    /// Apply `change`, write the file, and answer the new content. The
    /// content stays as it was when the write fails.
    pub(crate) fn update(&self, change: impl FnOnce(&mut Studio)) -> io::Result<Studio> {
        let mut data = self.lock();
        let mut next = data.clone();
        change(&mut next);
        if next != *data {
            self.save(&next, false)?;
            *data = next;
        }
        Ok(data.clone())
    }

    /// Replace the content and the file. `keep` makes a `.import.bak` copy of
    /// the file first.
    pub(crate) fn replace(&self, studio: Studio, keep: bool) -> io::Result<()> {
        let mut data = self.lock();
        self.save(&studio, keep)?;
        *data = studio;
        Ok(())
    }

    fn save(&self, studio: &Studio, keep: bool) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if keep {
            storage::keep(path)?;
        }
        let mut text = serde_json::to_string_pretty(studio).map_err(storage::invalid)?;
        text.push('\n');
        storage::write(path, text.as_bytes(), false)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::*;

    fn path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("govee-studio-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("studio.json")
    }

    #[test]
    fn a_missing_file_is_the_default() {
        let store = Store::open(Some(path("missing"))).unwrap();
        assert_eq!(store.get(), Studio::default());
        assert_eq!(store.get().studio.settings.theme, Theme::Dark);
        assert!(!store.get().server.settings.share);
    }

    #[test]
    fn an_update_reaches_the_file_and_the_next_open() {
        let path = path("update");
        let store = Store::open(Some(path.clone())).unwrap();
        store
            .update(|s| {
                s.server.settings.share = true;
                s.studio.settings.theme = Theme::Light;
            })
            .unwrap();
        let again = Store::open(Some(path.clone())).unwrap().get();
        assert!(again.server.settings.share);
        assert_eq!(again.studio.settings.theme, Theme::Light);
        let file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(file.pointer("/studio/version"), Some(&json!(VERSION)));
        assert_eq!(
            file.pointer("/server/version"),
            Some(&json!(compat::SERVER))
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn another_version_and_an_unknown_key_are_refused() {
        let studio = |part: Value| json!({ "server": { "version": "0.1.0" }, "studio": part });
        assert!(Studio::from_value(studio(json!({ "version": "0.2.0" }))).is_err());
        assert!(Studio::from_value(studio(json!({ "version": "0.1.0", "rooms": [] }))).is_err());
        assert!(Studio::from_value(studio(json!({ "version": "0.1.0" }))).is_ok());
        assert!(Studio::from_value(json!({ "version": 1 })).is_err());
    }

    #[test]
    fn an_unknown_key_from_a_newer_server_says_to_update() {
        let value = json!({ "server": { "version": "99.0.0", "settings": { "dmx": true } } });
        let error = Studio::from_value(value).unwrap_err();
        assert!(error.contains("Update govee-server"), "{error}");
    }

    #[test]
    fn a_file_that_does_not_parse_stops_the_open() {
        let path = path("broken");
        storage::write(&path, b"{", false).unwrap();
        let error = Store::open(Some(path.clone())).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("studio.json"), "{error}");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_favorite_is_a_color_and_the_list_is_short() {
        let favorites = |list: Value| serde_json::from_value::<Favorites>(list);
        assert!(favorites(json!(["#FF8000", "#00ff00"])).is_ok());
        assert!(favorites(json!(["#ff8000", "#FF8000"])).is_err());
        assert!(favorites(json!(["ff8000"])).is_err());
        assert!(favorites(json!(["#ff80zz"])).is_err());
        assert!(favorites(json!(["#+f8000"])).is_err());
        let many: Vec<String> = (0..33).map(|i| format!("#0000{i:02x}")).collect();
        assert!(favorites(json!(many.iter().take(32).collect::<Vec<_>>())).is_ok());
        assert!(favorites(json!(many)).is_err());
        let hex: String = serde_json::from_value::<Hex>(json!("#FF8000"))
            .unwrap()
            .into();
        assert_eq!(hex, "#ff8000");
    }

    #[test]
    fn a_language_code_is_short_and_plain() {
        assert!(Language::try_from("pt-BR".to_owned()).is_ok());
        assert!(Language::try_from("e".to_owned()).is_err());
        assert!(Language::try_from("../x".to_owned()).is_err());
    }
}
