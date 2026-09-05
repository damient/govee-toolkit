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

mod sender;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use self::sender::{Shared, send_enable};
use crate::codec::catalog::ArgSpec;
use crate::codec::{Device, Mode, Role};
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
    /// What the Govee app exposes, from `capabilities.segment_count`.
    #[default]
    App,
    /// Every addressable LED, from `capabilities.native_pixels`.
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
/// It stays armed until [`SegmentStream::close`]. Dropping it disarms on a
/// best-effort task instead, which cannot report a failure.
#[derive(Debug)]
pub struct SegmentStream {
    shared: Arc<Shared>,
    task: tokio::task::JoinHandle<()>,
}

impl SegmentStream {
    /// Arm the channel and start emitting. See
    /// [`DeviceHandle::open_stream`](crate::DeviceHandle::open_stream) for what
    /// this can fail with.
    pub(crate) async fn open(govee: &Govee, id: &DeviceId, options: StreamOptions) -> Result<Self> {
        // Chosen once, then carried: a mode is picked from recorded state, and
        // re-picking it per frame would put that decision on the fast path.
        let mode = govee.choose(id)?;
        if mode != Mode::Lan {
            return Err(Error::ModeNotImplemented {
                id: id.clone(),
                mode,
            });
        }

        let sku = govee.sku(id)?;
        let device = govee.catalog().device(&sku)?;
        let enable = named(device, mode, Role::SegmentEnable)?;
        let color = named(device, mode, Role::SegmentColor)?;
        let zones = zone_count(device, options.zones)?;
        let hz = rate_hz(
            device,
            &sku,
            zones,
            options.rate,
            govee.config().lan.stream_fallback_hz,
        );

        let shared = Arc::new(Shared {
            govee: govee.clone(),
            id: id.clone(),
            mode,
            enable: enable.to_owned(),
            color: color.to_owned(),
            gradient: gradient_arg(device, mode, color, options.gradient),
            hz,
            colors: Mutex::new(vec![[0, 0, 0]; zones]),
            generation: AtomicU64::new(0),
            emitted: AtomicU64::new(0),
            sent: AtomicU64::new(0),
            superseded: AtomicU64::new(0),
            failure: Mutex::new(None),
        });

        send_enable(&shared, 1).await?;
        let task = tokio::spawn(sender::run(Arc::clone(&shared)));
        Ok(Self { shared, task })
    }

    /// How many zones every frame carries. Fixed for the life of the stream.
    #[must_use]
    pub fn zones(&self) -> usize {
        self.shared.colors.lock().map_or(0, |colors| colors.len())
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
        self.paint(|current| {
            if current.len() != colors.len() {
                return Err(Error::ZoneCountMismatch {
                    expected: current.len(),
                    got: colors.len(),
                });
            }
            current.copy_from_slice(colors);
            Ok(())
        })
    }

    /// Repaint one zone, zero-based, leaving the others as they are.
    ///
    /// # Errors
    ///
    /// [`Error::ZoneCountMismatch`] if `index` is past the last zone.
    pub fn set_zone(&self, index: usize, color: [u8; 3]) -> Result<()> {
        self.paint(|current| match current.get_mut(index) {
            Some(zone) => {
                *zone = color;
                Ok(())
            }
            None => Err(Error::ZoneCountMismatch {
                expected: current.len(),
                got: index.saturating_add(1),
            }),
        })
    }

    /// Paint every zone one color.
    ///
    /// # Errors
    ///
    /// Nothing here fails; the signature matches the other writers.
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
    /// retrying would send nothing forever. A transport failure does not — the
    /// breaker already refuses a device that is down, cheaply, and a stream
    /// outlives a device that comes back.
    #[must_use]
    pub fn error(&self) -> Option<Arc<Error>> {
        self.shared.failure.lock().ok().and_then(|e| e.clone())
    }

    /// Stop emitting and disarm the channel.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the disarming frame cannot be sent. The task is
    /// stopped either way.
    pub async fn close(self) -> Result<()> {
        self.task.abort();
        send_enable(&self.shared, 0).await
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
        self.task.abort();
        let shared = Arc::clone(&self.shared);
        // Best effort: a `Drop` cannot await, and cannot report a failure
        // either. `close` is the way to know the channel was disarmed.
        tokio::spawn(async move {
            if let Err(e) = send_enable(&shared, 0).await {
                tracing::warn!(id = %shared.id, error = %e, "could not disarm the segment channel");
            }
        });
    }
}

/// The device file entry claiming `role`.
fn named(device: &Device, mode: Mode, role: Role) -> Result<&str> {
    device
        .command_for(mode, role)
        .ok_or_else(|| Error::NoSegmentCommand {
            sku: device.sku.clone(),
            mode,
            role,
        })
}

fn zone_count(device: &Device, zones: Zones) -> Result<usize> {
    let count = match zones {
        Zones::App => device.capabilities.segment_count,
        Zones::Native => match device.capabilities.native_pixels {
            0 => {
                return Err(Error::NativeResolutionUnknown {
                    sku: device.sku.clone(),
                });
            }
            measured => measured,
        },
        Zones::Exact(n) => u32::from(n),
    };
    Ok(count.try_into().unwrap_or(usize::MAX))
}

/// The rate to send at, and a warning when nothing was measured.
fn rate_hz(device: &Device, sku: &str, zones: usize, rate: Rate, fallback: f64) -> f64 {
    match rate {
        Rate::Fixed(hz) => hz,
        Rate::Measured => device
            .measurements
            .clean_hz(u32::try_from(zones).unwrap_or(u32::MAX))
            .unwrap_or_else(|| {
                tracing::warn!(
                    %sku,
                    fallback_hz = fallback,
                    "no `measurements.frame_rate` for this unit; streaming at the fallback rate"
                );
                fallback
            }),
    }
}

/// The `gradient` value to send, if the command declares the argument.
///
/// A device file that leaves it out gets nothing extra: the codec refuses an
/// argument the command does not declare.
fn gradient_arg(device: &Device, mode: Mode, command: &str, gradient: bool) -> Option<i64> {
    device
        .commands
        .get(mode)
        .get(command)
        .filter(|spec| matches!(spec.args.get("gradient"), Some(ArgSpec::Int { .. })))
        .map(|_| i64::from(gradient))
}
