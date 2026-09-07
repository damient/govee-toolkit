//! The task that puts frames on the wire.
//!
//! It owns the cadence and nothing else. A tick that finds the colors
//! unchanged sends nothing, so an idle stream costs no traffic on a channel it
//! shares with the breaker's status requests.

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
use crate::transport::{DeviceId, Transport, Verify};

/// What the stream handle and its task share.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) govee: Govee,
    pub(crate) id: DeviceId,
    /// Fixed for the stream's life: commands are named per mode, so a change
    /// would send another table's bytes.
    pub(crate) mode: Mode,
    /// Resolved once, so no frame re-takes the transport's lock for it.
    pub(crate) sku: String,
    /// The transport serving [`Shared::mode`], resolved once.
    pub(crate) transport: Arc<dyn Transport>,
    /// The entry that arms and disarms the channel, and the argument the flag
    /// goes in. `None` where the file declares none for this mode.
    pub(crate) enable: Option<Enable>,
    /// The entry that sets zone interpolation and the value to send, where
    /// the mode carries it outside the painting frame.
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
    /// Raised by the handle to end the stream. A signal rather than an abort,
    /// so the task can send the disarming frame itself.
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
/// The disarm belongs here because a handle cannot await a frame from `Drop`.
/// Returns whether that frame went out.
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

        let encoded = match encode_repaint(shared, colors) {
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
                // Transient: the next tick costs a lock and a refusal decided
                // from recorded state.
                Err(e) => tracing::warn!(id = %shared.id, error = %e, "segment frame not sent"),
            }
        }
    }
}

/// Every frame one repaint takes, all encoded before any goes out: a repaint
/// the codec refuses leaves the previous picture, not half of the new one.
fn encode_repaint(shared: &Shared, colors: Vec<[u8; 3]>) -> Result<Vec<Encoded>> {
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
/// Nothing is verified: the channel never answers.
async fn write(shared: &Shared, encoded: &Encoded) -> Result<()> {
    shared
        .transport
        .send(&shared.id, encoded, Verify::None)
        .await?;
    Ok(())
}
