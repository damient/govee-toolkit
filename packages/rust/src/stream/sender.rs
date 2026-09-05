//! The task that puts frames on the wire.
//!
//! It owns the cadence and nothing else: the colors it sends are whatever the
//! writers last left, and a tick that finds them unchanged sends nothing. An
//! idle stream therefore costs no traffic, which matters on a channel shared
//! with the status requests the breaker reads health from.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::MissedTickBehavior;

use crate::codec::{Args, Encoded, Mode};
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::lan::{DeviceId, Verify};

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
    /// The device file entry that arms and disarms the channel.
    pub(crate) enable: String,
    /// The device file entry that paints zones.
    pub(crate) color: String,
    /// The `gradient` argument to send, if the command declares one.
    pub(crate) gradient: Option<i64>,
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
}

impl Shared {
    /// The tick length. `hz` is checked above zero when the stream opens.
    fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.hz)
    }
}

pub(crate) async fn send_enable(shared: &Shared, on: i64) -> Result<()> {
    let encoded = encode(shared, &shared.enable, &Args::new().int("on", on))?;
    write(shared, &encoded).await
}

/// Emit the current colors, forever, at the stream's rate.
pub(crate) async fn run(shared: Arc<Shared>) {
    let mut ticker = tokio::time::interval(shared.interval());
    // A tick missed because a write took the lock is a tick to skip, not one to
    // catch up on: catching up would send a burst at exactly the moment the
    // device is least able to take one.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let generation = shared.generation.load(Ordering::Acquire);
        if generation == shared.emitted.load(Ordering::Acquire) {
            continue;
        }

        let Ok(colors) = shared.colors.lock().map(|colors| colors.clone()) else {
            return;
        };

        let encoded = match encode(&shared, &shared.color, &frame_args(&shared, colors)) {
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
        match write(&shared, &encoded).await {
            Ok(()) => {
                shared.sent.fetch_add(1, Ordering::Relaxed);
            }
            // Transient by nature: the device is unreachable or the breaker is
            // refusing it, and both are answered by carrying on — the next tick
            // costs a lock and a refusal decided from recorded state.
            Err(e) => tracing::warn!(id = %shared.id, error = %e, "segment frame not sent"),
        }
    }
}

/// The repeat count is left out on purpose: the codec derives it from the list,
/// which is the one place it cannot disagree with the colors actually sent.
fn frame_args(shared: &Shared, colors: Vec<[u8; 3]>) -> Args {
    let args = Args::new().rgb("colors", colors);
    match shared.gradient {
        Some(gradient) => args.int("gradient", gradient),
        None => args,
    }
}

fn encode(shared: &Shared, command: &str, args: &Args) -> Result<Encoded> {
    shared.govee.encode(&shared.sku, shared.mode, command, args)
}

/// Write a frame. The one thing here bound to a single transport, and the seam
/// the `Transport` trait replaces when `ble` lands — `docs/architecture.md`.
///
/// Nothing is verified: the channel never answers, so a status request would
/// only be traffic competing with the frames it checks.
async fn write(shared: &Shared, encoded: &Encoded) -> Result<()> {
    shared
        .govee
        .inner
        .lan
        .send(&shared.id, encoded, Verify::None)
        .await?;
    Ok(())
}
