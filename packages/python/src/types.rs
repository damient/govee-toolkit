//! What the core reports, as Python objects.
//!
//! Every one is read-only: it is an answer, not a request.

use std::collections::BTreeMap;

use govee_toolkit::transport::{Health as CoreHealth, Reply as CoreReply};
use govee_toolkit::{Device as CoreDevice, DeviceStatus as CoreStatus, Served as CoreServed};
use pyo3::prelude::*;

use crate::conv::{mode_name, to_py};

/// A flag as Python writes it.
fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

/// A device's health in one mode.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Health")]
#[derive(Debug, Clone)]
pub(crate) struct Health {
    /// `"ok"`, `"degraded"` or `"down"`.
    #[pyo3(get)]
    pub(crate) state: String,
    /// Consecutive unanswered verifications.
    #[pyo3(get)]
    pub(crate) failures: u32,
    /// Whether a command would be sent right now.
    #[pyo3(get)]
    pub(crate) available: bool,
}

#[pymethods]
impl Health {
    fn __repr__(&self) -> String {
        format!(
            "Health(state='{}', failures={}, available={})",
            self.state,
            self.failures,
            python_bool(self.available)
        )
    }
}

impl From<CoreHealth> for Health {
    fn from(health: CoreHealth) -> Self {
        Self {
            state: health.state.to_string(),
            failures: health.failures,
            available: health.available,
        }
    }
}

/// A device the SDK knows about.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Device")]
#[derive(Debug, Clone)]
pub(crate) struct Device {
    /// The MAC it reports, uppercased.
    #[pyo3(get)]
    pub(crate) id: String,
    /// The SKU it is encoded under.
    #[pyo3(get)]
    pub(crate) sku: String,
    /// The name the configuration gives it, if any.
    #[pyo3(get)]
    pub(crate) name: Option<String>,
    /// The enabled modes, in preference order.
    #[pyo3(get)]
    pub(crate) modes: Vec<String>,
    /// Its health per enabled mode. A mode is absent when no transport has
    /// heard from it.
    #[pyo3(get)]
    pub(crate) health: BTreeMap<String, Health>,
}

#[pymethods]
impl Device {
    fn __repr__(&self) -> String {
        format!("Device(id='{}', sku='{}')", self.id, self.sku)
    }
}

impl From<CoreDevice> for Device {
    fn from(device: CoreDevice) -> Self {
        Self {
            id: device.id.to_string(),
            sku: device.sku,
            name: device.name,
            modes: device
                .modes
                .iter()
                .map(|m| mode_name(*m).to_owned())
                .collect(),
            health: device
                .health
                .into_iter()
                .map(|(mode, health)| (mode_name(mode).to_owned(), health.into()))
                .collect(),
        }
    }
}

/// A command that was served.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Served")]
#[derive(Debug, Clone)]
pub(crate) struct Served {
    /// The device it went to.
    #[pyo3(get)]
    pub(crate) id: String,
    /// The mode that served it.
    #[pyo3(get)]
    pub(crate) mode: String,
    /// The device file entry that was sent.
    #[pyo3(get)]
    pub(crate) command: String,
    /// The name the wire carries, where it carries one.
    #[pyo3(get)]
    pub(crate) cmd: String,
}

#[pymethods]
impl Served {
    fn __repr__(&self) -> String {
        format!(
            "Served(id='{}', mode='{}', command='{}')",
            self.id, self.mode, self.command
        )
    }
}

impl From<CoreServed> for Served {
    fn from(served: CoreServed) -> Self {
        Self {
            id: served.id.to_string(),
            mode: mode_name(served.mode).to_owned(),
            command: served.command,
            cmd: served.cmd,
        }
    }
}

/// What a device reported about itself. Every field is optional: no firmware
/// fills them all in.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "DeviceStatus"
)]
#[derive(Debug, Clone)]
pub(crate) struct DeviceStatus {
    /// Which device answered.
    #[pyo3(get)]
    pub(crate) id: String,
    /// Whether it is on.
    #[pyo3(get)]
    pub(crate) on: Option<bool>,
    /// The level it reports. A percentage on every unit seen so far, and not
    /// normalized here.
    #[pyo3(get)]
    pub(crate) brightness: Option<i64>,
    /// The color, as three channels. Reset to `(0, 0, 0)` in white mode.
    #[pyo3(get)]
    pub(crate) color: Option<(u8, u8, u8)>,
    /// The white temperature. `0` means the device is in color mode.
    #[pyo3(get)]
    pub(crate) color_temp_kelvin: Option<i64>,
    raw: serde_json::Value,
}

#[pymethods]
impl DeviceStatus {
    /// The whole reply, with every field the SDK does not model.
    #[getter]
    fn raw(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &self.raw)
    }

    /// Whether the device is in white mode. Mutually exclusive with color.
    #[getter]
    fn is_white(&self) -> bool {
        self.color_temp_kelvin.is_some_and(|k| k > 0)
    }

    fn __repr__(&self) -> String {
        let on = self.on.map_or("None", python_bool);
        format!("DeviceStatus(id='{}', on={on})", self.id)
    }
}

impl From<CoreStatus> for DeviceStatus {
    fn from(status: CoreStatus) -> Self {
        Self {
            id: status.id.to_string(),
            on: status.on,
            brightness: status.brightness,
            color: status.color.map(|[r, g, b]| (r, g, b)),
            color_temp_kelvin: status.color_temp_kelvin,
            raw: status.raw,
        }
    }
}

/// What one command's `reply:` layouts captured.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Reply")]
#[derive(Debug, Clone)]
pub(crate) struct Reply {
    /// Which device answered.
    #[pyo3(get)]
    pub(crate) id: String,
    fields: serde_json::Value,
}

#[pymethods]
impl Reply {
    /// Every field the exchanges captured, by the name the device file gives
    /// it.
    #[getter]
    fn fields(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &self.fields)
    }

    fn __repr__(&self) -> String {
        format!("Reply(id='{}')", self.id)
    }
}

impl From<CoreReply> for Reply {
    fn from(reply: CoreReply) -> Self {
        Self {
            id: reply.id.to_string(),
            fields: reply.fields.to_json(),
        }
    }
}

/// Add the answer types to the module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Health>()?;
    module.add_class::<Device>()?;
    module.add_class::<Served>()?;
    module.add_class::<DeviceStatus>()?;
    module.add_class::<Reply>()?;
    Ok(())
}
