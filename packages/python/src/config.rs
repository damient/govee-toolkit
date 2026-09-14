//! The configuration, and what is wrong with it.

use govee_toolkit::Config as CoreConfig;
use pyo3::prelude::*;

use crate::errors::map;

/// The configuration in force.
///
/// Read it from the file, or build the default one and enable the modes a
/// device needs. Every field is read-only: the SDK reads the configuration
/// once, at startup.
#[pyclass(frozen, from_py_object, module = "govee_toolkit", name = "Config")]
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) inner: CoreConfig,
}

#[pymethods]
impl Config {
    /// The default configuration: `lan` alone, and no device entry.
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreConfig::default(),
        }
    }

    /// Read `$XDG_CONFIG_HOME/govee-toolkit/config.yaml`.
    ///
    /// A missing file is the default configuration, not an error. A file that
    /// does not parse raises `ConfigError`.
    #[staticmethod]
    fn load() -> PyResult<Self> {
        Ok(Self {
            inner: map(CoreConfig::load())?,
        })
    }

    /// Read the configuration from one path.
    #[staticmethod]
    fn load_from(path: std::path::PathBuf) -> PyResult<Self> {
        Ok(Self {
            inner: map(CoreConfig::load_from(path))?,
        })
    }

    /// The modes enabled for a device with no entry of its own.
    #[getter]
    fn default_modes(&self) -> Vec<String> {
        self.inner
            .defaults
            .modes
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// The identity of every device the file names.
    #[getter]
    fn devices(&self) -> Vec<String> {
        self.inner
            .devices
            .keys()
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// The rate a stream sends at when the device file measured none.
    #[getter]
    fn stream_fallback_hz(&self) -> f64 {
        self.inner.stream.fallback_hz
    }

    fn __repr__(&self) -> String {
        format!(
            "Config(default_modes={:?}, devices={})",
            self.default_modes(),
            self.inner.devices.len()
        )
    }
}

/// Add the configuration to the module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Config>()
}
