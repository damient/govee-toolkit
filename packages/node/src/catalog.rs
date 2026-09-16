//! What `devices/*.yaml` declares, as the SDK reads it.

use govee_toolkit::Catalog as CoreCatalog;
use napi::Env;
use napi_derive::napi;

use crate::conv::to_js;
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
