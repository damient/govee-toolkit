//! Streaming colors to a device's zones over the raw segment channel.
//!
//! The channel is armed once, then fed frames — `docs/protocol/lan.md` 2.3.
//!
//! - **It never answers.** Nothing acknowledges a frame, and the firmware drops
//!   a malformed one in silence. The stream therefore verifies nothing and asks
//!   for nothing back.
//! - **It saturates.** A rate above what the firmware accepts makes the light
//!   freeze or stutter. That limit falls as frames grow, so the rate comes from
//!   the zone count and from a value measured on a physical unit and recorded
//!   in its device file — `docs/protocol/lan.md` 2.7.
//!
//! What a frame costs depends on how the device file paints zones over the
//! chosen mode. A `segment_color` command carries every zone in one frame. A
//! `segment_color_masked` one carries a single color and the zones that use it,
//! so a repaint costs one write per distinct color: a solid fill is one write,
//! and a picture of fifteen colors is fifteen. That is what `ble` offers. No
//! per-pixel channel sits behind it, so a stream there runs at the zone count
//! the device file declares, not at native resolution. [`Zones::Native`] is
//! refused there: sent as a mask, the firmware drops the high bits in silence.
//!
//! On `ble` the transport paces, not the stream. Every write goes through the
//! same budget, whether or not a stream opened it.
//!
//! Writes never block. The stream holds the current colors and an emitting task
//! sends them on a fixed interval, so a source faster than the device is not
//! throttled: its later frame replaces the earlier one and only the latest is
//! sent. That is what an Art-Net universe or an audio spectrum needs, and it is
//! why [`SegmentStream::frames_superseded`] exists rather than a queue.
//!
//! ```no_run
//! use govee_toolkit::{Args, Config, Govee};
//! use govee_toolkit::stream::{StreamOptions, Zones};
//!
//! # async fn example(govee: Govee, id: govee_toolkit::DeviceId) -> Result<(), govee_toolkit::Error> {
//! let device = govee.device(&id);
//! device.send("power", &Args::new().int("on", 1)).await?;
//!
//! let stream = device
//!     .open_stream(StreamOptions {
//!         zones: Zones::Native,
//!         ..StreamOptions::default()
//!     })
//!     .await?;
//!
//! stream.fill([255, 0, 0])?;
//! stream.set_zone(0, [0, 0, 255])?;
//! stream.close().await?;
//! # Ok(())
//! # }
//! ```

mod paint;
mod resolve;
mod sender;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

use self::resolve::{plan, rate_hz};
use self::sender::{Shared, send_enable};
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::transport::DeviceId;

/// The rate used when a device file records no measurement for the mode a
/// stream opens on, in hertz.
///
/// Below every rate measured so far: too fast stutters, too slow is only a
/// coarser animation. It is a fallback and not a measurement, and it is the
/// same number for every mode. A `ble` stream runs at it, because no device
/// file records a rate measured over that mode. Measure the unit, record the
/// value in its device file, and the stream uses that instead. Configurable as
/// `stream.fallback_hz`.
pub const FALLBACK_HZ: f64 = 10.0;

/// How many zones a stream carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Zones {
    /// What the Govee app exposes, from `capabilities.segments.count`.
    #[default]
    App,
    /// Every addressable LED, from `capabilities.segments.native_pixels`.
    ///
    /// Fails when nobody measured it: that number belongs to the physical unit
    /// and cannot be inferred from the SKU. Fails too on a mode that paints by
    /// zone mask, which addresses zones and reaches no pixel behind them.
    Native,
    /// A count the caller picks. The firmware groups LEDs into blocks to serve
    /// it, so asking for more than the unit has refines nothing.
    Exact(u16),
}

/// How fast frames go out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Rate {
    /// From the device file's `measurements.frame_rate` for the mode the
    /// stream opens on, falling back to [`FALLBACK_HZ`] when it records none
    /// there. A rate measured over one mode is never carried to another.
    #[default]
    Measured,
    /// A rate the caller picks, in hertz.
    Fixed(f64),
}

