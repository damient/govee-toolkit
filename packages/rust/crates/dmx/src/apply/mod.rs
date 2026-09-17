//! The send path: the channels a frame carries, joined to the device.
//!
//! One task per fixture. The node hands each task the latest look and never
//! waits for a write, so a slow device holds up no other one and no frame
//! holds up the socket. A look replaced before its task took it is counted,
//! not queued: that count is what tells the operator the desk sends faster
//! than the devices accept — see `docs/dmx.md`.

mod backoff;
mod device;
mod look;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use govee_toolkit::{DeviceId, Govee};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

pub use self::look::Look;
use crate::patch::Rig;

/// How many failures the node holds before it drops one. A report that waits
/// would put the reader on the send path.
const FAILURES: usize = 64;

/// The two waits a fixture keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// How long a device goes without a write before it receives the current
    /// values again. Nothing acknowledges a LAN frame.
    pub refresh: Duration,
    /// How long a fixture waits for a frame before the patch decides what it
    /// shows — see [`crate::patch::SignalLoss`].
    pub silence: Duration,
}

/// What one device did over a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    /// The device.
    pub id: DeviceId,
    /// Writes that reached the transport, which is one per command and one
    /// per stream frame.
    pub frames_sent: u64,
    /// Looks and frames a later one replaced before they went out. Expected:
    /// it is what a desk faster than the device costs.
    pub frames_superseded: u64,
}

/// A write that failed. The device keeps its patch entry and the run carries
/// on — a show does not stop because one fixture dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The device that did not take the write.
    pub id: DeviceId,
    /// What the SDK reported.
    pub reason: String,
}

/// The send path of a whole rig.
#[derive(Debug)]
pub struct Applier {
    feeds: BTreeMap<DeviceId, Feed>,
}

#[derive(Debug)]
struct Feed {
    looks: watch::Sender<(u64, Look)>,
    /// Counts the looks pushed. The task reads the gap to know what it
    /// missed.
    pushed: AtomicU64,
    task: JoinHandle<Counts>,
}

impl Applier {
    /// Start one task per fixture of `rig`.
    ///
    /// The tasks write nothing until the first look arrives, so a rig waits
    /// dark for the desk rather than going to the signal loss answer at
    /// start.
    #[must_use]
    pub fn start(govee: &Govee, rig: &Rig, timing: Timing) -> (Self, mpsc::Receiver<Failure>) {
        let (sender, failures) = mpsc::channel(FAILURES);
        let mut feeds = BTreeMap::new();
        for fixture in rig.fixtures() {
            let feeder = device::Feeder::new(govee, fixture, timing, sender.clone());
            let (looks, receiver) = watch::channel((0, Look::default()));
            feeds.insert(
                fixture.entry.device.clone(),
                Feed {
                    looks,
                    pushed: AtomicU64::new(0),
                    task: tokio::spawn(device::run(feeder, receiver)),
                },
            );
        }
        (Self { feeds }, failures)
    }

    /// Hand one device the look a frame asks of it.
    ///
    /// Never waits. A look that arrives before the task took the previous one
    /// replaces it, and the replaced one is counted.
    pub fn push(&self, id: &DeviceId, look: Look) {
        let Some(feed) = self.feeds.get(id) else {
            return;
        };
        let generation = feed.pushed.fetch_add(1, Ordering::Relaxed) + 1;
        feed.looks.send_replace((generation, look));
    }

    /// Stop every task, disarm every stream, and answer what each device did.
    pub async fn close(self) -> Vec<Counts> {
        let mut counts = Vec::with_capacity(self.feeds.len());
        for (_, feed) in self.feeds {
            // The task ends when the last sender of its channel goes.
            drop(feed.looks);
            if let Ok(count) = feed.task.await {
                counts.push(count);
            }
        }
        counts
    }
}
