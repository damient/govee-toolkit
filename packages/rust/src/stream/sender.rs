//! The task that puts frames on the wire.
//!
//! It owns the cadence and nothing else: the colors it sends are whatever the
//! writers last left, and a tick that finds them unchanged sends nothing. An
//! idle stream therefore costs no traffic, which matters on a channel shared
//! with the status requests the breaker reads health from.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;

use crate::codec::{Args, Encoded, Mode};
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::stream::paint;
use crate::stream::resolve::{Enable, Painter};
use crate::transport::{DeviceId, Verify};

/// What the stream handle and its task share.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) govee: Govee,
    pub(crate) id: DeviceId,
    /// The mode chosen when the stream opened, and fixed for its life: the
    /// device file names its commands per mode, so a stream that changed mode
    /// would be sending another file's bytes.
    pub(crate) mode: Mode,
    /// The SKU resolved when the stream opened. Carried so a frame does not
    /// re-take the transport's lock to look it up again.
    pub(crate) sku: String,
    /// The device file entry that arms and disarms the channel, and the
    /// argument the flag goes in. `None` where the file declares none for this
    /// mode: nothing is armed, and nothing is disarmed on close.
    pub(crate) enable: Option<Enable>,
    /// The entry that sets zone interpolation and the value to send, where the
    /// mode carries that setting outside the painting frame. `None` where the
    /// painting frame carries it, or where the file names neither.
    pub(crate) gradient: Option<(Enable, i64)>,
    /// How the device file paints zones over this mode, and the arguments it
    /// names for it.
    pub(crate) painter: Painter,
    pub(crate) hz: f64,
    /// Fixed when the stream opens: the firmware reads the count off the frame
    /// and re-groups the LEDs around it.
    pub(crate) zones: usize,
    pub(crate) colors: Mutex<Vec<[u8; 3]>>,
    /// Bumped by every write.
    pub(crate) generation: AtomicU64,
    /// The generation the last frame carried.
    pub(crate) emitted: AtomicU64,
    pub(crate) sent: AtomicU64,
    pub(crate) superseded: AtomicU64,
    /// What stopped the task.
    pub(crate) failure: Mutex<Option<Arc<Error>>>,
    /// Raised by the handle to end the stream. Signalling rather than aborting
    /// is what lets the task send the disarming frame itself.
    pub(crate) stop: Notify,
}

impl Shared {
    /// The tick length. `hz` is checked above zero when the stream opens.
    fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.hz)
    }
}

/// Arm or disarm the channel, where the device file names a command for it.
pub(crate) async fn send_enable(shared: &Shared, on: i64) -> Result<()> {
    let Some(enable) = &shared.enable else {
        return Ok(());
    };
    send_flag(shared, enable, on).await
}

/// Set zone interpolation, where the mode carries it in a frame of its own.
pub(crate) async fn send_gradient(shared: &Shared) -> Result<()> {
    let Some((command, on)) = &shared.gradient else {
        return Ok(());
    };
    send_flag(shared, command, *on).await
}

/// Write a one-flag command the device file named by role.
async fn send_flag(shared: &Shared, command: &Enable, value: i64) -> Result<()> {
    let encoded = encode(
        shared,
        &command.command,
        &Args::new().int(command.arg.as_str(), value),
    )?;
    write(shared, &encoded).await
}

/// Emit the current colors at the stream's rate, then disarm the channel.
///
/// The disarm belongs to this task rather than to the handle: a `Drop` cannot
/// await one, and spawning it there panics when no runtime is left to spawn
/// onto. Returns whether that frame went out, which is what
/// [`SegmentStream::close`](crate::SegmentStream::close) reports.
pub(crate) async fn run(shared: Arc<Shared>) -> Result<()> {
    emit(&shared).await;
    send_enable(&shared, 0).await
}

/// Returns when the handle asks for a stop, or when a frame cannot be encoded.
async fn emit(shared: &Shared) {
    let mut ticker = tokio::time::interval(shared.interval());
    // A tick missed because a write took the lock is a tick to skip, not one to
    // catch up on: catching up would send a burst at exactly the moment the
    // device is least able to take one.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            () = shared.stop.notified() => return,
            _ = ticker.tick() => {}
        }

        let generation = shared.generation.load(Ordering::Acquire);
        if generation == shared.emitted.load(Ordering::Acquire) {
            continue;
        }

        let Ok(colors) = shared.colors.lock().map(|colors| colors.clone()) else {
            return;
        };

        let encoded = match encode_repaint(shared, &colors) {
            Ok(encoded) => encoded,
            // The arguments will not become valid on a later tick.
            Err(e) => {
                tracing::error!(id = %shared.id, error = %e, "segment stream stopped");
                if let Ok(mut failure) = shared.failure.lock() {
                    *failure = Some(Arc::new(e));
                }
                return;
            }
        };

        shared.emitted.store(generation, Ordering::Release);
        for frame in &encoded {
            match write(shared, frame).await {
                Ok(()) => {
                    shared.sent.fetch_add(1, Ordering::Relaxed);
                }
                // Transient by nature: the device is unreachable or the breaker
                // is refusing it, and both are answered by carrying on — the
                // next tick costs a lock and a refusal decided from recorded
                // state.
                Err(e) => tracing::warn!(id = %shared.id, error = %e, "segment frame not sent"),
            }
        }
    }
}

/// Every frame one repaint takes: one for a whole-frame command, one per
/// distinct color for a masked one.
///
/// All of them are encoded before any goes out, so a repaint the codec refuses
/// leaves the device wearing what it wore rather than half of the new picture.
fn encode_repaint(shared: &Shared, colors: &[[u8; 3]]) -> Result<Vec<Encoded>> {
    paint::frames(&shared.painter, colors)?
        .iter()
        .map(|args| encode(shared, shared.painter.command(), args))
        .collect()
}

fn encode(shared: &Shared, command: &str, args: &Args) -> Result<Encoded> {
    shared.govee.encode(&shared.sku, shared.mode, command, args)
}

/// Write a frame, over whichever mode the stream was opened on.
///
/// Nothing is verified: the channel never answers, so a status request would
/// only be traffic competing with the frames it checks.
async fn write(shared: &Shared, encoded: &Encoded) -> Result<()> {
    shared
        .govee
        .transport(&shared.id, shared.mode)?
        .send(&shared.id, encoded, Verify::None)
        .await?;
    Ok(())
}
