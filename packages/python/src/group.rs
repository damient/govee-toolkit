//! Several devices driven as one, and what each member answered.
//!
//! A verb on a group raises for nothing a member does: every member answers
//! its own `Outcome`, and a member that fails stops no other one.

use std::future::Future;

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, Govee, Music, Outcome as CoreOutcome, Paint, Served as CoreServed};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv;
use crate::errors::to_py;
use crate::types::Served;

/// What one member answered: `served` or `mode` where the call succeeded,
/// and `error` where it failed.
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

    /// The command that was served. `None` where the call failed, and for
    /// `ensure_known()`.
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
    fn new<T>(outcome: CoreOutcome<T>, read: impl FnOnce(T) -> (Mode, Option<Served>)) -> Self {
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

/// A handle on the members of a group. It holds no state of its own.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "GroupHandle"
)]
#[derive(Debug, Clone)]
pub(crate) struct GroupHandle {
    pub(crate) govee: Govee,
    pub(crate) pinned: Option<Mode>,
    pub(crate) members: Vec<DeviceId>,
}

#[pymethods]
impl GroupHandle {
    /// The identities of the members, in the order every outcome list follows.
    #[getter]
    fn members(&self) -> Vec<String> {
        self.members.iter().map(ToString::to_string).collect()
    }

    /// Scan for every member that no mode knows yet. Each outcome carries the
    /// mode a command would go over.
    fn ensure_known<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, pinned, members) = self.parts();
        future_into_py(py, async move {
            let known = govee.group_maybe_on(&members, pinned).ensure_known().await;
            Ok(known
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |mode| (mode, None)))
                .collect::<Vec<_>>())
        })
    }

    /// Turn every member on or off.
    fn power<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).power(on).await
        })
    }

    /// Set the level on every member. The range is each member's own.
    fn brightness<'py>(&self, py: Python<'py>, level: i64) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, move |govee, pinned, members| async move {
            govee
                .group_maybe_on(&members, pinned)
                .brightness(level)
                .await
        })
    }

    /// Set one color on every member.
    fn color<'py>(&self, py: Python<'py>, rgb: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let rgb = conv::rgb(rgb)?;
        self.served(py, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).color(rgb).await
        })
    }

    /// Set the white temperature on every member, in kelvin.
    fn color_temp<'py>(&self, py: Python<'py>, kelvin: i64) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, move |govee, pinned, members| async move {
            govee
                .group_maybe_on(&members, pinned)
                .color_temp(kelvin)
                .await
        })
    }

    /// Play an effect on every member, as `DeviceHandle.music()` does.
    #[pyo3(signature = (effect, sensitivity=None, soft=None, color=None))]
    fn music<'py>(
        &self,
        py: Python<'py>,
        effect: i64,
        sensitivity: Option<i64>,
        soft: Option<bool>,
        color: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let default = Music::default();
        let music = Music {
            effect,
            sensitivity: sensitivity.unwrap_or(default.sensitivity),
            soft: soft.unwrap_or(default.soft),
            color: color.map(conv::rgb).transpose()?,
        };
        self.served(py, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).music(&music).await
        })
    }

    /// Paint the segments of every member once, as `DeviceHandle.segment()`
    /// does. A zone list reads against each member's own zones.
    #[pyo3(signature = (colors, zones=None, resolution=None, gradient=false))]
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
        self.served(py, move |govee, pinned, members| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            govee.group_maybe_on(&members, pinned).segment(&paint).await
        })
    }

    /// Set the interpolation between zones on every member.
    fn gradient<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).gradient(on).await
        })
    }

    fn __repr__(&self) -> String {
        format!("GroupHandle(members={:?})", self.members())
    }
}

impl GroupHandle {
    fn parts(&self) -> (Govee, Option<Mode>, Vec<DeviceId>) {
        (self.govee.clone(), self.pinned, self.members.clone())
    }

    fn served<'py, Fut>(
        &self,
        py: Python<'py>,
        verb: impl FnOnce(Govee, Option<Mode>, Vec<DeviceId>) -> Fut,
    ) -> PyResult<Bound<'py, PyAny>>
    where
        Fut: Future<Output = Vec<CoreOutcome<CoreServed>>> + Send + 'static,
    {
        let (govee, pinned, members) = self.parts();
        let call = verb(govee, pinned, members);
        future_into_py(py, async move {
            Ok(call
                .await
                .into_iter()
                .map(|outcome| {
                    Outcome::new(outcome, |served| (served.mode, Some(Served::from(served))))
                })
                .collect::<Vec<_>>())
        })
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<GroupHandle>()?;
    module.add_class::<Outcome>()
}
