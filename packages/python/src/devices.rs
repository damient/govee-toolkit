//! Several devices, and the verbs each member answers alone.

use std::future::Future;

use govee_toolkit::{Answer, Music, Outcome as CoreOutcome, OwnedDevices, Style, input};
use pyo3::prelude::*;
use pyo3::types::{PyIterator, PyList};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::apply::Applied;
use crate::conv::{self, read};
use crate::device::DeviceHandle;
use crate::errors::to_py;
use crate::types::{Device, Served};

/// What one member answered.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "Outcome"
)]
#[derive(Debug)]
pub(crate) struct Outcome {
    id: String,
    mode: Option<String>,
    served: Option<Served>,
    error: Option<Py<PyAny>>,
    record: serde_json::Value,
}

#[pymethods]
impl Outcome {
    /// The member.
    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    /// Whether the call on this member succeeded.
    #[getter]
    fn ok(&self) -> bool {
        self.error.is_none()
    }

    /// The mode that served the call. `None` where it failed.
    #[getter]
    fn mode(&self) -> Option<String> {
        self.mode.clone()
    }

    /// The command served. `None` on a failure and for `ensure_known()`.
    #[getter]
    fn served(&self) -> Option<Served> {
        self.served.clone()
    }

    /// What the call on this member raises. `None` where it succeeded.
    #[getter]
    fn error(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.error.as_ref().map(|error| error.clone_ref(py))
    }

    /// The record that every surface prints: `id`, `ok`, `mode`, `command`,
    /// and `error` as a dict of `code` and `message`.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        conv::to_py(py, &self.record)
    }

    fn __repr__(&self) -> String {
        match (&self.mode, &self.error) {
            (Some(mode), None) => format!("Outcome(id='{}', mode='{mode}')", self.id),
            _ => format!("Outcome(id='{}', ok=False)", self.id),
        }
    }
}

impl Outcome {
    fn new<T: Answer>(outcome: &CoreOutcome<T>) -> Self {
        Self {
            id: outcome.id.to_string(),
            mode: outcome.mode().map(|mode| mode.to_string()),
            served: outcome.served().cloned().map(Served::from),
            error: outcome
                .error()
                .map(|error| Python::attach(|py| to_py(error).into_value(py).into_any())),
            record: outcome.to_json(),
        }
    }
}

pub(crate) fn outcomes<T: Answer>(outcomes: &[CoreOutcome<T>]) -> Vec<Outcome> {
    outcomes.iter().map(Outcome::new).collect()
}

/// The devices that `Govee.devices()` selects. A verb runs on every member at
/// once, answers one `Outcome` per member, and raises for nothing a member
/// does.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "Devices"
)]
#[derive(Debug, Clone)]
pub(crate) struct Devices {
    pub(crate) inner: OwnedDevices,
}

#[pymethods]
impl Devices {
    /// One handle per member, in outcome order, with the mode that
    /// `Govee.devices()` pins.
    #[getter]
    fn members(&self) -> Vec<DeviceHandle> {
        self.handles()
    }

    /// What the SDK holds for each member. Reads no hardware. A member that no
    /// transport knows and that the configuration gives no SKU is left out.
    fn list(&self) -> Vec<Device> {
        self.inner
            .devices()
            .list()
            .into_iter()
            .map(Device::from)
            .collect()
    }

