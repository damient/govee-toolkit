//! One device, and everything a caller can ask of it.
//!
//! Every call goes over a mode the user enabled. Nothing falls back to another
//! one: a device no enabled mode reaches raises, and says so.

use govee_toolkit::{DeviceId, Govee as CoreGovee, Music, StreamOptions, WifiCredentials};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv::{self, mode_name, to_py};
use crate::errors::map;
use crate::events::StatusStream;
use crate::stream::SegmentStream;
use crate::types::{DeviceStatus, Health, Reply, Served};
use crate::verbs;

/// A handle on one identity. It holds no state of its own: every answer comes
/// from the SDK it was made by.
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "govee_toolkit",
    name = "DeviceHandle"
)]
#[derive(Debug, Clone)]
pub(crate) struct DeviceHandle {
    pub(crate) govee: CoreGovee,
    pub(crate) id: DeviceId,
}

#[pymethods]
impl DeviceHandle {
    /// The MAC the device reports, uppercased.
    #[getter]
    fn id(&self) -> String {
        self.id.to_string()
    }

    /// The modes enabled for it, in preference order.
    #[getter]
    fn modes(&self) -> Vec<String> {
        self.govee
            .device(&self.id)
            .modes()
            .iter()
            .map(|mode| mode_name(*mode).to_owned())
            .collect()
    }

    /// Its health in one mode. `None` when the transport that serves that
    /// mode has never heard from it.
    fn health(&self, mode: &str) -> PyResult<Option<Health>> {
        Ok(self
            .govee
            .device(&self.id)
            .health(conv::mode(mode)?)
            .map(Health::from))
    }

    /// The mode a command sent now would go over. Read from recorded state,
    /// so the answer can change before the next call.
    fn serving_mode(&self) -> PyResult<String> {
        Ok(mode_name(map(self.govee.device(&self.id).serving_mode())?).to_owned())
    }

