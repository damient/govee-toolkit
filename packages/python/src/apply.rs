//! What `DeviceHandle.invoke()` and `Devices.apply()` answer.

use govee_toolkit::codec::Mode;
use govee_toolkit::{Applied as CoreApplied, Invoked as CoreInvoked, Outcome as CoreOutcome};
use pyo3::prelude::*;

use crate::conv::to_py;
use crate::devices::{Outcome, outcomes};

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

impl Invoked {
    pub(crate) fn new(mode: Mode, command: String, invoked: CoreInvoked) -> Self {
        let (id, fields) = match invoked {
            CoreInvoked::Sent(served) => (served.id.to_string(), None),
            CoreInvoked::Read(reply) => (reply.id.to_string(), Some(reply.fields.to_json())),
        };
        Self {
            id,
            mode: mode.to_string(),
            command,
            fields,
        }
    }
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
    verdict: Option<String>,
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
        self.verdict.is_none()
    }

    /// `"N of M devices failed: ids"`, or the error's own message for a set
    /// of one member. `None` where every member took every step.
    #[getter]
    fn verdict(&self) -> Option<String> {
        self.verdict.clone()
    }

    fn __repr__(&self) -> String {
        format!("Applied(steps={}, ok={})", self.steps.len(), self.ok())
    }
}

fn shared<T>(py: Python<'_>, outcomes: &[CoreOutcome<T>]) -> PyResult<Vec<Py<Outcome>>>
where
    T: govee_toolkit::Answer,
{
    self::outcomes(outcomes)
        .into_iter()
        .map(|outcome| Py::new(py, outcome))
        .collect()
}

impl Applied {
    pub(crate) fn new(applied: &CoreApplied) -> PyResult<Self> {
        let verdict = applied.verdict();
        Python::attach(|py| {
            let steps = applied
                .steps
                .iter()
                .map(|step| {
                    let outcomes = shared(py, &step.outcomes)?;
                    Py::new(
                        py,
                        AppliedStep {
                            step: step.name,
                            outcomes,
                        },
                    )
                })
                .collect::<PyResult<_>>()?;
            Ok(Self {
                reached: shared(py, &applied.reached)?,
                steps,
                verdict,
            })
        })
    }
}
