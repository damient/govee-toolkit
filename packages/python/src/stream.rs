//! The raw segment channel, armed until it closes.

use std::sync::Mutex;

use govee_toolkit::SegmentStream as CoreStream;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv::{colors, rgb};
use crate::errors::{map, value_error};

/// An open segment channel. The writers never block: each one replaces what
/// the next frame carries.
///
/// Close it, or leave it with `async with`. A stream that is dropped disarms
/// the channel as well, and reports no failure.
#[pyclass(frozen, module = "govee_toolkit", name = "SegmentStream")]
#[derive(Debug)]
pub(crate) struct SegmentStream {
    /// Taken by `close`, which awaits the disarm.
    inner: Mutex<Option<CoreStream>>,
    zones: usize,
    rate_hz: f64,
}

impl SegmentStream {
    /// Wrap a channel the core armed.
    pub(crate) fn new(stream: CoreStream) -> Self {
        let zones = stream.zones();
        let rate_hz = stream.rate_hz();
        Self {
            inner: Mutex::new(Some(stream)),
            zones,
            rate_hz,
        }
    }

    /// Run one writer against the open channel.
    fn with<T>(
        &self,
        call: impl FnOnce(&CoreStream) -> Result<T, govee_toolkit::Error>,
    ) -> PyResult<T> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| value_error("this stream failed while another call held it"))?;
        let stream = guard
            .as_ref()
            .ok_or_else(|| value_error("this stream is closed"))?;
        map(call(stream))
    }
}

#[pymethods]
impl SegmentStream {
    /// How many zones the frames carry. The firmware reads the count off the
    /// frame, so it never changes while the stream is open.
    #[getter]
    fn zones(&self) -> usize {
        self.zones
    }

    /// How fast frames go out, in hertz.
    #[getter]
    fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    /// How many frames left.
    #[getter]
    fn frames_sent(&self) -> PyResult<u64> {
        self.with(|stream| Ok(stream.frames_sent()))
    }

    /// How many frames a later write replaced before they left.
    #[getter]
    fn frames_superseded(&self) -> PyResult<u64> {
        self.with(|stream| Ok(stream.frames_superseded()))
    }

    /// What the emitting task failed with, if it failed. The stream stops
    /// sending, and the writers keep answering.
    #[getter]
    fn error(&self) -> PyResult<Option<String>> {
        self.with(|stream| Ok(stream.error().map(|e| e.to_string())))
    }

    /// State every zone. The count must be the stream's own.
    fn set_all(&self, colors: &Bound<'_, PyAny>) -> PyResult<()> {
        let values = self::colors(colors)?;
        self.with(|stream| stream.set_all(&values))
    }

    /// State one zone, by its zero-based index.
    fn set_zone(&self, index: usize, color: &Bound<'_, PyAny>) -> PyResult<()> {
        let color = rgb(color)?;
        self.with(|stream| stream.set_zone(index, color))
    }

    /// Put one color in every zone.
    fn fill(&self, color: &Bound<'_, PyAny>) -> PyResult<()> {
        let color = rgb(color)?;
        self.with(|stream| stream.fill(color))
    }

    /// Put black in every zone. The channel stays armed.
    fn clear(&self) -> PyResult<()> {
        self.with(govee_toolkit::SegmentStream::clear)
    }

    /// What the next frame carries.
    fn buffer(&self) -> PyResult<Vec<(u8, u8, u8)>> {
        self.with(|stream| Ok(stream.buffer()))
            .map(|colors| colors.into_iter().map(|[r, g, b]| (r, g, b)).collect())
    }

    /// Disarm the channel and wait for the last frame to leave.
    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let taken = self
            .inner
            .lock()
            .map_err(|_| value_error("this stream failed while another call held it"))?
            .take();
        future_into_py(py, async move {
            if let Some(stream) = taken {
                map(stream.close().await)?;
            }
            Ok(())
        })
    }

    fn __aenter__<'py>(slf: Bound<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handle = slf.unbind();
        future_into_py(py, async move { Ok(handle) })
    }

    #[pyo3(signature = (*_args))]
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _args: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.close(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "SegmentStream(zones={}, rate_hz={})",
            self.zones, self.rate_hz
        )
    }
}

/// Add the stream to the module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<SegmentStream>()
}
