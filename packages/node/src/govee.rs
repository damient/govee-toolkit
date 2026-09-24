//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{DeviceId, Govee as CoreGovee};
use napi::Env;
use napi::bindgen_prelude::PromiseRaw;
use napi_derive::napi;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::map;
use crate::events::EventStream;
use crate::promise::promise;
use crate::types::Device;

/// The SDK. Start one and keep it: it holds the catalog, the configuration
/// and one transport per mode.
#[napi]
pub struct Govee {
    pub(crate) inner: CoreGovee,
}

#[napi]
impl Govee {
    /// Start the SDK. Without a configuration, it reads the file. Without a
    /// catalog, it reads the one the package carries.
    #[napi]
    pub fn start<'env>(
        env: &'env Env,
        config: Option<&Config>,
        catalog: Option<&Catalog>,
    ) -> napi::Result<PromiseRaw<'env, Govee>> {
        let config = match config {
            Some(config) => config.inner.clone(),
            None => map(env, govee_toolkit::Config::load())?,
        };
        let catalog = match catalog {
            Some(catalog) => catalog.inner.clone(),
            None => map(env, govee_toolkit::Catalog::embedded().map_err(Into::into))?,
        };
        promise(env, async move {
            Ok(Govee {
                inner: CoreGovee::start_with(config, catalog).await?,
            })
        })
    }

    /// Run a discovery scan on every mode and return what answered.
    ///
    /// The scans run at the same time, so the call takes the longest window
    /// and not their sum. Nothing on the send path calls this.
    #[napi]
    pub fn scan<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Vec<Device>>> {
        let govee = self.inner.clone();
        promise(env, async move {
            Ok(govee.scan().await?.into_iter().map(Device::from).collect())
        })
    }

    /// Run a discovery scan on the modes named.
    ///
    /// A mode this build carries no transport for contributes nothing and is
    /// not an error.
    #[napi]
    pub fn scan_on<'env>(
        &self,
        env: &'env Env,
        modes: Vec<String>,
    ) -> napi::Result<PromiseRaw<'env, Vec<Device>>> {
        let wanted = conv::modes(env, modes)?;
        let govee = self.inner.clone();
        promise(env, async move {
            let found = govee.scan_on(&wanted).await?;
            Ok(found.into_iter().map(Device::from).collect())
        })
    }

    /// Every device known, across every mode. One reachable over two modes
    /// appears once.
    #[napi]
    pub fn devices(&self) -> Vec<Device> {
        self.inner.devices().into_iter().map(Device::from).collect()
    }

    /// The devices the targets name, in the order they were written.
    ///
    /// A target is an identity (`1C:8B:…`), a SKU (`H6159`), or a name the
    /// configuration gives a device (`name:kitchen`). `id:`, `sku:` and
    /// `name:` state the kind where the target alone does not. A SKU and a
    /// name select among the devices the SDK knows, so scan first.
    ///
    /// `mode` is the one mode the caller will drive. A SKU and a name then
    /// match among the devices that enable it. An identity selects itself
    /// either way.
    #[napi]
    pub fn select(
        &self,
        env: &Env,
        targets: Vec<String>,
        mode: Option<String>,
    ) -> napi::Result<Vec<String>> {
        let only = mode.map(|name| conv::mode(env, &name)).transpose()?;
        let chosen = map(env, self.inner.select(targets, only).map_err(Into::into))?;
        Ok(chosen.iter().map(ToString::to_string).collect())
    }

    /// The modes this build carries a transport for. Not a preference order:
    /// that is each device's own configuration.
    #[napi]
    pub fn modes(&self) -> Vec<String> {
        self.inner
            .modes()
            .into_iter()
            .map(|m| m.to_string())
            .collect()
    }

    /// Everything wrong with the configuration, as one sentence each.
    #[napi]
    pub fn problems(&self) -> Vec<String> {
        self.inner
            .problems()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// The configuration in force.
    #[napi(getter)]
    pub fn config(&self) -> Config {
        Config {
            inner: self.inner.config().clone(),
        }
    }

    /// The device catalog in force.
    #[napi(getter)]
    pub fn catalog(&self) -> Catalog {
        Catalog {
            inner: self.inner.catalog().clone(),
        }
    }

    /// A handle for one device, by the identity it reports or by the name
    /// the configuration gives.
    ///
    /// `id:…` and `name:…` state the kind. A bare target is a name where the
    /// configuration gives one, and an identity otherwise. It reads the
    /// configuration and no scan.
    #[napi]
    pub fn device(&self, env: &Env, target: String) -> napi::Result<DeviceHandle> {
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned: None,
            id: self.target(env, &target)?,
        })
    }

    /// A handle that drives the device over one mode alone.
    ///
    /// Every call on it goes over `mode` or throws. Use it where the caller
    /// serves one mode by design, such as a bridge that reaches a device over
    /// `lan`: a handle from `device()` would move to the next enabled mode
    /// when that one stops answering.
    ///
    /// `target` reads as it does for `device()`.
    #[napi]
    pub fn device_on(&self, env: &Env, target: String, mode: String) -> napi::Result<DeviceHandle> {
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned: Some(conv::mode(env, &mode)?),
            id: self.target(env, &target)?,
        })
    }

    /// Subscribe to what the SDK reports. Iterate it with `for await`.
    #[napi]
    pub fn events(&self) -> EventStream {
        EventStream::new(&self.inner)
    }

    /// Release what every transport holds. Call it before the program ends,
    /// or `ble` loses the last frame it wrote.
    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, ()>> {
        let govee = self.inner.clone();
        promise(env, async move { govee.shutdown().await })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!(
            "Govee(modes={:?}, devices={})",
            self.modes(),
            self.inner.devices().len()
        )
    }
}

impl Govee {
    fn target(&self, env: &Env, target: &str) -> napi::Result<DeviceId> {
        map(env, self.inner.target(target).map_err(Into::into))
    }
}
