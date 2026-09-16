//! The configuration, and what is wrong with it.

use govee_toolkit::Config as CoreConfig;
use napi::Env;
use napi_derive::napi;

use crate::conv::to_js;
use crate::errors::map;

/// The configuration in force. Every field is read-only: the SDK reads the
/// configuration once, at startup.
#[napi]
#[derive(Clone)]
pub struct Config {
    pub(crate) inner: CoreConfig,
}

#[napi]
impl Config {
    /// The default configuration: `lan` alone, and no device entry.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreConfig::default(),
        }
    }

    /// Read `$XDG_CONFIG_HOME/govee-toolkit/config.yaml`.
    ///
    /// A missing file is the default configuration, not an error. A file that
    /// does not parse throws with the code `config`.
    #[napi(factory)]
    pub fn load(env: &Env) -> napi::Result<Self> {
        Ok(Self {
            inner: map(env, CoreConfig::load())?,
        })
    }

    /// Read the configuration from one path.
    #[napi(factory)]
    pub fn load_from(env: &Env, path: String) -> napi::Result<Self> {
        Ok(Self {
            inner: map(env, CoreConfig::load_from(path))?,
        })
    }

    /// The modes enabled for a device with no entry of its own.
    #[napi(getter)]
    pub fn default_modes(&self) -> Vec<String> {
        self.inner
            .defaults
            .modes
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// The identity of every device the file names.
    #[napi(getter)]
    pub fn devices(&self) -> Vec<String> {
        self.inner.devices.keys().map(ToString::to_string).collect()
    }

    /// The rate a stream sends at when the device file measured none.
    #[napi(getter)]
    pub fn stream_fallback_hz(&self) -> f64 {
        self.inner.stream.fallback_hz
    }

    /// The whole configuration, as the core serializes it. It carries no
    /// credential: a key comes from the environment, never from the file.
    #[napi(js_name = "toJSON")]
    pub fn to_json(&self, env: &Env) -> napi::Result<serde_json::Value> {
        to_js(env, &self.inner)
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!(
            "Config(defaultModes={:?}, devices={})",
            self.default_modes(),
            self.inner.devices.len()
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
