//! One device, and everything a caller can ask of it.
//!
//! Every call goes over a mode the user enabled. Nothing falls back to another
//! one: a device no enabled mode reaches raises, and says so.

use std::future::Future;

use govee_toolkit::codec::Mode;
use govee_toolkit::{
    DeviceHandle as CoreHandle, DeviceId, Error, Govee, Identify, Music, Paint,
    Served as CoreServed, StreamOptions, WifiCredentials, describe,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv::{self, to_py};
use crate::errors::map;
use crate::events::StatusStream;
use crate::stream::SegmentStream;
use crate::types::{DeviceStatus, Health, Reply, Served};

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
    pub(crate) govee: Govee,
    /// The one mode every call on this handle goes over, where the caller
    /// named one.
    pub(crate) pinned: Option<Mode>,
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
        self.core()
            .modes()
            .iter()
            .map(ToString::to_string)
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
        Ok(map(self.core().serving_mode())?.to_string())
    }

    /// What `devices/<SKU>.yaml` declares for it. Reads no hardware.
    fn spec(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, map(self.core().spec())?)
    }

    /// What `devices/<SKU>.yaml` declares, as the record `govee describe`
    /// prints: the modes in one place, the commands under the mode that
    /// carries them, and each argument's type, role and bound.
    fn describe(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &describe(map(self.core().spec())?))
    }

    /// The last status heard, without asking for a new one.
    fn last_status(&self) -> Option<DeviceStatus> {
        self.core().last_status().map(DeviceStatus::from)
    }

    /// Watch its status as answers arrive, over the mode that would serve a
    /// command now. `None` when no enabled mode can, or when that transport
    /// has heard nothing.
    fn watch_status(&self) -> Option<StatusStream> {
        self.core().watch_status().map(StatusStream::new)
    }

    /// Scan for the device if no mode knows it yet, then answer the mode a
    /// command would go over.
    ///
    /// The scan covers every enabled mode, whatever this handle is pinned to.
    fn ensure_known<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, _, id) = self.parts();
        future_into_py(py, async move {
            Ok(map(govee.ensure_known(&id).await)?.to_string())
        })
    }

    /// Send a command, named as the device file names it.
    ///
    /// The arguments are the ones the entry declares, and each value is read
    /// under the type the entry declares for it. A value outside the declared
    /// range raises `CodecError`, and nothing is sent.
    #[pyo3(signature = (command, **args))]
    fn send<'py>(
        &self,
        py: Python<'py>,
        command: String,
        args: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let supplied = conv::args(args)?;
        self.served(py, |govee, pinned, id| async move {
            let handle = govee.device_maybe_on(&id, pinned);
            let call = handle.resolve()?;
            let values = call.args(&command, supplied)?;
            call.send(&command, &values).await
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
        let supplied = conv::args(args)?;
        let (govee, pinned, id) = self.parts();
        future_into_py(py, async move {
            let handle = govee.device_maybe_on(&id, pinned);
            let call = map(handle.resolve())?;
            let values = map(call.args(&command, supplied))?;
            let reply = map(call.read(&command, &values).await)?;
            Ok(Reply::from(reply))
        })
    }

    /// Ask the device for its state and wait for the answer.
    fn status<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let (govee, pinned, id) = self.parts();
        future_into_py(py, async move {
            let status = map(govee.device_maybe_on(&id, pinned).status().await)?;
            Ok(DeviceStatus::from(status))
        })
    }

    /// Turn the device on or off.
    fn power<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).power(on).await
        })
    }

    /// Set the level, in the unit the device file declares. A level outside
    /// that range is an error, never a clamp.
    fn brightness<'py>(&self, py: Python<'py>, level: i64) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).brightness(level).await
        })
    }

    /// Set one color, as three channels.
    fn color<'py>(&self, py: Python<'py>, rgb: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let rgb = conv::rgb(rgb)?;
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).color(rgb).await
        })
    }

    /// Power the device on and paint one color, so a person sees which
    /// fixture this identity drives.
    ///
    /// The look the device held is lost. To walk a rig, power every device
    /// off, wait a second, and then call this on one device at a time.
    ///
    /// `None` takes the core's defaults: green, and the top of the
    /// brightness range the device file declares.
    #[pyo3(signature = (color=None, full_brightness=None))]
    fn identify<'py>(
        &self,
        py: Python<'py>,
        color: Option<&Bound<'py, PyAny>>,
        full_brightness: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let default = Identify::default();
        let options = Identify {
            color: color.map(conv::rgb).transpose()?.unwrap_or(default.color),
            full_brightness: full_brightness.unwrap_or(default.full_brightness),
        };
        let (govee, pinned, id) = self.parts();
        future_into_py(py, async move {
            map(govee.device_maybe_on(&id, pinned).identify(&options).await)?;
            Ok(())
        })
    }

    /// Set the white temperature, in kelvin. It ends color mode.
    fn color_temp<'py>(&self, py: Python<'py>, kelvin: i64) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).color_temp(kelvin).await
        })
    }

    /// Play an effect the device renders from its own microphone.
    ///
    /// The identifiers are the mode's own: one the entry accepts is not one
    /// the device renders. `color` imposes a color, and `None` leaves the
    /// colors to the firmware.
    ///
    /// `None` takes the core's default for `sensitivity` and for `soft`.
    #[pyo3(signature = (effect, sensitivity=None, soft=None, color=None))]
    fn music<'py>(
        &self,
        py: Python<'py>,
        effect: i64,
        sensitivity: Option<i64>,
        soft: Option<bool>,
        color: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let color = color.map(conv::rgb).transpose()?;
        let default = Music::default();
        let music = Music {
            effect,
            sensitivity: sensitivity.unwrap_or(default.sensitivity),
            soft: soft.unwrap_or(default.soft),
            color,
        };
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).music(&music).await
        })
    }

    /// Paint the segments once.
    ///
    /// One color fills every zone, and a list states them all. A zone list
    /// takes one color. `resolution` takes `"app"` when it is `None`.
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
        self.served(py, |govee, pinned, id| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            govee.device_maybe_on(&id, pinned).segment(&paint).await
        })
    }

    /// Ask the firmware to interpolate between zones, and to wrap from the
    /// last zone back to the first.
    fn gradient<'py>(&self, py: Python<'py>, on: bool) -> PyResult<Bound<'py, PyAny>> {
        self.served(py, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).gradient(on).await
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
        let (govee, pinned, id) = self.parts();
        future_into_py(py, async move {
            let done = map(govee
                .device_maybe_on(&id, pinned)
                .provision_wifi(&credentials)
                .await)?;
            Ok(done.as_str())
        })
    }

    /// Open the raw segment channel and paint it frame by frame.
    ///
    /// Power the device on first: arming a dark strip paints nothing. The
    /// channel holds the colors only while it is armed, and the device goes
    /// back to the color it showed before once the stream closes.
    ///
    /// `resolution` takes `"app"` when it is `None`, and `rate` takes
    /// `"measured"`.
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
        let (govee, pinned, id) = self.parts();
        future_into_py(py, async move {
            let stream = map(govee
                .device_maybe_on(&id, pinned)
                .open_stream(options)
                .await)?;
            Ok(SegmentStream::new(stream))
        })
    }

    fn __repr__(&self) -> String {
        format!("DeviceHandle(id='{}')", self.id)
    }
}

impl DeviceHandle {
    fn core(&self) -> CoreHandle<'_> {
        self.govee.device_maybe_on(&self.id, self.pinned)
    }

    fn parts(&self) -> (Govee, Option<Mode>, DeviceId) {
        (self.govee.clone(), self.pinned, self.id.clone())
    }

    fn served<'py, Fut>(
        &self,
        py: Python<'py>,
        verb: impl FnOnce(Govee, Option<Mode>, DeviceId) -> Fut,
    ) -> PyResult<Bound<'py, PyAny>>
    where
        Fut: Future<Output = Result<CoreServed, Error>> + Send + 'static,
    {
        let (govee, pinned, id) = self.parts();
        let call = verb(govee, pinned, id);
        future_into_py(py, async move { Ok(Served::from(map(call.await)?)) })
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DeviceHandle>()
}