    /// Scan for every member that no mode knows.
    fn ensure_known<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.each(
            py,
            |owned| async move { owned.devices().ensure_known().await },
        )
    }

    /// Turn every member on or off.
    fn power<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.each(
            py,
            move |owned| async move { owned.devices().power(on).await },
        )
    }

    /// Set the level on every member, against its own range.
    fn brightness<'py>(&self, py: Python<'py>, level: i64) -> PyResult<Bound<'py, PyAny>> {
        self.each(py, move |owned| async move {
            owned.devices().brightness(level).await
        })
    }

    /// Set one color on every member.
    fn color<'py>(&self, py: Python<'py>, rgb: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let rgb = conv::rgb(rgb)?;
        self.each(
            py,
            move |owned| async move { owned.devices().color(rgb).await },
        )
    }

    /// Set the white temperature on every member, in kelvin.
    fn color_temp<'py>(&self, py: Python<'py>, kelvin: i64) -> PyResult<Bound<'py, PyAny>> {
        self.each(py, move |owned| async move {
            owned.devices().color_temp(kelvin).await
        })
    }

    /// `DeviceHandle.music()` on every member.
    #[pyo3(
        signature = (effect, sensitivity=None, soft=None, color=None),
        text_signature = "($self, effect, sensitivity=50, soft=False, color=None)"
    )]
    fn music<'py>(
        &self,
        py: Python<'py>,
        effect: i64,
        sensitivity: Option<i64>,
        soft: Option<bool>,
        color: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let color = color.map(conv::rgb).transpose()?;
        let music = Music::new(effect, sensitivity, soft, color);
        self.each(py, move |owned| async move {
            owned.devices().music(&music).await
        })
    }

    /// `DeviceHandle.segment()` on every member, against its own zones.
    #[pyo3(
        signature = (colors, zones=None, resolution=None, gradient=false),
        text_signature = "($self, colors, zones=None, resolution='app', gradient=False)"
    )]
    fn segment<'py>(
        &self,
        py: Python<'py>,
        colors: &Bound<'py, PyAny>,
        zones: Option<&Bound<'py, PyAny>>,
        resolution: Option<&Bound<'py, PyAny>>,
        gradient: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let painting = conv::painting(colors, zones, resolution, gradient)?;
        self.each(py, move |owned| async move {
            owned.devices().segment(&painting.paint()).await
        })
    }

    /// Set the interpolation between zones on every member.
    fn gradient<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.each(py, move |owned| async move {
            owned.devices().gradient(on).await
        })
    }

    /// Scan for the members, then send the verbs, power on first and power
    /// off last. `segment` and `music` take the keys of `segment()` and
    /// `music()`. A member that fails takes no later step.
    #[pyo3(signature = (*, power=None, brightness=None, color=None, color_temp=None, segment=None, music=None, gradient=None))]
    #[expect(
        clippy::too_many_arguments,
        reason = "each keyword is one Python argument"
    )]
    fn apply<'py>(
        &self,
        py: Python<'py>,
        power: Option<&Bound<'py, PyAny>>,
        brightness: Option<&Bound<'py, PyAny>>,
        color: Option<&Bound<'py, PyAny>>,
        color_temp: Option<&Bound<'py, PyAny>>,
        segment: Option<&Bound<'py, PyAny>>,
        music: Option<&Bound<'py, PyAny>>,
        gradient: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = conv::keywords(&[
            ("power", power),
            ("brightness", brightness),
            ("color", color),
            ("color_temp", color_temp),
            ("segment", segment),
            ("music", music),
            ("gradient", gradient),
        ])?;
        let verbs = read(input::verbs(&options, Style::Python))?;
        let owned = self.inner.clone();
        future_into_py(py, async move {
            let applied = owned.devices().apply(verbs).await;
            Applied::new(&applied)
        })
    }

    fn __len__(&self) -> usize {
        self.inner.ids().len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        PyList::new(py, self.handles())?.try_iter()
    }

    fn __repr__(&self) -> String {
        let ids: Vec<String> = self.inner.ids().iter().map(ToString::to_string).collect();
        format!("Devices(members={ids:?})")
    }
}

impl Devices {
    fn handles(&self) -> Vec<DeviceHandle> {
        self.inner
            .members()
            .into_iter()
            .map(|inner| DeviceHandle { inner })
            .collect()
    }

    fn each<'py, T, Fut>(
        &self,
        py: Python<'py>,
        call: impl FnOnce(OwnedDevices) -> Fut,
    ) -> PyResult<Bound<'py, PyAny>>
    where
        T: Answer,
        Fut: Future<Output = Vec<CoreOutcome<T>>> + Send + 'static,
    {
        let call = call(self.inner.clone());
        future_into_py(py, async move { Ok(outcomes(&call.await)) })
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Devices>()?;
    module.add_class::<Outcome>()?;
    module.add_class::<Applied>()?;
    module.add_class::<crate::apply::AppliedStep>()
}
