//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{DeviceId, Govee as CoreGovee};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::map;
use crate::events::EventStream;
use crate::group::GroupHandle;
use crate::types::Device;

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

    /// Run a discovery scan on every mode and return what answered.
    ///
    /// The scans run at the same time, so the call takes the longest window
    /// and not their sum. Nothing on the send path calls this.
    fn scan<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let govee = self.inner.clone();
        future_into_py(py, async move {
            let found = map(govee.scan().await)?;
            Ok(found.into_iter().map(Device::from).collect::<Vec<_>>())
        })
    }

    /// Run a discovery scan on the modes named.
    ///
    /// A mode this build carries no transport for contributes nothing and is
    /// not an error.
    fn scan_on<'py>(&self, py: Python<'py>, modes: Vec<String>) -> PyResult<Bound<'py, PyAny>> {
        let wanted = conv::modes(modes)?;
        let govee = self.inner.clone();
        future_into_py(py, async move {
            let found = map(govee.scan_on(&wanted).await)?;
            Ok(found.into_iter().map(Device::from).collect::<Vec<_>>())
        })
    }

    /// Every device known, across every mode. One reachable over two modes
    /// appears once.
    fn devices(&self) -> Vec<Device> {
        self.inner.devices().into_iter().map(Device::from).collect()
    }

    /// The devices the targets name, in the order they were written.
    ///
    /// A target is an identity (`1C:8B:…`), a SKU (`H6159`), a name the
    /// configuration gives a device (`name:kitchen`), or a group it gives
    /// (`group:ambient`). `id:`, `sku:`, `name:` and `group:` state the kind
    /// where the target alone does not. A SKU, a name and a group select
    /// among the devices the SDK knows, so scan first.
    ///
    /// `mode` is the one mode the caller will drive. A SKU and a name then
    /// match among the devices that enable it. An identity selects itself
    /// either way.
    #[pyo3(signature = (targets, mode = None))]
    fn select(&self, targets: Vec<String>, mode: Option<String>) -> PyResult<Vec<String>> {
        let only = mode.map(|name| conv::mode(&name)).transpose()?;
        let chosen = map(self
            .inner
            .select(targets, only)
            .map_err(govee_toolkit::Error::from))?;
        Ok(chosen.iter().map(ToString::to_string).collect())
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

    /// A handle for one device, by the identity it reports or by the name
    /// the configuration gives.
    ///
    /// `id:…` and `name:…` state the kind. A bare target is a name where the
    /// configuration gives one, and an identity otherwise. It reads the
    /// configuration and no scan.
    fn device(&self, target: &str) -> PyResult<DeviceHandle> {
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned: None,
            id: self.target(target)?,
        })
    }

    /// A handle that drives the device over one mode alone.
    ///
    /// Every call on it goes over `mode` or raises. Use it where the caller
    /// serves one mode by design, such as a bridge that reaches a device over
    /// `lan`: a handle from `device()` would move to the next enabled mode
    /// when that one stops answering.
    ///
    /// `target` reads as it does for `device()`.
    fn device_on(&self, target: &str, mode: &str) -> PyResult<DeviceHandle> {
        Ok(DeviceHandle {
            govee: self.inner.clone(),
            pinned: Some(conv::mode(mode)?),
            id: self.target(target)?,
        })
    }

    /// The identities that one target names: one device, or every member of
    /// a group the configuration gives, in identity order.
    ///
    /// `group:…` states the kind. A bare target is a group where no device
    /// carries it as a name. It reads the configuration and no scan.
    fn targets(&self, target: &str) -> PyResult<Vec<String>> {
        Ok(self
            .members(target)?
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    /// A handle for one device or one group. Every verb on it answers one
    /// `Outcome` per member and raises for nothing a member does.
    ///
    /// `target` reads as it does for `targets()`. `mode` pins every member
    /// to one mode, as `device_on()` does.
    #[pyo3(signature = (target, mode = None))]
    fn group(&self, target: &str, mode: Option<String>) -> PyResult<GroupHandle> {
        Ok(GroupHandle {
            govee: self.inner.clone(),
            pinned: mode.map(|name| conv::mode(&name)).transpose()?,
            members: self.members(target)?,
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
        format!(
            "Govee(modes={:?}, devices={})",
            self.modes(),
            self.inner.devices().len()
        )
    }
}

impl Govee {
    fn members(&self, target: &str) -> PyResult<Vec<DeviceId>> {
        map(self
            .inner
            .targets(target)
            .map_err(govee_toolkit::Error::from))
    }

    fn target(&self, target: &str) -> PyResult<DeviceId> {
        map(self
            .inner
            .target(target)
            .map_err(govee_toolkit::Error::from))
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Govee>()
}
