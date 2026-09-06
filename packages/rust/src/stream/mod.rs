//! Streaming colors to a device's zones over the raw segment channel.
//!
//! The channel is armed once, then fed frames — `docs/protocol/lan.md` 2.3. Two
//! facts about it shape everything here:
//!
//! - **It never answers.** Nothing acknowledges a frame, and a malformed one is
//!   dropped in silence. So the stream verifies nothing and asks for nothing
//!   back; it sends, and the caller looks at the light.
//! - **It saturates.** Push faster than the firmware drains and the rope
//!   freezes or stutters. The ceiling falls as frames grow, so the rate comes
//!   from the zone count, read off numbers measured on a physical unit and
//!   recorded in its device file — `docs/protocol/lan.md` 2.7.
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

mod resolve;
mod sender;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

use self::resolve::{arg_named, gradient_arg, named, rate_hz, zone_count};
use self::sender::{Shared, send_enable};
use crate::codec::{ArgRole, Role};
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::lan::DeviceId;

/// The rate used when a device file records no measurement, in hertz.
///
/// Below every rate measured so far, on a channel where too fast is a rope that
/// stutters and too slow is only a coarser animation. It is a fallback, not a
/// finding: measure the unit at hand and record it, and the stream will use
/// that instead. Configurable as `lan.stream_fallback_hz`.
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
    /// and cannot be inferred from the SKU.
    Native,
    /// A count the caller picks. The firmware groups LEDs into blocks to serve
    /// it, so asking for more than the unit has refines nothing.
    Exact(u16),
}

/// How fast frames go out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Rate {
    /// From the device file's `measurements.frame_rate`, falling back to
    /// [`FALLBACK_HZ`] when it records none.
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
        // before anything is armed, rather than on the first frame.
        govee.transport(id, mode)?;

        let sku = govee.sku(id)?;
        let device = govee.catalog().device(&sku)?;
        let enable = named(device, mode, Role::SegmentEnable)?;
        let color = named(device, mode, Role::SegmentColor)?;
        let enable_arg = arg_named(device, mode, enable, ArgRole::Enable)?;
        let colors_arg = arg_named(device, mode, color, ArgRole::Colors)?;
        let zones = zone_count(device, options.zones)?;
        let hz = rate_hz(
            device,
            &sku,
            zones,
            options.rate,
            govee.config().lan.stream_fallback_hz,
        );
        if hz <= 0.0 {
            return Err(Error::StreamRateOutOfRange { hz });
        }

        let shared = Arc::new(Shared {
            govee: govee.clone(),
            id: id.clone(),
            mode,
            sku,
            enable: enable.to_owned(),
            enable_arg: enable_arg.to_owned(),
            color: color.to_owned(),
            colors_arg: colors_arg.to_owned(),
            gradient: gradient_arg(device, mode, color, options.gradient),
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

    /// The rate frames go out at, in hertz.
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

    /// Frames written to the socket.
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
                .map_err(|_| Error::Transport(crate::lan::Error::ShutDown))?,
            None => Ok(()),
        }
    }

    fn paint(&self, edit: impl FnOnce(&mut Vec<[u8; 3]>) -> Result<()>) -> Result<()> {
        {
            let mut colors = self
                .shared
                .colors
                .lock()
                .map_err(|_| Error::Transport(crate::lan::Error::ShutDown))?;
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
        // The emitting task sends the disarming frame. Signalling it is all a
        // `Drop` can do: it cannot await that frame, and spawning it here
        // panics when the handle outlives the runtime. `close` is the way to
        // know the channel was disarmed.
        self.shared.stop.notify_one();
    }
}