    /// What `devices/<SKU>.yaml` declares for it. Reads no hardware.
    fn spec(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, map(self.govee.device(&self.id).spec())?)
    }

    /// The last status heard, without asking for a new one.
    fn last_status(&self) -> Option<DeviceStatus> {
        self.govee
            .device(&self.id)
            .last_status()
            .map(DeviceStatus::from)
    }

    /// Watch its status as answers arrive, over the mode that would serve a
    /// command now. `None` when no enabled mode can, or when that transport
    /// has heard nothing.
    fn watch_status(&self) -> Option<StatusStream> {
        self.govee
            .device(&self.id)
            .watch_status()
            .map(StatusStream::new)
    }

    /// Scan for the device if no mode knows it yet, then answer the mode a
    /// command would go over.
    fn ensure_known<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            let mode = map(verbs::ensure_known(govee, id).await)?;
            Ok(mode_name(mode).to_owned())
        })
    }

    /// Send a command, named as the device file names it.
    ///
    /// The arguments are the ones the entry declares. A value outside the
    /// declared range raises `CodecError`, and nothing is sent.
    #[pyo3(signature = (command, **args))]
    fn send<'py>(
        &self,
        py: Python<'py>,
        command: String,
        args: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let values = conv::args(args)?;
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(
                verbs::send(govee, id, command, values).await
            )?))
        })
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    #[pyo3(signature = (command, **args))]
    fn read<'py>(
        &self,
        py: Python<'py>,
        command: String,
        args: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let values = conv::args(args)?;
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Reply::from(map(
                verbs::read(govee, id, command, values).await
            )?))
        })
    }

    /// Ask the device for its state and wait for the answer.
    fn status<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(DeviceStatus::from(map(verbs::status(govee, id).await)?))
        })
    }

    /// Turn the device on or off.
    fn power<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(verbs::power(govee, id, on).await)?))
        })
    }

    /// Set the level, in the unit the device file declares.
    fn brightness<'py>(&self, py: Python<'py>, level: i64) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(
                map(verbs::brightness(govee, id, level).await)?,
            ))
        })
    }

    /// Set one color, as three channels.
    fn color<'py>(&self, py: Python<'py>, rgb: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let rgb = conv::rgb(rgb)?;
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(verbs::color(govee, id, rgb).await)?))
        })
    }

    /// Set the white temperature, in kelvin. It ends color mode.
    fn color_temp<'py>(&self, py: Python<'py>, kelvin: i64) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(
                verbs::color_temp(govee, id, kelvin).await
            )?))
        })
    }

    /// Play an effect the device renders from its own microphone.
    ///
    /// The identifiers are the mode's own: one the entry accepts is not one
    /// the device renders. `color` imposes a color, and `None` leaves the
    /// colors to the firmware.
    #[pyo3(signature = (effect, sensitivity=0, soft=false, color=None))]
    fn music<'py>(
        &self,
        py: Python<'py>,
        effect: i64,
        sensitivity: i64,
        soft: bool,
        color: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let color = color.map(conv::rgb).transpose()?;
        let music = Music {
            effect,
            sensitivity,
            soft,
            color,
        };
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(verbs::music(govee, id, music).await)?))
        })
    }

    /// Paint the segments once.
    ///
    /// One color fills every zone, and a list states them all. A zone list
    /// takes one color.
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
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(verbs::segment(
                govee, id, zones, colors, resolution, gradient,
            )
            .await)?))
        })
    }

    /// Ask the firmware to interpolate between zones, and to wrap from the
    /// last zone back to the first.
    fn gradient<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(Served::from(map(verbs::gradient(govee, id, on).await)?))
        })
    }

    /// Put the device on a Wi-Fi network over `ble`.
    ///
    /// The device must be in Bluetooth range and closed in the phone
    /// controller. The password travels in plaintext: anything in Bluetooth
    /// range during the transfer reads it. The network must be 2.4 GHz.
    ///
    /// Answers `"accepted"` where the device acknowledged the transfer, and
    /// `"sent"` where its device file declares no acknowledgement.
    #[pyo3(signature = (network, password, utc_offset_hours=0, utc_offset_minutes=0))]
    fn provision_wifi<'py>(
        &self,
        py: Python<'py>,
        network: String,
        password: String,
        utc_offset_hours: u8,
        utc_offset_minutes: u8,
    ) -> PyResult<Bound<'py, PyAny>> {
        let credentials = WifiCredentials {
            network,
            password,
            utc_offset_hours,
            utc_offset_minutes,
        };
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            let done = map(verbs::provision_wifi(govee, id, credentials).await)?;
            Ok(verbs::provisioned_name(done))
        })
    }

    /// Open the raw segment channel and paint it frame by frame.
    ///
    /// Power the device on first: arming a dark strip paints nothing. The
    /// channel holds the colors only while it is armed, and the device goes
    /// back to the color it showed before once the stream closes.
    #[pyo3(signature = (resolution=None, rate=None, gradient=false))]
    fn open_stream<'py>(
        &self,
        py: Python<'py>,
        resolution: Option<&Bound<'py, PyAny>>,
        rate: Option<&Bound<'py, PyAny>>,
        gradient: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = StreamOptions {
            resolution: conv::resolution_or_default(resolution)?,
            rate: conv::rate_or_default(rate)?,
            gradient,
        };
        let (govee, id) = self.parts();
        future_into_py(py, async move {
            Ok(SegmentStream::new(map(verbs::open_stream(
                govee, id, options,
            )
            .await)?))
        })
    }

    fn __repr__(&self) -> String {
        format!("DeviceHandle(id='{}')", self.id)
    }
}

impl DeviceHandle {
    /// What an async call takes with it.
    fn parts(&self) -> (CoreGovee, DeviceId) {
        (self.govee.clone(), self.id.clone())
    }
}

/// Add the handle to the module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DeviceHandle>()
}
