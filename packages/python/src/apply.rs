//! `DeviceHandle.invoke()` and `Devices.apply()`.

use govee_toolkit::codec::{Mode, Supplied};
use govee_toolkit::{Applied as CoreApplied, DeviceId, Govee, Invoked as CoreInvoked, Verb};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::conv::{self, to_py};
use crate::devices::Outcome;
use crate::errors::{map, value_error};
use crate::types::Served;

/// What `DeviceHandle.invoke()` did with a command.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "Invoked"
)]
#[derive(Debug)]
pub(crate) struct Invoked {
    id: String,
    mode: String,
    command: String,
    fields: Option<serde_json::Value>,
}

#[pymethods]
impl Invoked {
    /// The device.
    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    /// The mode that served the command.
    #[getter]
    fn mode(&self) -> String {
        self.mode.clone()
    }

    /// The command, as the device file names it.
    #[getter]
    fn command(&self) -> String {
        self.command.clone()
    }

    /// What the `reply:` layouts captured, or `None` for a command that was
    /// sent and not read.
    #[getter]
    fn fields(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.fields
            .as_ref()
            .map(|fields| to_py(py, fields))
            .transpose()
    }

    fn __repr__(&self) -> String {
        format!(
            "Invoked(id='{}', mode='{}', command='{}')",
            self.id, self.mode, self.command
        )
    }
}

pub(crate) async fn invoke(
    govee: Govee,
    pinned: Option<Mode>,
    id: DeviceId,
    command: String,
    supplied: Vec<(String, Supplied)>,
    refuse_secrets: bool,
) -> PyResult<Invoked> {
    let handle = map(govee.device(&id, pinned))?;
    let call = map(handle.resolve())?;
    if refuse_secrets {
        map(call.refuse_secret(&command))?;
    }
    let values = map(call.args(&command, supplied))?;
    let mode = call.mode().to_string();
    Ok(match map(call.invoke(&command, &values).await)? {
        CoreInvoked::Sent(served) => Invoked {
            id: served.id.to_string(),
            mode,
            command,
            fields: None,
        },
        CoreInvoked::Read(reply) => Invoked {
            id: reply.id.to_string(),
            mode,
            command,
            fields: Some(reply.fields.to_json()),
        },
    })
}

/// What one verb of `Devices.apply()` answered.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "AppliedStep"
)]
#[derive(Debug)]
pub(crate) struct AppliedStep {
    step: &'static str,
    outcomes: Vec<Py<Outcome>>,
}

#[pymethods]
impl AppliedStep {
    /// The verb, in snake case: `color_temp`.
    #[getter]
    fn step(&self) -> &'static str {
        self.step
    }

    /// One per member that the step reached, in member order.
    #[getter]
    fn outcomes(&self, py: Python<'_>) -> Vec<Py<Outcome>> {
        self.outcomes.iter().map(|o| o.clone_ref(py)).collect()
    }

    fn __repr__(&self) -> String {
        format!("AppliedStep(step='{}')", self.step)
    }
}

/// What `Devices.apply()` answered.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "Applied"
)]
#[derive(Debug)]
pub(crate) struct Applied {
    reached: Vec<Py<Outcome>>,
    steps: Vec<Py<AppliedStep>>,
    ok: bool,
}

#[pymethods]
impl Applied {
    /// The scan of every member, run before the verbs.
    #[getter]
    fn reached(&self, py: Python<'_>) -> Vec<Py<Outcome>> {
        self.reached.iter().map(|o| o.clone_ref(py)).collect()
    }

    /// One per verb, in the order sent.
    #[getter]
    fn steps(&self, py: Python<'_>) -> Vec<Py<AppliedStep>> {
        self.steps.iter().map(|s| s.clone_ref(py)).collect()
    }

    /// Whether every member took every step.
    #[getter]
    fn ok(&self) -> bool {
        self.ok
    }

    fn __repr__(&self) -> String {
        format!("Applied(steps={}, ok={})", self.steps.len(), self.ok)
    }
}

