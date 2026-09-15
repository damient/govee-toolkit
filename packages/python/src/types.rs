//! What the core reports, as Python objects.
//!
//! Every one holds the core value and reads the answers off it, so no field
//! and no rule is written twice. Every one is read-only: it is an answer, not
//! a request.

use std::collections::BTreeMap;

use govee_toolkit::transport::{Health as CoreHealth, Reply as CoreReply};
use govee_toolkit::{Device as CoreDevice, DeviceStatus as CoreStatus, Served as CoreServed};
use pyo3::prelude::*;

use crate::conv::to_py;

/// A flag as Python writes it.
fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

/// A device's health in one mode.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Health")]
#[derive(Debug, Clone)]
pub(crate) struct Health {
    inner: CoreHealth,
}

#[pymethods]
impl Health {
    /// `"ok"`, `"degraded"` or `"down"`.
    #[getter]
    fn state(&self) -> String {
        self.inner.state.to_string()
    }

    /// Consecutive unanswered verifications.
    #[getter]
    fn failures(&self) -> u32 {
        self.inner.failures
    }

    /// Whether a command would be sent right now.
    #[getter]
    fn available(&self) -> bool {
        self.inner.available
    }

    fn __repr__(&self) -> String {
        format!(
            "Health(state='{}', failures={}, available={})",
            self.state(),
            self.failures(),
            python_bool(self.available())
        )
    }
}

impl From<CoreHealth> for Health {
    fn from(inner: CoreHealth) -> Self {
        Self { inner }
    }
}

/// A device the SDK knows about.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Device")]
#[derive(Debug, Clone)]
pub(crate) struct Device {
    inner: CoreDevice,
}

#[pymethods]
impl Device {
    /// The MAC it reports, uppercased.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// The SKU it is encoded under.
    #[getter]
    fn sku(&self) -> String {
        self.inner.sku.clone()
    }

    /// The name the configuration gives it, if any.
    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// The enabled modes, in preference order.
    #[getter]
    fn modes(&self) -> Vec<String> {
        self.inner.modes.iter().map(ToString::to_string).collect()
    }

    /// Its health per enabled mode. A mode is absent when no transport has
    /// heard from it.
    #[getter]
    fn health(&self) -> BTreeMap<String, Health> {
        self.inner
            .health
            .iter()
            .map(|(mode, health)| (mode.to_string(), Health::from(*health)))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("Device(id='{}', sku='{}')", self.id(), self.inner.sku)
    }
}

impl From<CoreDevice> for Device {
    fn from(inner: CoreDevice) -> Self {
        Self { inner }
    }
}

/// A command that was served.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Served")]
#[derive(Debug, Clone)]
pub(crate) struct Served {
    inner: CoreServed,
}

#[pymethods]
impl Served {
    /// The device it went to.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// The mode that served it.
    #[getter]
    fn mode(&self) -> String {
        self.inner.mode.to_string()
    }

    /// The device file entry that was sent.
    #[getter]
    fn command(&self) -> String {
        self.inner.command.clone()
    }

    /// The name the wire carries, where it carries one.
    #[getter]
    fn cmd(&self) -> String {
        self.inner.cmd.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Served(id='{}', mode='{}', command='{}')",
            self.id(),
            self.mode(),
            self.inner.command
        )
    }
}

impl From<CoreServed> for Served {
    fn from(inner: CoreServed) -> Self {
        Self { inner }
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
    inner: CoreStatus,
}

#[pymethods]
impl DeviceStatus {
    /// Which device answered.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Whether it is on.
    #[getter]
    fn on(&self) -> Option<bool> {
        self.inner.on
    }

    /// The level it reports. A percentage on every unit seen so far, and not
    /// normalized here.
    #[getter]
    fn brightness(&self) -> Option<i64> {
        self.inner.brightness
    }

    /// The color, as three channels. Reset to `(0, 0, 0)` in white mode.
    #[getter]
    fn color(&self) -> Option<(u8, u8, u8)> {
        self.inner.color.map(|[r, g, b]| (r, g, b))
    }

    /// The white temperature. `0` means the device is in color mode.
    #[getter]
    fn color_temp_kelvin(&self) -> Option<i64> {
        self.inner.color_temp_kelvin
    }

    /// The whole reply, with every field the SDK does not model.
    #[getter]
    fn raw(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &self.inner.raw)
    }

    /// Whether the device is in white mode. Mutually exclusive with color.
    #[getter]
    fn is_white(&self) -> bool {
        self.inner.is_white()
    }

    fn __repr__(&self) -> String {
        let on = self.inner.on.map_or("None", python_bool);
        format!("DeviceStatus(id='{}', on={on})", self.id())
    }
}

impl From<CoreStatus> for DeviceStatus {
    fn from(inner: CoreStatus) -> Self {
        Self { inner }
    }
}

/// What one command's `reply:` layouts captured.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Reply")]
#[derive(Debug, Clone)]
pub(crate) struct Reply {
    inner: CoreReply,
}

#[pymethods]
impl Reply {
    /// Which device answered.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Every field the exchanges captured, by the name the device file gives
    /// it.
    #[getter]
    fn fields(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &self.inner.fields.to_json())
    }

    fn __repr__(&self) -> String {
        format!("Reply(id='{}')", self.id())
    }
}

impl From<CoreReply> for Reply {
    fn from(inner: CoreReply) -> Self {
        Self { inner }
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
