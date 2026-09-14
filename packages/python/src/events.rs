//! What the SDK reports while it runs, as one asynchronous iterator.

use std::sync::Arc;

use govee_toolkit::transport::{Change, Event as TransportEvent};
use govee_toolkit::{DeviceStatus as CoreStatus, Event, Govee};
use pyo3::exceptions::PyStopAsyncIteration;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{Mutex, watch};

use crate::conv::mode_name;
use crate::types::DeviceStatus;

/// The events of one SDK. Iterate it with `async for`.
///
/// Every event is a dict, and `type` says which one it is. A subscription
/// that falls behind reports `{"type": "lagged", "missed": n}` rather than
/// hide the gap.
#[pyclass(frozen, module = "govee_toolkit", name = "EventStream")]
#[derive(Debug)]
pub(crate) struct EventStream {
    events: Arc<Mutex<Receiver<Event>>>,
}

impl EventStream {
    /// Subscribe to what an SDK reports.
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
            let received = events.lock().await.recv().await;
            match received {
                Ok(event) => Python::attach(|py| describe(py, &event)),
                Err(RecvError::Lagged(missed)) => Python::attach(|py| {
                    let dict = PyDict::new(py);
                    dict.set_item("type", "lagged")?;
                    dict.set_item("missed", missed)?;
                    Ok(dict.unbind().into_any())
                }),
                Err(RecvError::Closed) => Err(PyStopAsyncIteration::new_err(
                    "the SDK that reported these events is gone",
                )),
            }
        })
    }
}

/// One event, as the dict Python reads.
fn describe(py: Python<'_>, event: &Event) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    match event {
        Event::UnknownSku { id, sku } => {
            dict.set_item("type", "unknown_sku")?;
            dict.set_item("id", id.to_string())?;
            dict.set_item("sku", sku)?;
        }
        Event::Transport(TransportEvent::Discovered {
            mode,
            device,
            change,
        }) => {
            dict.set_item("type", "discovered")?;
            dict.set_item("mode", mode_name(*mode))?;
            dict.set_item("id", device.id.to_string())?;
            dict.set_item("sku", &device.sku)?;
            dict.set_item("endpoint", &device.endpoint)?;
            dict.set_item("firmware", device.firmware.clone())?;
            dict.set_item("change", change_name(*change))?;
        }
        Event::Transport(TransportEvent::Forgotten { mode, id }) => {
            dict.set_item("type", "forgotten")?;
            dict.set_item("mode", mode_name(*mode))?;
            dict.set_item("id", id.to_string())?;
        }
        Event::Transport(TransportEvent::Sent(sent)) => {
            dict.set_item("type", "sent")?;
            dict.set_item("mode", mode_name(sent.mode))?;
            dict.set_item("id", sent.id.to_string())?;
            dict.set_item("cmd", &sent.cmd)?;
            dict.set_item("endpoint", &sent.endpoint)?;
        }
        Event::Transport(TransportEvent::Status { mode, status }) => {
            dict.set_item("type", "status")?;
            dict.set_item("mode", mode_name(*mode))?;
            dict.set_item("id", status.id.to_string())?;
            dict.set_item("status", DeviceStatus::from(status.clone()))?;
        }
        Event::Transport(TransportEvent::HealthChanged {
            id,
            mode,
            transition,
        }) => {
            dict.set_item("type", "health_changed")?;
            dict.set_item("mode", mode_name(*mode))?;
            dict.set_item("id", id.to_string())?;
            dict.set_item("from", transition.from.to_string())?;
            dict.set_item("to", transition.to.to_string())?;
        }
        // A variant this build does not name still reaches the caller.
        other => {
            dict.set_item("type", "other")?;
            dict.set_item("detail", format!("{other:?}"))?;
        }
    }
    Ok(dict.unbind().into_any())
}

/// What a discovery changed about what was already known.
fn change_name(change: Change) -> &'static str {
    match change {
        Change::New => "new",
        Change::Refreshed => "refreshed",
        Change::Moved => "moved",
        Change::FirmwareChanged => "firmware_changed",
    }
}

/// Add the event streams to the module.
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
    /// Watch what one transport hears from one device.
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
