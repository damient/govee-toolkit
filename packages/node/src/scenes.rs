//! The scene library of a model, as the cache holds it.

// A doc comment here is the JSDoc, which names the error codes in prose.
#![allow(clippy::missing_errors_doc)]

use govee_toolkit::input::{self, Value};
use govee_toolkit::{Scene as CoreScene, scenes};
use napi::Env;
use napi::bindgen_prelude::{PromiseRaw, Uint8Array, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::errors::{map, value_error};
use crate::promise::promise;

/// One scene that a model plays.
#[napi(object)]
pub struct Scene {
    /// The library's name. Each effect of one scene has its own entry:
    /// `Aurora A`, `Aurora B`.
    pub name: String,
    /// The library's category.
    pub category: String,
    /// The code that the device reports back for the scene.
    pub code: u32,
    /// The library's type for the parameters.
    #[allow(clippy::struct_field_names)] // `sceneType`, as the core names it.
    pub scene_type: i64,
    /// Empty for a scene that the firmware stores.
    pub param: Uint8Array,
}

impl From<CoreScene> for Scene {
    fn from(scene: CoreScene) -> Self {
        Self {
            name: scene.name,
            category: scene.category,
            code: u32::from(scene.code),
            scene_type: scene.scene_type,
            param: Uint8Array::new(scene.param),
        }
    }
}

/// Fetch the scene library of `sku` from the cloud, and store it in the
/// cache. The request carries no account and no key.
///
/// Rejects with `scene_fetch_failed`, `scene_list_unreadable` or `bad_sku`.
#[napi]
pub fn fetch_scenes(env: &Env, sku: String) -> napi::Result<PromiseRaw<'_, Vec<Scene>>> {
    promise(env, async move {
        Ok(scenes::fetch(&sku)
            .await?
            .into_iter()
            .map(Scene::from)
            .collect())
    })
}

/// The scene library of `sku` that `fetchScenes()` stored. Reads the cache
/// alone.
///
/// Throws `scene_list_missing`, `scene_list_unreadable` or `bad_sku`.
#[napi]
pub fn load_scenes(env: &Env, sku: String) -> napi::Result<Vec<Scene>> {
    let library = map(env, scenes::load(&sku).map_err(Into::into))?;
    Ok(library.into_iter().map(Scene::from).collect())
}

/// What `DeviceHandle.scene()` plays.
pub(crate) enum Query {
    Named(String),
    Scene(CoreScene),
}

/// A number reads as the code of the scene.
pub(crate) fn named(env: &Env, value: &Unknown<'_>) -> napi::Result<String> {
    conv::read(env, input::scene(&conv::value(env, value)?))
}

pub(crate) fn query(env: &Env, value: &Unknown<'_>) -> napi::Result<Query> {
    let value = conv::value(env, value)?;
    if !matches!(value, Value::Map(_)) {
        return input::scene(&value)
            .map(Query::Named)
            .map_err(|_| value_error(env, "a scene is a name, a code or a Scene"));
    }
    conv::read(
        env,
        value.known_keys("scene", &["name", "category", "code", "sceneType", "param"]),
    )?;
    let text = |key: &str| match value.get(key) {
        Some(Value::Text(text)) => Ok(text.clone()),
        _ => Err(value_error(
            env,
            format!("a Scene carries `{key}` as a string"),
        )),
    };
    let whole = |key: &str| match value.get(key) {
        Some(Value::Int(number)) => Ok(*number),
        _ => Err(value_error(
            env,
            format!("a Scene carries `{key}` as a whole number"),
        )),
    };
    let code = whole("code")?;
    let code = u16::try_from(code)
        .map_err(|_| value_error(env, format!("{code} is outside a scene code's 0-65535")))?;
    let param = match value.get("param") {
        Some(Value::Bytes(bytes)) => bytes.clone(),
        _ => return Err(value_error(env, "a Scene carries `param` as a Uint8Array")),
    };
    Ok(Query::Scene(CoreScene {
        name: text("name")?,
        category: text("category")?,
        code,
        scene_type: whole("sceneType")?,
        param,
    }))
}
