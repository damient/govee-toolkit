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
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use govee_toolkit::{DeviceId, Govee};
use tokio::sync::mpsc;

use crate::apply::{Applier, Counts, Failure, Look, Timing};
use crate::input::UniverseFrame;
use crate::input::artnet::{self, Gate, Identity, Packet, replies};
use crate::input::socket::{Error, Listener, MAX_DATAGRAM};
use crate::patch::{NODE_NAME, Rig};

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

    /// An Art-Net packet that carries no channel value.
    fn ignored(&mut self, source: SocketAddr, opcode: u16) {
        let _ = (source, opcode);
    }

    /// A poll the node answered, and how many replies went out.
    fn polled(&mut self, source: SocketAddr, replies: usize) {
        let _ = (source, replies);
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
    /// The name a desk lists the node under, which `ArtPollReply` carries.
    name: String,
    /// Where an `ArtPollReply` goes.
    reply_to: SocketAddr,
}

/// Where an `ArtPollReply` goes: every Art-Net application on the network,
/// which includes one that shares the host with the node.
const BROADCAST: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), artnet::PORT);

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
            name: NODE_NAME.to_owned(),
            reply_to: BROADCAST,
        }
    }

    /// A node that writes to the devices `govee` reaches.
    ///
    /// One task starts per fixture, and `timing` says how long each one waits
    /// before a refresh and before the patched answer to a silent sender.
    #[must_use]
    pub fn live(rig: Rig, govee: &Govee, timing: Timing) -> Self {
        let (applier, failures) = Applier::start(govee, &rig, timing);
        Self {
            rig,
            gate: Gate::new(),
            output: Output::Live { applier, failures },
            name: NODE_NAME.to_owned(),
            reply_to: BROADCAST,
        }
    }

    /// The name a desk lists the node under. The default is
    /// [`crate::patch::NODE_NAME`].
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Where an `ArtPollReply` goes. The default is the broadcast address,
    /// which is what Art-Net asks for. A test names an address of its own, so
    /// that it puts no packet on the network.
    #[must_use]
    pub fn replies_to(mut self, address: SocketAddr) -> Self {
        self.reply_to = address;
        self
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
                    self.datagram(datagram, source, listener, observer).await;
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
    async fn datagram(
        &mut self,
        bytes: &[u8],
        source: SocketAddr,
        listener: &Listener,
        observer: &mut impl Observer,
    ) {
        match artnet::parse(bytes, source) {
            Ok(Packet::Dmx(dmx)) => {
                if self.gate.accept(source, dmx.frame.universe, dmx.sequence) {
                    self.frame(&dmx.frame, observer);
                } else {
                    observer.refused(source, "older than the last packet accepted");
                }
            }
            Ok(Packet::Poll(_)) => self.answer(source, listener, observer).await,
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

impl Node {
    /// Answer a poll: one `ArtPollReply` for each group of 4 port-addresses
    /// the patch holds.
    ///
    /// The replies are broadcast, which Art-Net asks for. A unicast reply to
    /// port 6454 reaches one socket alone, so a desk that shares the host with
    /// the node lists nothing: the node receives its own reply instead.
    async fn answer(&self, source: SocketAddr, listener: &Listener, observer: &mut impl Observer) {
        let universes: Vec<u16> = self
            .rig
            .universes()
            .iter()
            .map(|address| address.get())
            .collect();
        let identity = Identity {
            ip: ip(listener, source),
            name: &self.name,
        };
        let mut sent = 0;
        for reply in replies(&identity, &universes) {
            match listener.send_to(&reply, self.reply_to).await {
                Ok(()) => sent += 1,
                Err(error) => observer.refused(source, &error.to_string()),
            }
        }
        observer.polled(source, sent);
    }
}

/// The address a desk must send `ArtDmx` to.
///
/// A node bound to `0.0.0.0` answers with the interface that reaches the desk,
/// because `0.0.0.0` is an address nothing can send to.
fn ip(listener: &Listener, source: SocketAddr) -> Ipv4Addr {
    let bound = listener.local_addr().map(|address| address.ip());
    let towards = || Listener::local_ip_towards(source);
    match bound
        .filter(|address| !address.is_unspecified())
        .or_else(towards)
    {
        Some(IpAddr::V4(address)) => address,
        _ => Ipv4Addr::UNSPECIFIED,
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
