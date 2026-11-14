//! The raw segment channel, armed until it closes.

use std::sync::Arc;

use govee_toolkit::SegmentStream as CoreStream;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv::{colors, rgb};
use crate::errors::map;

/// An open segment channel. The writers never block: each one replaces what
/// the next frame carries.
///
/// Close it, or leave it with `async with`. A stream that is dropped disarms
/// the channel as well, and reports no failure.
#[pyclass(frozen, module = "govee_toolkit", name = "SegmentStream")]
#[derive(Debug)]
pub(crate) struct SegmentStream {
    inner: Arc<CoreStream>,
}

impl SegmentStream {
    pub(crate) fn new(inner: Arc<CoreStream>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl SegmentStream {
    /// How many zones the frames carry. The firmware reads the count off the
    /// frame, so it never changes while the stream is open.
    #[getter]
    fn zones(&self) -> usize {
        self.inner.zones()
    }

    /// How fast frames go out, in hertz.
    #[getter]
    fn rate_hz(&self) -> f64 {
        self.inner.rate_hz()
    }

    /// How many frames reached the wire.
    #[getter]
    fn frames_sent(&self) -> u64 {
        self.inner.frames_sent()
    }

    /// How many frames a later write replaced before they left.
    #[getter]
    fn frames_superseded(&self) -> u64 {
        self.inner.frames_superseded()
    }

    /// What the emitting task failed with, if it failed. The stream stops
    /// sending, and the writers keep answering.
    #[getter]
    fn error(&self) -> Option<String> {
        self.inner.error().map(|e| e.to_string())
    }

    /// State every zone. The count must be the stream's own. `bytes` carries
    /// three bytes for every zone.
    fn set_all(&self, colors: &Bound<'_, PyAny>) -> PyResult<()> {
        let values = self::colors(colors)?;
        map(self.inner.set_all(&values))
    }

    /// State one zone, by its zero-based index.
    fn set_zone(&self, index: usize, color: &Bound<'_, PyAny>) -> PyResult<()> {
        let color = rgb(color)?;
        map(self.inner.set_zone(index, color))
    }

    /// Put one color in every zone.
    fn fill(&self, color: &Bound<'_, PyAny>) -> PyResult<()> {
        let color = rgb(color)?;
        map(self.inner.fill(color))
    }

    /// Put black in every zone. The channel stays armed.
    fn clear(&self) -> PyResult<()> {
        map(self.inner.clear())
    }

    /// What the next frame carries.
    fn buffer(&self) -> Vec<(u8, u8, u8)> {
        self.inner
            .buffer()
            .into_iter()
            .map(|[r, g, b]| (r, g, b))
            .collect()
    }

    /// Disarm the channel and wait for the last frame to leave. A second
    /// call does nothing. A writer called after it raises `TransportError`
    /// with the code `stream_closed`.
    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let stream = Arc::clone(&self.inner);
        future_into_py(py, async move {
            map(stream.close().await)?;
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
            self.inner.zones(),
            self.inner.rate_hz()
        )
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<SegmentStream>()
}