/// How a stream is opened.
#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    /// How many zones to carry.
    pub zones: Zones,
    /// How fast to send.
    pub rate: Rate,
    /// Ask the firmware to interpolate between zones, and wrap from the last
    /// back to the first.
    ///
    /// `false`, the default, gives hard-edged zones — what a caller painting a
    /// pattern wants, and the only sensible choice at native resolution, where
    /// interpolation blurs exactly the detail that resolution buys. Which
    /// default a model itself uses is a per-SKU fact no device file records.
    pub gradient: bool,
}

/// An open segment channel.
///
/// It stays armed until [`SegmentStream::close`]. Dropping it asks the emitting
/// task to disarm instead, which reports no failure and does nothing at all if
/// the runtime is already gone.
#[derive(Debug)]
pub struct SegmentStream {
    shared: Arc<Shared>,
    /// Taken by `close`, which awaits the disarm the task sends.
    task: Option<tokio::task::JoinHandle<Result<()>>>,
}

impl SegmentStream {
    /// Arm the channel and start emitting. See
    /// [`DeviceHandle::open_stream`](crate::DeviceHandle::open_stream) for what
    /// this can fail with.
    pub(crate) async fn open(govee: &Govee, id: &DeviceId, options: StreamOptions) -> Result<Self> {
        // Chosen once, then carried: a mode is picked from recorded state, and
        // re-picking it per frame would put that decision on the fast path.
        let mode = govee.choose(id)?;
        // Resolved here so that a mode with no transport in this build fails
        // before anything is armed, rather than on the first frame, and so
        // that no frame pays for the lookup.
        let transport = Arc::clone(govee.transport(id, mode)?);

        let sku = govee.sku(id)?;
        let device = govee.catalog().device(&sku)?;
        let plan = plan(device, mode, &options)?;
        let zones = plan.zones;
        let hz = rate_hz(
            device,
            &sku,
            mode,
            zones,
            options.rate,
            govee.config().stream.fallback_hz,
        );
        if hz <= 0.0 {
            return Err(Error::StreamRateOutOfRange { hz });
        }

        let shared = Arc::new(Shared {
            govee: govee.clone(),
            id: id.clone(),
            mode,
            sku,
            transport,
            enable: plan.enable,
            gradient: plan.gradient,
            painter: plan.painter,
            hz,
            zones,
            colors: Mutex::new(vec![[0, 0, 0]; zones]),
            generation: AtomicU64::new(0),
            emitted: AtomicU64::new(0),
            sent: AtomicU64::new(0),
            superseded: AtomicU64::new(0),
            failure: Mutex::new(None),
            stop: Notify::new(),
        });

        // Before arming, so the first frame is painted under the setting the
        // caller asked for.
        sender::send_gradient(&shared).await?;
        send_enable(&shared, 1).await?;
        let task = tokio::spawn(sender::run(Arc::clone(&shared)));
        Ok(Self {
            shared,
            task: Some(task),
        })
    }

    /// How many zones every frame carries. Fixed for the life of the stream.
    #[must_use]
    pub fn zones(&self) -> usize {
        self.shared.zones
    }

    /// How often the stream repaints, in hertz.
    ///
    /// A repaint is one frame where the device file paints every zone at once,
    /// and one frame per distinct color where it paints by mask — so frames
    /// leave at this rate times the number of colors the picture holds.
    #[must_use]
    pub fn rate_hz(&self) -> f64 {
        self.shared.hz
    }

    /// Replace every zone.
    ///
    /// # Errors
    ///
    /// [`Error::ZoneCountMismatch`] if `colors` is not [`SegmentStream::zones`]
    /// long. The firmware reads the count off the frame and re-groups the LEDs
    /// around it, so a stream carries one count throughout.
    pub fn set_all(&self, colors: &[[u8; 3]]) -> Result<()> {
        if colors.len() != self.shared.zones {
            return Err(Error::ZoneCountMismatch {
                expected: self.shared.zones,
                got: colors.len(),
            });
        }
        self.paint(|current| {
            current.copy_from_slice(colors);
            Ok(())
        })
    }

