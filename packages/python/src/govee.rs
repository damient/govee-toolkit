//! The facade: what starts the SDK, what it knows, and what it reaches.

use govee_toolkit::{DeviceId, Govee as CoreGovee};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::catalog::Catalog;
use crate::config::Config;
use crate::conv;
use crate::device::DeviceHandle;
use crate::driver::Driver;
use crate::errors::map;
use crate::events::EventStream;
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

    /// A handle for one device, by the MAC it reports.
    fn device(&self, id: &str) -> DeviceHandle {
        DeviceHandle {
            govee: Driver::new(self.inner.clone(), None),
            id: DeviceId::new(id),
        }
    }

    /// A handle that drives the device over one mode alone.
    ///
    /// Every call on it goes over `mode` or raises. Use it where the caller
    /// serves one mode by design, such as a bridge that reaches a device over
    /// `lan`: a handle from `device()` would move to the next enabled mode
    /// when that one stops answering.
    fn device_on(&self, id: &str, mode: &str) -> PyResult<DeviceHandle> {
        Ok(DeviceHandle {
            govee: Driver::new(self.inner.clone(), Some(conv::mode(mode)?)),
            id: DeviceId::new(id),
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

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Govee>()
}
