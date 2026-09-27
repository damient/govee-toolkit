//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{Filter, Govee as CoreGovee};
use napi::Env;
use napi::bindgen_prelude::{Either, Object, PromiseRaw};
use napi_derive::napi;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv;
use crate::device::DeviceHandle;
use crate::devices::Devices;
use crate::errors::map;
use crate::events::EventStream;
use crate::promise::promise;
use crate::types::{Device, WalkReport};

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

    /// Run a discovery scan and return what answered. Without `modes`, it
    /// scans every mode.
    ///
    /// The scans run at the same time, so the call takes the longest window
    /// and not their sum. A mode this build carries no transport for
    /// contributes nothing and is not an error. Nothing on the send path calls
    /// this.
    #[napi]
    pub fn scan<'env>(
        &self,
        env: &'env Env,
        modes: Option<Vec<String>>,
    ) -> napi::Result<PromiseRaw<'env, Vec<Device>>> {
        let wanted = modes.map(|modes| conv::modes(env, modes)).transpose()?;
        let govee = self.inner.clone();
        promise(env, async move {
            let found = govee.scan(wanted.as_deref()).await?;
            Ok(found.into_iter().map(Device::from).collect())
        })
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

    /// A handle for one device, by its identity or by the name the
    /// configuration gives it. It reads the configuration and no scan, and
    /// throws for a SKU or a group.
    ///
    /// A bare target is a name where the configuration gives one, and an
    /// identity where it reads as one. `id:` and `name:` state the kind.
    ///
    /// `mode` pins every call on the handle to that mode: a call goes over
    /// it or throws. Without it, each call goes over the first enabled mode
    /// that answers. Pin a mode where the caller serves one mode by design,
    /// such as a bridge that reaches a device over `lan`.
    #[napi(ts_args_type = "target: string, options?: { mode?: string }")]
    pub fn device(
        &self,
        env: &Env,
        target: String,
        options: Option<Object<'_>>,
    ) -> napi::Result<DeviceHandle> {
        let [pinned] = conv::option_modes(env, options.as_ref(), ["mode"])?;
        let handle = map(env, self.inner.device(&target, pinned))?;
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned,
            id: handle.id().clone(),
        })
    }

    /// The devices that the targets name, in the order written. A device
    /// that two targets name appears once, where it was named first.
    ///
    /// A target is an identity (`1C:8B:…`), a SKU (`H6159`), a name that the
    /// configuration gives a device (`name:kitchen`), or a group that it
    /// gives (`group:ambient`). `id:`, `sku:`, `name:` and `group:` state the
    /// kind where the target alone does not. `null` selects every device that
    /// a scan finds.
    ///
    /// A SKU, and `null`, read the devices that a scan found: the first call
    /// that needs a scan runs one, and `scan()` counts as that scan. An
    /// identity, a name and a group read the configuration and scan nothing.
    ///
    /// `enables` keeps the devices that enable that mode, and rejects with
    /// `no_such_target` where a SKU, a name or a group then keeps none. An
    /// identity is kept either way. `mode` pins every member, as for
    /// `device()`: a member that does not enable it fails alone, in its
    /// outcome.
    #[napi(
        ts_args_type = "targets?: Array<string> | null, options?: { enables?: string; mode?: string }"
    )]
    pub fn devices<'env>(
        &self,
        env: &'env Env,
        targets: Option<Vec<String>>,
        options: Option<Object<'_>>,
    ) -> napi::Result<PromiseRaw<'env, Devices>> {
        let [enables, pinned] = conv::option_modes(env, options.as_ref(), ["enables", "mode"])?;
        let mut filter = targets.map_or_else(Filter::all, Filter::targets);
        if let Some(mode) = enables {
            filter = filter.enables(mode);
        }
        let govee = self.inner.clone();
        promise(env, async move {
            let ids = govee
                .devices(filter, pinned)
                .await?
                .iter()
                .map(|handle| handle.id().clone())
                .collect();
            Ok(Devices { govee, pinned, ids })
        })
    }

    /// Run the walk `govee identify` runs. `targets` reads as `devices()`
    /// reads it; `null` walks every device that a scan finds. `wait` is the
    /// time between two steps and `hold` the time on the last device, in
    /// seconds. An unknown option is refused.
    ///
    /// Rejects with `mode_not_enabled` before it sends a command where a
    /// device does not enable the mode. A device that fails is in the report.
    ///
    /// @param [options.color=[0, 255, 0]]
    /// @param [options.wait=1]
    /// @param [options.hold=5]
    /// @param [options.keep=false]
    /// @param [options.mode='lan']
    #[napi(
        ts_args_type = "targets?: string | Array<string> | null, options?: { color?: [number, number, number] | Uint8Array; wait?: number; hold?: number; keep?: boolean; mode?: string }"
    )]
    pub fn identify<'env>(
        &self,
        env: &'env Env,
        targets: Option<Either<String, Vec<String>>>,
        options: Option<Object<'_>>,
    ) -> napi::Result<PromiseRaw<'env, WalkReport>> {
        let walk = conv::walk(env, options)?;
        let named = targets.map(|targets| match targets {
            Either::A(one) => vec![one],
            Either::B(many) => many,
        });
        let govee = self.inner.clone();
        promise(env, async move {
            let report = govee.identify(named.as_deref(), &walk, &()).await?;
            Ok(WalkReport::from(report))
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
        format!("Govee(modes={:?})", self.modes())
    }
}