    /// Repaint one zone, zero-based, leaving the others as they are.
    ///
    /// # Errors
    ///
    /// [`Error::ZoneOutOfRange`] if `index` is past the last zone.
    pub fn set_zone(&self, index: usize, color: [u8; 3]) -> Result<()> {
        if index >= self.shared.zones {
            return Err(Error::ZoneOutOfRange {
                index,
                zones: self.shared.zones,
            });
        }
        self.paint(|current| {
            if let Some(zone) = current.get_mut(index) {
                *zone = color;
            }
            Ok(())
        })
    }

    /// Paint every zone one color.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the stream has shut down.
    pub fn fill(&self, color: [u8; 3]) -> Result<()> {
        self.paint(|current| {
            current.fill(color);
            Ok(())
        })
    }

    /// Paint every zone black.
    ///
    /// The channel stays armed and frames keep going out — this is not
    /// [`SegmentStream::close`], and not the same as powering the device off.
    ///
    /// # Errors
    ///
    /// As for [`SegmentStream::fill`].
    pub fn clear(&self) -> Result<()> {
        self.fill([0, 0, 0])
    }

    /// The colors as they stand.
    #[must_use]
    pub fn buffer(&self) -> Vec<[u8; 3]> {
        self.shared
            .colors
            .lock()
            .map(|colors| colors.clone())
            .unwrap_or_default()
    }

    /// Frames handed to the transport, which is one per write rather than one
    /// per repaint: a masked painter writes once per distinct color.
    #[must_use]
    pub fn frames_sent(&self) -> u64 {
        self.shared.sent.load(Ordering::Relaxed)
    }

    /// Writes replaced by a later one before a frame carried them.
    ///
    /// Expected, not an error: it is what a source faster than the device
    /// costs, and the alternative is throttling that source.
    #[must_use]
    pub fn frames_superseded(&self) -> u64 {
        self.shared.superseded.load(Ordering::Relaxed)
    }

    /// What stopped the stream, if anything did.
    ///
    /// Only an encoding failure stops it: the arguments cannot become valid, so
    /// retrying would send nothing forever, and the channel is disarmed there
    /// and then. A transport failure does not — the breaker already refuses a
    /// device that is down, cheaply, and a stream outlives a device that comes
    /// back.
    #[must_use]
    pub fn error(&self) -> Option<Arc<Error>> {
        self.shared.failure.lock().ok().and_then(|e| e.clone())
    }

    /// Stop emitting and disarm the channel.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the disarming frame cannot be sent, or if the
    /// emitting task is gone and never sent it. Emitting has stopped either
    /// way.
    pub async fn close(mut self) -> Result<()> {
        self.shared.stop.notify_one();
        match self.task.take() {
            Some(task) => task
                .await
                .map_err(|_| Error::Transport(crate::transport::Error::ShutDown))?,
            None => Ok(()),
        }
    }

    fn paint(&self, edit: impl FnOnce(&mut Vec<[u8; 3]>) -> Result<()>) -> Result<()> {
        {
            let mut colors = self
                .shared
                .colors
                .lock()
                .map_err(|_| Error::Transport(crate::transport::Error::ShutDown))?;
            edit(&mut colors)?;
        }
        // A write onto a generation no frame has carried yet replaces one that
        // was never sent.
        if self.shared.generation.fetch_add(1, Ordering::Release)
            > self.shared.emitted.load(Ordering::Acquire)
        {
            self.shared.superseded.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

impl Drop for SegmentStream {
    fn drop(&mut self) {
        // A `Drop` cannot await the disarming frame, and spawning one here
        // panics when the handle outlives the runtime. The emitting task sends
        // it; only `close` reports whether it went out.
        self.shared.stop.notify_one();
    }
}