pub(crate) async fn apply(
    govee: Govee,
    pinned: Option<Mode>,
    members: Vec<DeviceId>,
    verbs: Vec<Verb>,
) -> PyResult<Applied> {
    let devices = map(crate::devices::core(&govee, &members, pinned))?;
    let applied: CoreApplied = devices.apply(verbs).await;
    let ok = applied.is_clean();
    let reached: Vec<Outcome> = applied
        .reached
        .into_iter()
        .map(|outcome| Outcome::new(outcome, |mode| (mode, None)))
        .collect();
    let steps: Vec<(&'static str, Vec<Outcome>)> = applied
        .steps
        .into_iter()
        .map(|step| {
            let outcomes = step
                .outcomes
                .into_iter()
                .map(|outcome| {
                    Outcome::new(outcome, |served| (served.mode, Some(Served::from(served))))
                })
                .collect();
            (step.name, outcomes)
        })
        .collect();
    Python::attach(|py| {
        let reached = reached
            .into_iter()
            .map(|outcome| Py::new(py, outcome))
            .collect::<PyResult<_>>()?;
        let steps = steps
            .into_iter()
            .map(|(step, outcomes)| {
                let outcomes = outcomes
                    .into_iter()
                    .map(|outcome| Py::new(py, outcome))
                    .collect::<PyResult<_>>()?;
                Py::new(py, AppliedStep { step, outcomes })
            })
            .collect::<PyResult<_>>()?;
        Ok(Applied { reached, steps, ok })
    })
}

const SEGMENT: [&str; 4] = ["colors", "zones", "resolution", "gradient"];
const MUSIC: [&str; 4] = ["effect", "sensitivity", "soft", "color"];

/// A misspelled key is refused: ignored, it reads as a setting that failed.
fn known_keys(dict: &Bound<'_, PyDict>, what: &str, keys: &[&str]) -> PyResult<()> {
    for key in dict.keys() {
        let key: String = key.extract()?;
        if !keys.contains(&key.as_str()) {
            return Err(value_error(format!(
                "`{key}` is no {what} key; the keys are {}",
                keys.join(", ")
            )));
        }
    }
    Ok(())
}

/// The keyword arguments of `Devices.apply()`.
pub(crate) struct Verbs<'a, 'py> {
    pub(crate) power: Option<bool>,
    pub(crate) brightness: Option<i64>,
    pub(crate) color: Option<&'a Bound<'py, PyAny>>,
    pub(crate) color_temp: Option<i64>,
    pub(crate) segment: Option<&'a Bound<'py, PyDict>>,
    pub(crate) music: Option<&'a Bound<'py, PyDict>>,
    pub(crate) gradient: Option<bool>,
}

impl Verbs<'_, '_> {
    pub(crate) fn read(&self) -> PyResult<Vec<Verb>> {
        let mut verbs = Vec::new();
        if let Some(on) = self.power {
            verbs.push(Verb::Power(on));
        }
        if let Some(level) = self.brightness {
            verbs.push(Verb::Brightness(level));
        }
        if let Some(color) = self.color {
            verbs.push(Verb::Color(conv::rgb(color)?));
        }
        if let Some(kelvin) = self.color_temp {
            verbs.push(Verb::ColorTemp(kelvin));
        }
        if let Some(segment) = self.segment {
            verbs.push(segment_verb(segment)?);
        }
        if let Some(music) = self.music {
            verbs.push(music_verb(music)?);
        }
        if let Some(on) = self.gradient {
            verbs.push(Verb::Gradient(on));
        }
        Ok(verbs)
    }
}

fn segment_verb(segment: &Bound<'_, PyDict>) -> PyResult<Verb> {
    known_keys(segment, "segment", &SEGMENT)?;
    let Some(colors) = segment.get_item("colors")? else {
        return Err(value_error("a segment carries `colors`"));
    };
    let zones = match segment.get_item("zones")? {
        Some(zones) if !zones.is_none() => Some(zones.extract::<Vec<u16>>()?),
        _ => None,
    };
    let resolution = segment.get_item("resolution")?.filter(|r| !r.is_none());
    let gradient = match segment.get_item("gradient")? {
        Some(gradient) if !gradient.is_none() => gradient.extract::<bool>()?,
        _ => false,
    };
    Ok(Verb::Segment {
        zones,
        colors: conv::colors(&colors)?,
        resolution: conv::resolution_or_default(resolution.as_ref())?,
        gradient,
    })
}

fn music_verb(music: &Bound<'_, PyDict>) -> PyResult<Verb> {
    known_keys(music, "music", &MUSIC)?;
    let Some(effect) = music.get_item("effect")? else {
        return Err(value_error("a music verb carries `effect`"));
    };
    let optional = |key: &str| -> PyResult<Option<Bound<'_, PyAny>>> {
        Ok(music.get_item(key)?.filter(|value| !value.is_none()))
    };
    let sensitivity = optional("sensitivity")?
        .map(|v| v.extract::<i64>())
        .transpose()?;
    let soft = optional("soft")?.map(|v| v.extract::<bool>()).transpose()?;
    let color = optional("color")?;
    Ok(Verb::Music(conv::music(
        effect.extract()?,
        sensitivity,
        soft,
        color.as_ref(),
    )?))
}
