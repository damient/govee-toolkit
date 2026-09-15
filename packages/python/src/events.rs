//! What the SDK reports while it runs, as one asynchronous iterator.

use std::sync::Arc;

use govee_toolkit::{DeviceStatus as CoreStatus, Event, Govee};
use pyo3::exceptions::PyStopAsyncIteration;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{Mutex, watch};

use crate::conv::to_py;
use crate::types::DeviceStatus;

/// The events of one SDK. Iterate it with `async for`.
///
/// Every event is a dict, and `event` says which one it is. The records are
/// the core's own, so `govee watch --json` prints the same ones. A
/// subscription that falls behind reports `{"event": "lagged", "missed": n}`
/// rather than hide the gap.
#[pyclass(frozen, module = "govee_toolkit", name = "EventStream")]
#[derive(Debug)]
pub(crate) struct EventStream {
    events: Arc<Mutex<Receiver<Event>>>,
}

impl EventStream {
    pub(crate) fn new(govee: &Govee) -> Self {
        Self {
            events: Arc::new(Mutex::new(govee.events())),
        }
    }
}

#[pymethods]
impl EventStream {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let events = Arc::clone(&self.events);
        future_into_py(py, async move {
            let record = match events.lock().await.recv().await {
                Ok(event) => event.to_json(),
                Err(RecvError::Lagged(missed)) => Event::lagged(missed),
                Err(RecvError::Closed) => {
                    return Err(PyStopAsyncIteration::new_err(
                        "the SDK that reported these events is gone",
                    ));
                }
            };
            Python::attach(|py| to_py(py, &record))
        })
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<EventStream>()?;
    module.add_class::<StatusStream>()
}

/// One device's status, as answers arrive. Iterate it with `async for`.
///
/// It requests nothing: it reports the answers a status request or a
/// verification already brought back.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "StatusStream"
)]
#[derive(Debug)]
pub(crate) struct StatusStream {
    statuses: Arc<Mutex<watch::Receiver<Option<CoreStatus>>>>,
}

impl StatusStream {
    pub(crate) fn new(statuses: watch::Receiver<Option<CoreStatus>>) -> Self {
        Self {
            statuses: Arc::new(Mutex::new(statuses)),
        }
    }
}

#[pymethods]
impl StatusStream {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let statuses = Arc::clone(&self.statuses);
        future_into_py(py, async move {
            let mut guard = statuses.lock().await;
            loop {
                guard.changed().await.map_err(|_| {
                    PyStopAsyncIteration::new_err("the transport that heard this device is gone")
                })?;
                let latest = guard.borrow_and_update().clone();
                if let Some(status) = latest {
                    return Ok(DeviceStatus::from(status));
                }
            }
        })
    }
}
