//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{Filter, Govee as CoreGovee};
use pyo3::prelude::*;
use pyo3::types::PyString;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv;
use crate::device::DeviceHandle;
use crate::devices::Devices;
use crate::errors::map;
use crate::events::EventStream;
use crate::types::{Device, WalkReport};

/// The SDK. Start one and keep it: it holds the catalog, the configuration
/// and one transport per mode.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Govee")]
#[derive(Debug, Clone)]
pub(crate) struct Govee {
    pub(crate) inner: CoreGovee,
}

#[pymethods]
impl Govee {
    /// Start the SDK. Without a configuration, it reads the file. Without a
    /// catalog, it reads the one the wheel carries.
    #[staticmethod]
    #[pyo3(signature = (config=None, catalog=None))]
    fn start(
        py: Python<'_>,
        config: Option<Config>,
        catalog: Option<Catalog>,
    ) -> PyResult<Bound<'_, PyAny>> {
        let config = match config {
            Some(config) => config.inner,
            None => map(govee_toolkit::Config::load())?,
        };
        let catalog = match catalog {
            Some(catalog) => catalog.inner,
            None => map(govee_toolkit::Catalog::embedded().map_err(Into::into))?,
        };
        future_into_py(py, async move {
            Ok(Govee {
                inner: map(CoreGovee::start_with(config, catalog).await)?,
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
    #[pyo3(signature = (modes = None))]
    fn scan<'py>(
        &self,
        py: Python<'py>,
        modes: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let wanted = modes.map(conv::modes).transpose()?;
        let govee = self.inner.clone();
        future_into_py(py, async move {
            let found = map(govee.scan(wanted.as_deref()).await)?;
            Ok(found.into_iter().map(Device::from).collect::<Vec<_>>())
        })
    }

    /// The modes this build carries a transport for. Not a preference order:
    /// that is each device's own configuration.
    fn modes(&self) -> Vec<String> {
        self.inner
            .modes()
            .into_iter()
            .map(|mode| mode.to_string())
            .collect()
    }

    /// Everything wrong with the configuration, as one sentence each.
    fn problems(&self) -> Vec<String> {
        self.inner
            .problems()
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// The configuration in force.
    #[getter]
    fn config(&self) -> Config {
        Config {
            inner: self.inner.config().clone(),
        }
    }

    /// The device catalog in force.
    #[getter]
    fn catalog(&self) -> Catalog {
        Catalog {
            inner: self.inner.catalog().clone(),
        }
    }

    /// A handle for one device, by its identity or by the name that the
    /// configuration gives it. It reads the configuration and scans nothing.
    ///
    /// A bare target is a name where the configuration gives one, and an
    /// identity where it reads as one. `id:` and `name:` state the kind. A
    /// SKU or a group raises `ConfigError` with `target_not_understood`: use
    /// `devices()` for them.
    ///
    /// `mode` pins every call on the handle to that mode: each call goes over
    /// it or raises. Without it, each call goes over the first enabled mode
    /// that answers.
    #[pyo3(signature = (target, *, mode = None))]
    fn device(&self, target: &str, mode: Option<&str>) -> PyResult<DeviceHandle> {
        let pinned = mode.map(conv::mode).transpose()?;
        let id = map(self.inner.device(target, pinned))?.id().clone();
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned,
            id,
        })
    }

    /// The devices that the targets name, in the order written, as one
    /// `Devices`. A device that two targets name appears once.
    ///
    /// A target is an identity (`1C:8B:…`), a SKU (`H6159`), a name that the
    /// configuration gives a device (`name:kitchen`), or a group that it gives
    /// (`group:ambient`). `id:`, `sku:`, `name:` and `group:` state the kind
    /// where the target alone does not. An identity selects itself, and a name
    /// or a group reads the configuration. A SKU reads the devices that a scan
    /// found. Without `targets`, it selects every device that a scan finds.
    ///
    /// The first call that reads a SKU, or that has no `targets`, scans once:
    /// over `enables`, else over `mode`, else over every mode. A `scan()`
    /// counts as that scan.
    ///
    /// `enables` keeps the devices that enable that mode. A SKU, a name or a
    /// group that keeps no device then raises `ConfigError`. An identity is
    /// kept whatever it enables. `mode` pins every member, as it does for
    /// `device()`, and filters nothing: a member that does not enable it fails
    /// alone in its `Outcome`.
    #[pyo3(signature = (targets = None, *, enables = None, mode = None))]
    fn devices<'py>(
        &self,
        py: Python<'py>,
        targets: Option<Vec<String>>,
        enables: Option<&str>,
        mode: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut filter = targets.map_or_else(Filter::all, Filter::targets);
        if let Some(enables) = enables {
            filter = filter.enables(conv::mode(enables)?);
        }
        let pinned = mode.map(conv::mode).transpose()?;
        let govee = self.inner.clone();
        future_into_py(py, async move {
            let members = map(govee.devices(filter, pinned).await)?
                .iter()
                .map(|handle| handle.id().clone())
                .collect();
            Ok(Devices {
                govee: govee.clone(),
                pinned,
                members,
            })
        })
    }

    /// Run the walk `govee identify` runs. `targets` reads as `devices()`
    /// reads it; `None` walks every device that a scan finds. `wait` is the
    /// time between two steps and `hold` the time on the last device, in
    /// seconds.
    ///
    /// Raises `ConfigError` with `mode_not_enabled` before it sends a command
    /// where a device does not enable the mode. A device that fails is in the
    /// report.
    #[expect(
        clippy::too_many_arguments,
        reason = "each keyword is one Python argument"
    )]
    #[pyo3(
        signature = (targets=None, *, color=None, wait=None, hold=None, keep=None, mode=None),
        text_signature = "($self, targets=None, *, color=(0, 255, 0), wait=1.0, hold=5.0, keep=False, mode='lan')"
    )]
    fn identify<'py>(
        &self,
        py: Python<'py>,
        targets: Option<&Bound<'py, PyAny>>,
        color: Option<&Bound<'py, PyAny>>,
        wait: Option<f64>,
        hold: Option<f64>,
        keep: Option<bool>,
        mode: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let walk = conv::walk(color, wait, hold, keep, mode)?;
        let named: Option<Vec<String>> = match targets {
            None => None,
            Some(one) if one.is_instance_of::<PyString>() => Some(vec![one.extract()?]),
            Some(many) => Some(many.extract()?),
        };
        let govee = self.inner.clone();
        future_into_py(py, async move {
            let report = map(govee.identify(named.as_deref(), &walk, &()).await)?;
            Ok(WalkReport::from(report))
        })
    }

    /// Subscribe to what the SDK reports. Iterate it with `async for`.
    fn events(&self) -> EventStream {
        EventStream::new(&self.inner)
    }

    /// Release what every transport holds. Call it before the program ends,
    /// or `ble` loses the last frame it wrote.
    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let govee = self.inner.clone();
        future_into_py(py, async move {
            map(govee.shutdown().await)?;
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        format!("Govee(modes={:?})", self.modes())
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Govee>()
}
