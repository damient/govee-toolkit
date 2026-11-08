//! Several devices driven as one.

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, Govee, Outcome as CoreOutcome, Paint, Served as CoreServed};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyList};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::apply::{self, Verbs};
use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::{map, to_py};
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

    /// What the call on one device raises. `None` where it succeeded.
    #[getter]
    fn error(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.error.as_ref().map(|error| error.clone_ref(py))
    }

    fn __repr__(&self) -> String {
        match (&self.mode, &self.error) {
            (Some(mode), None) => format!("Outcome(id='{}', mode='{mode}')", self.id),
            _ => format!("Outcome(id='{}', ok=False)", self.id),
        }
    }
}

impl Outcome {
    pub(crate) fn new<T>(
        outcome: CoreOutcome<T>,
        read: impl FnOnce(T) -> (Mode, Option<Served>),
    ) -> Self {
        let id = outcome.id.to_string();
        match outcome.result {
            Ok(value) => {
                let (mode, served) = read(value);
                Self {
                    id,
                    mode: Some(mode.to_string()),
                    served,
                    error: None,
                }
            }
            Err(error) => Self {
                id,
                mode: None,
                served: None,
                error: Some(Python::attach(|py| to_py(&error).into_value(py).into_any())),
            },
        }
    }
}

/// The devices that `Govee.devices()` selects. Every verb on it runs on every
/// member at once, answers one `Outcome` per member in member order, and
/// raises for nothing a member does.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "Devices"
)]
#[derive(Debug, Clone)]
pub(crate) struct Devices {
    pub(crate) govee: Govee,
    pub(crate) pinned: Option<Mode>,
    pub(crate) members: Vec<DeviceId>,
}

/// Rebuild the core `Devices` from the stored identities, then run `$call` on
/// it and read each outcome as a `Served`.
macro_rules! each {
    ($self:ident, $py:ident, |$devices:ident| $call:expr) => {{
        let (govee, pinned, members) = $self.parts();
        future_into_py($py, async move {
            let $devices = map(core(&govee, &members, pinned))?;
            Ok(served($call))
        })
    }};
}

#[pymethods]
impl Devices {
    /// One handle per member, in the order of every outcome list. Each handle
    /// is pinned as the members are.
    #[getter]
    fn members(&self) -> Vec<DeviceHandle> {
        self.handles()
    }

    /// What the SDK holds for each member: the SKU, the name, the groups, the
    /// modes and the health. Reads no hardware.
    ///
    /// A member that no transport knows and that the configuration pins no
    /// SKU for is not in the list. `members` holds every member.
    fn list(&self) -> PyResult<Vec<Device>> {
        let listed = core(&self.govee, &self.members, self.pinned).map(|devices| devices.list());
        Ok(map(listed)?.into_iter().map(Device::from).collect())
    }

    /// Scan for every member that no mode knows. Each outcome carries its
    /// mode. A pinned member scans over its mode alone.
    fn ensure_known<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, pinned, members) = self.parts();
        future_into_py(py, async move {
            let devices = map(core(&govee, &members, pinned))?;
            Ok(devices
                .ensure_known()
                .await
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |mode| (mode, None)))
                .collect::<Vec<_>>())
        })
    }

    /// Turn every member on or off.
    fn power<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        each!(self, py, |devices| devices.power(on).await)
    }

    /// Set the level on every member, against its own range.
    fn brightness<'py>(&self, py: Python<'py>, level: i64) -> PyResult<Bound<'py, PyAny>> {
        each!(self, py, |devices| devices.brightness(level).await)
    }

    /// Set one color on every member.
    fn color<'py>(&self, py: Python<'py>, rgb: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let rgb = conv::rgb(rgb)?;
        each!(self, py, |devices| devices.color(rgb).await)
    }

    /// Set the white temperature on every member, in kelvin.
    fn color_temp<'py>(&self, py: Python<'py>, kelvin: i64) -> PyResult<Bound<'py, PyAny>> {
        each!(self, py, |devices| devices.color_temp(kelvin).await)
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
        let music = conv::music(effect, sensitivity, soft, color)?;
        each!(self, py, |devices| devices.music(&music).await)
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
        zones: Option<Vec<u16>>,
        resolution: Option<&Bound<'py, PyAny>>,
        gradient: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let colors = conv::colors(colors)?;
        let resolution = conv::resolution_or_default(resolution)?;
        each!(self, py, |devices| {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            devices.segment(&paint).await
        })
    }

    /// Set the interpolation between zones on every member.
    fn gradient<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        each!(self, py, |devices| devices.gradient(on).await)
    }

    /// Scan for the members, then send the verbs, power on first and power
    /// off last. `segment` and `music` take the keys of `segment()` and
    /// `music()`. A member that fails takes no later step.
    #[pyo3(signature = (*, power=None, brightness=None, color=None, color_temp=None, segment=None, music=None, gradient=None))]
    #[allow(clippy::too_many_arguments)]
    fn apply<'py>(
        &self,
        py: Python<'py>,
        power: Option<bool>,
        brightness: Option<i64>,
        color: Option<&Bound<'py, PyAny>>,
        color_temp: Option<i64>,
        segment: Option<&Bound<'py, PyDict>>,
        music: Option<&Bound<'py, PyDict>>,
        gradient: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let verbs = Verbs {
            power,
            brightness,
            color,
            color_temp,
            segment,
            music,
            gradient,
        }
        .read()?;
        let (govee, pinned, members) = self.parts();
        future_into_py(py, apply::apply(govee, pinned, members, verbs))
    }

    fn __len__(&self) -> usize {
        self.members.len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        PyList::new(py, self.handles())?.try_iter()
    }

    fn __repr__(&self) -> String {
        let ids: Vec<String> = self.members.iter().map(ToString::to_string).collect();
        format!("Devices(members={ids:?})")
    }
}

impl Devices {
    fn parts(&self) -> (Govee, Option<Mode>, Vec<DeviceId>) {
        (self.govee.clone(), self.pinned, self.members.clone())
    }

    fn handles(&self) -> Vec<DeviceHandle> {
        self.members
            .iter()
            .map(|id| DeviceHandle {
                govee: self.govee.clone(),
                pinned: self.pinned,
                id: id.clone(),
            })
            .collect()
    }
}

fn served(outcomes: Vec<CoreOutcome<CoreServed>>) -> Vec<Outcome> {
    outcomes
        .into_iter()
        .map(|outcome| Outcome::new(outcome, |served| (served.mode, Some(Served::from(served)))))
        .collect()
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Devices>()?;
    module.add_class::<Outcome>()?;
    module.add_class::<apply::Applied>()?;
    module.add_class::<apply::AppliedStep>()
}

/// The core set of the stored identities. It parses nothing and scans
/// nothing.
pub(crate) fn core<'a>(
    govee: &'a Govee,
    ids: &[DeviceId],
    pinned: Option<Mode>,
) -> govee_toolkit::Result<govee_toolkit::Devices<'a>> {
    ids.iter().map(|id| govee.device(id, pinned)).collect()
}
