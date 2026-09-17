//! The node: one socket in, one rig out.
//!
//! It receives the datagrams, drops what the protocol refuses, resolves each
//! frame against the patch, and hands every fixture what the frame asks of it.
//! A dry run stops before the last step and writes to no device.
//!
//! The node prints nothing. It reports through an [`Observer`], so the binary
//! decides what a person reads — see `docs/dmx.md`.

#[cfg(test)]
mod tests;

use std::future::{Future, pending};
use std::net::SocketAddr;
use std::time::Duration;

use govee_toolkit::{DeviceId, Govee};
use tokio::sync::mpsc;

use crate::apply::{Applier, Counts, Failure, Look};
use crate::input::UniverseFrame;
use crate::input::artnet::{self, Gate, Packet};
use crate::input::socket::{Error, Listener, MAX_DATAGRAM};
use crate::patch::Rig;

/// What the node reports as it runs.
///
/// Every method does nothing by default, so an implementation carries the
/// ones it prints and no other.
pub trait Observer {
    /// One frame the node accepted.
    fn received(&mut self, frame: &UniverseFrame) {
        let _ = frame;
    }

    /// What one patched fixture reads out of that frame.
    fn resolved(&mut self, id: &DeviceId, look: &Look) {
        let _ = (id, look);
    }

    /// A packet the node dropped, and why.
    fn refused(&mut self, source: SocketAddr, reason: &str) {
        let _ = (source, reason);
    }

    /// An Art-Net packet that carries no channel value. `ArtPoll` lands here
    /// until the node answers polls.
    fn ignored(&mut self, source: SocketAddr, opcode: u16) {
        let _ = (source, opcode);
    }

    /// A write that failed. Every other device keeps running.
    fn failed(&mut self, id: &DeviceId, reason: &str) {
        let _ = (id, reason);
    }
}

/// What the node does with a resolved look.
#[derive(Debug)]
enum Output {
    /// Resolve and write nothing.
    DryRun,
    /// Write to the devices.
    Live {
        applier: Applier,
        failures: mpsc::Receiver<Failure>,
    },
}

/// The Art-Net node.
#[derive(Debug)]
pub struct Node {
    rig: Rig,
    gate: Gate,
    output: Output,
}

/// What ended one pass of the receive loop.
enum Wake {
    Stop,
    Failed(Failure),
    Datagram(usize, SocketAddr),
}

impl Node {
    /// A node that resolves every frame and writes to no device.
    #[must_use]
    pub fn dry_run(rig: Rig) -> Self {
        Self {
            rig,
            gate: Gate::new(),
            output: Output::DryRun,
        }
    }

    /// A node that writes to the devices `govee` reaches.
    ///
    /// One task starts per fixture. `refresh` is how long a device goes
    /// without a write before it receives the current values again.
    #[must_use]
    pub fn live(rig: Rig, govee: &Govee, refresh: Duration) -> Self {
        let (applier, failures) = Applier::start(govee, &rig, refresh);
        Self {
            rig,
            gate: Gate::new(),
            output: Output::Live { applier, failures },
        }
    }

    /// The rig the node drives.
    #[must_use]
    pub fn rig(&self) -> &Rig {
        &self.rig
    }

    /// Receive until `shutdown` answers.
    ///
    /// # Errors
    ///
    /// [`Error::Receive`] where the socket fails. A packet the protocol
    /// refuses is not an error: it reaches [`Observer::refused`], and the node
    /// keeps listening.
    pub async fn run(
        &mut self,
        listener: &Listener,
        shutdown: impl Future<Output = ()>,
        observer: &mut impl Observer,
    ) -> Result<(), Error> {
        let mut buffer = [0u8; MAX_DATAGRAM];
        tokio::pin!(shutdown);
        loop {
            let wake = tokio::select! {
                () = &mut shutdown => Wake::Stop,
                failure = failure(&mut self.output) => Wake::Failed(failure),
                received = listener.receive(&mut buffer) => {
                    let (read, source) = received?;
                    Wake::Datagram(read, source)
                }
            };
            match wake {
                Wake::Stop => return Ok(()),
                Wake::Failed(failure) => observer.failed(&failure.id, &failure.reason),
                Wake::Datagram(read, source) => {
                    let datagram = buffer.get(..read).unwrap_or(&[]);
                    self.datagram(datagram, source, observer);
                }
            }
        }
    }

    /// Stop every fixture, disarm every stream, and answer what each device
    /// did.
    pub async fn close(self) -> Vec<Counts> {
        match self.output {
            Output::DryRun => Vec::new(),
            Output::Live { applier, .. } => applier.close().await,
        }
    }

    /// One datagram, from the wire to the fixtures.
    fn datagram(&mut self, bytes: &[u8], source: SocketAddr, observer: &mut impl Observer) {
        match artnet::parse(bytes, source) {
            Ok(Packet::Dmx(dmx)) => {
                if self.gate.accept(source, dmx.frame.universe, dmx.sequence) {
                    self.frame(&dmx.frame, observer);
                } else {
                    observer.refused(source, "older than the last packet accepted");
                }
            }
            Ok(Packet::Other { opcode }) => observer.ignored(source, opcode),
            Err(error) => observer.refused(source, &error.to_string()),
        }
    }

    /// One frame, resolved against the patch.
    fn frame(&self, frame: &UniverseFrame, observer: &mut impl Observer) {
        observer.received(frame);
        for fixture in self.rig.fixtures() {
            if fixture.universe.get() != frame.universe {
                continue;
            }
            let look = Look::read(fixture, frame);
            observer.resolved(&fixture.entry.device, &look);
            if let Output::Live { applier, .. } = &self.output {
                applier.push(&fixture.entry.device, look);
            }
        }
    }
}

/// The next failure a fixture reports. A dry run reports none, so the branch
/// that waits for one never completes.
async fn failure(output: &mut Output) -> Failure {
    let Output::Live { failures, .. } = output else {
        return pending().await;
    };
    match failures.recv().await {
        Some(failure) => failure,
        None => pending().await,
    }
}
