//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{Filter, Govee as CoreGovee, input};
use napi::Env;
use napi::bindgen_prelude::{Either, Object, PromiseRaw};
use napi_derive::napi;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv::{self, STYLE};
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
    /// catalog, it reads the one the package carries, with the local device
    /// files over it where the configuration opts in.
    #[napi]
    pub fn start<'env>(
        env: &'env Env,
        config: Option<&Config>,
        catalog: Option<&Catalog>,
    ) -> napi::Result<PromiseRaw<'env, Govee>> {
        let config = config.map(|config| config.inner.clone());
        let catalog = catalog.map(|catalog| catalog.inner.clone());
        promise(env, async move {
            Ok(Govee {
                inner: CoreGovee::start_or_defaults(config, catalog).await?,
            })
        })
    }

    /// Run a discovery scan and return what answered. Without `modes`, it
    /// scans every mode. The modes scan at the same time. A mode that this
    /// build carries no transport for adds nothing and is not an error.
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

    /// A handle for one device, by its identity or by its configured name.
    /// `id:` and `name:` state the kind. It scans nothing. A SKU or a group
    /// throws `target_not_understood`: use `devices()` for them.
    ///
    /// `mode` pins every call on the handle to that mode. Without it, each
    /// call goes over the first enabled mode that answers.
    #[napi(ts_args_type = "target: string, options?: { mode?: string }")]
    pub fn device(
        &self,
        env: &Env,
        target: String,
        options: Option<Object<'_>>,
    ) -> napi::Result<DeviceHandle> {
        let [pinned] = conv::option_modes(env, options.as_ref(), ["mode"])?;
        Ok(DeviceHandle {
            inner: map(env, self.inner.device_owned(&target, pinned))?,
        })
    }

    /// The devices that the targets name, each once, in the order written. A
    /// target is an identity, a SKU, a configured name or a group: `id:`,
    /// `sku:`, `name:` and `group:` state the kind. `null` selects every device
    /// that a scan finds. A SKU or `null` runs one scan on the first call, and
    /// `scan()` counts as that scan.
    ///
    /// `enables` keeps the devices that enable that mode, and rejects with
    /// `no_such_target` where a SKU, a name or a group keeps none. `mode` pins
    /// every member as for `device()`: a member that does not enable it fails
    /// alone, in its outcome.
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
        let filter = Filter::from_parts(targets, enables);
        let govee = self.inner.clone();
        promise(env, async move {
            Ok(Devices {
                inner: govee.devices_owned(filter, pinned).await?,
            })
        })
    }

    /// Run the walk `govee identify` runs. `targets` reads as for `devices()`.
    /// `wait` and `hold` are in seconds. An unknown option is refused.
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
        let walk = conv::read(
            env,
            input::walk(&conv::options(env, options.as_ref())?, STYLE),
        )?;
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
