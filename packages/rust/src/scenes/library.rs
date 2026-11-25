//! The library answer: categories, scenes, and the effects of each scene.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use super::Error;

/// One scene that a model plays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scene {
    /// The library's name. A scene with more than one effect gives each effect
    /// its own entry, named with the effect's letter: `Aurora A`, `Aurora B`.
    pub name: String,
    /// The library's category.
    pub category: String,
    /// The code that the device reports back for the scene.
    pub code: u16,
    /// The library's type for the parameters. A device file entry declares
    /// the types that it plays.
    pub scene_type: i64,
    /// The parameters, as the library gives them. Empty for a scene that the
    /// firmware stores.
    #[serde(with = "hex")]
    pub param: Vec<u8>,
}

mod hex {
    use std::fmt::Write;

    pub(super) fn serialize<S: serde::Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let text = bytes.iter().fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        });
        s.serialize_str(&text)
    }
}

#[derive(Deserialize)]
struct Answer {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    categories: Vec<Category>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Category {
    category_name: String,
    #[serde(default)]
    scenes: Vec<RawScene>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawScene {
    scene_name: String,
    #[serde(default)]
    light_effects: Vec<Effect>,
}

// `scence` is the library's own spelling.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Effect {
    #[serde(default)]
    scence_name: String,
    scene_code: u16,
    scene_type: i64,
    #[serde(default)]
    scence_param: String,
    #[serde(default, rename = "specialEffect")]
    specials: Vec<Special>,
}

/// Parameters that replace the shared ones on the models that `supportSku`
/// names.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Special {
    #[serde(default)]
    scence_param: String,
    #[serde(default)]
    support_sku: Vec<String>,
}

/// Read a library answer for `sku`. Where an effect carries parameters for
/// `sku` of its own, those replace the shared ones.
///
/// # Errors
///
/// [`Error::Unreadable`] if the text is not a library answer, or if a
/// parameter is not base64. `path` only names the source in that error.
pub fn parse(sku: &str, text: &str, path: &std::path::Path) -> Result<Vec<Scene>, Error> {
    let unreadable = |reason: String| Error::Unreadable {
        path: path.to_owned(),
        reason,
    };
    let answer: Answer = serde_json::from_str(text).map_err(|e| unreadable(e.to_string()))?;
    let mut scenes = Vec::new();
    for category in answer.data.categories {
        for raw in category.scenes {
            let variants = raw.light_effects.len();
            for (i, effect) in raw.light_effects.into_iter().enumerate() {
                let own = effect
                    .specials
                    .iter()
                    .find(|s| s.support_sku.iter().any(|k| k == sku));
                let encoded = own.map_or(&effect.scence_param, |s| &s.scence_param);
                let param = STANDARD
                    .decode(encoded)
                    .map_err(|e| unreadable(format!("`{}`: {e}", raw.scene_name)))?;
                scenes.push(Scene {
                    name: variant_name(&raw.scene_name, &effect.scence_name, i, variants),
                    category: category.category_name.clone(),
                    code: effect.scene_code,
                    scene_type: effect.scene_type,
                    param,
                });
            }
        }
    }
    Ok(scenes)
}

fn variant_name(scene: &str, letter: &str, index: usize, variants: usize) -> String {
    if variants < 2 {
        return scene.to_owned();
    }
    if letter.is_empty() {
        return format!("{scene} {}", index + 1);
    }
    format!("{scene} {letter}")
}

/// The scene that `query` names: a code, a full name, or the name that the
/// variants of one scene share. Names compare without case.
///
/// # Errors
///
/// [`Error::Unknown`] if nothing matches, and [`Error::Ambiguous`] if more
/// than one scene does.
pub fn find<'a>(sku: &str, scenes: &'a [Scene], query: &str) -> Result<&'a Scene, Error> {
    let query = query.trim();
    let matches: Vec<&Scene> = if let Ok(code) = query.parse::<u16>() {
        scenes.iter().filter(|s| s.code == code).collect()
    } else {
        let exact: Vec<&Scene> = scenes
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case(query))
            .collect();
        if exact.is_empty() {
            scenes
                .iter()
                .filter(|s| shares_name(&s.name, query))
                .collect()
        } else {
            exact
        }
    };
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(Error::Unknown {
            sku: sku.to_owned(),
            query: query.to_owned(),
        }),
        many => Err(Error::Ambiguous {
            sku: sku.to_owned(),
            query: query.to_owned(),
            matches: many
                .iter()
                .map(|s| format!("{} ({}, {})", s.name, s.category, s.code))
                .collect(),
        }),
    }
}

/// `Aurora B` shares the name `aurora`.
fn shares_name(name: &str, query: &str) -> bool {
    name.rsplit_once(' ')
        .is_some_and(|(base, _)| base.eq_ignore_ascii_case(query))
}

#[cfg(test)]
mod tests;
