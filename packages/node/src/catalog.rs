//! What `devices/*.yaml` declares, as the SDK reads it.

use govee_toolkit::{Catalog as CoreCatalog, describe_sku, input, profile, query};
use napi::Env;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::conv::{self, to_js};
use crate::errors::map;

/// Every device the build knows. Reads no hardware.
#[napi]
#[derive(Clone)]
pub struct Catalog {
    pub(crate) inner: CoreCatalog,
}

#[napi]
impl Catalog {
    /// The catalog compiled into this build.
    #[napi(factory)]
    pub fn embedded(env: &Env) -> napi::Result<Self> {
        Ok(Self {
            inner: map(env, CoreCatalog::embedded().map_err(Into::into))?,
        })
    }

    /// Every SKU that resolves, verified aliases included.
    #[napi]
    pub fn skus(&self) -> Vec<String> {
        self.inner.skus().map(ToOwned::to_owned).collect()
    }

    /// Whether a SKU resolves.
    #[napi]
    pub fn has(&self, sku: String) -> bool {
        self.inner.device(&sku).is_ok()
    }

    /// One device file, with every `include:` and every override applied.
    ///
    /// Throws with the code `unknown_sku` when nothing declares it.
    #[napi]
    pub fn device(&self, env: &Env, sku: String) -> napi::Result<serde_json::Value> {
        let device = map(env, self.inner.device(&sku).map_err(Into::into))?;
        to_js(env, device)
    }

    /// The record that `govee describe --json` prints for a SKU. Throws
    /// `unknown_sku` when nothing declares it.
    #[napi]
    pub fn describe(&self, env: &Env, sku: String) -> napi::Result<serde_json::Value> {
        map(env, describe_sku(&self.inner, &sku).map_err(Into::into))
    }

    /// The DMX channel tables of a SKU, as `dist/catalog.json` carries them.
    /// Throws `unknown_sku` when nothing declares it.
    #[napi]
    pub fn dmx(&self, env: &Env, sku: String) -> napi::Result<serde_json::Value> {
        let device = map(env, self.inner.device(&sku).map_err(Into::into))?;
        Ok(profile::report::entry(device))
    }

    /// Every capability name that a device of the catalog declares, sorted.
    #[napi]
    pub fn capabilities(&self) -> Vec<String> {
        self.inner
            .capabilities()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Every SKU, verified aliases included, that matches every filter given.
    /// `capability` reads the capabilities of `mode` where it is given, and
    /// those of the hardware otherwise. `support` needs `mode`, and an
    /// unknown key is refused.
    #[napi(
        ts_args_type = "filter?: { mode?: string | undefined; capability?: string | undefined; support?: string | undefined }"
    )]
    pub fn query(&self, env: &Env, filter: Option<Object<'_>>) -> napi::Result<Vec<String>> {
        let filter = conv::options(env, filter.as_ref())?;
        let [mode, capability, support] = conv::read(
            env,
            input::names(&filter, "query", ["mode", "capability", "support"]),
        )?;
        let mode = mode.map(|name| conv::mode(env, name)).transpose()?;
        conv::read(env, query(&self.inner, mode, capability, support))
    }

    /// How many device files the build carries.
    #[napi(getter)]
    pub fn size(&self) -> u32 {
        u32::try_from(self.inner.devices().count()).unwrap_or(u32::MAX)
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!("Catalog(devices={})", self.size())
    }
}
