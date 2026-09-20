//! The receive loop, over a socket on the loopback address.
//!
//! A dry run resolves and writes nothing, so these tests need no device. The
//! send path is exercised against `crates/sim` in `tests/wire.rs`.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Catalog;
use tokio::sync::mpsc;

use super::{Node, Observer};
use crate::apply::Look;
use crate::input::UniverseFrame;
use crate::input::artnet::REPLY;
use crate::input::socket::{Listener, MAX_DATAGRAM};
use crate::patch::{Patch, Rig};

/// The rig the patch tests load: two `segment` fixtures at 1 and 33, and one
/// `full` fixture at 65.
const RIG: &str = include_str!("../../tests/fixtures/patch.yaml");

/// What the node reported, and a stop once enough of it arrived.
struct Recorder {
    received: Vec<u16>,
    resolved: Vec<(DeviceId, Look)>,
    refused: Vec<String>,
    ignored: Vec<u16>,
    polled: Vec<usize>,
    stop: mpsc::Sender<()>,
}

impl Recorder {
    fn new(stop: mpsc::Sender<()>) -> Self {
        Self {
            received: Vec::new(),
            resolved: Vec::new(),
            refused: Vec::new(),
            ignored: Vec::new(),
            polled: Vec::new(),
            stop,
        }
    }

    /// End the run. The test asks for one packet, so the loop stops as soon as
    /// that packet is reported.
    fn done(&self) {
        let _ = self.stop.try_send(());
    }
}

impl Observer for Recorder {
    fn received(&mut self, frame: &UniverseFrame) {
        self.received.push(frame.universe);
    }

    fn resolved(&mut self, id: &DeviceId, look: &Look) {
        self.resolved.push((id.clone(), look.clone()));
        if self.resolved.len() == 3 {
            self.done();
        }
    }

    fn refused(&mut self, _source: SocketAddr, reason: &str) {
        self.refused.push(reason.to_owned());
        self.done();
    }

    fn ignored(&mut self, _source: SocketAddr, opcode: u16) {
        self.ignored.push(opcode);
        self.done();
    }

    fn polled(&mut self, _source: SocketAddr, replies: usize) {
        self.polled.push(replies);
        self.done();
    }
}

fn loopback() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}

fn rig(catalog: &Catalog) -> Rig {
    let patch = Patch::parse(RIG, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
    let segment = catalog.device("H61A0").expect("the SKU resolves");
    let full = catalog.device("H6008").expect("the SKU resolves");
    patch
        .resolve(
            |id| {
                if id == &DeviceId::new("AA:BB:CC:DD:EE:03") {
                    Some(full)
                } else {
                    Some(segment)
                }
            },
            |sku| catalog.device(sku).ok(),
        )
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

/// An `ArtDmx` packet, in the layout `docs/dmx.md` documents.
fn packet(universe: u16, sequence: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = b"Art-Net\0".to_vec();
    bytes.extend_from_slice(&0x5000u16.to_le_bytes());
    bytes.extend_from_slice(&14u16.to_be_bytes());
    bytes.push(sequence);
    bytes.push(0);
    let [net, sub_uni] = universe.to_be_bytes();
    bytes.push(sub_uni);
    bytes.push(net);
    let length = u16::try_from(data.len()).unwrap_or(0);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(data);
    bytes
}

/// Send `datagram` to the node's socket, then run until the recorder stops the
/// loop.
async fn drive(datagram: &[u8], rig: Rig) -> Recorder {
    let listener = Listener::bind(loopback()).expect("the node socket binds");
    let bound = listener.local_addr().expect("a bound address");
    let sender = Listener::bind(loopback()).expect("the sender socket binds");
    sender.send_to(datagram, bound).await.expect("the send");

    let (stop, mut stopped) = mpsc::channel(1);
    let mut recorder = Recorder::new(stop);
    let mut node = Node::dry_run(rig);
    node.run(
        &listener,
        async move {
            stopped.recv().await;
        },
        &mut recorder,
    )
    .await
    .expect("the receive loop");
    recorder
}

/// A desk drives every fixture of the universe it addresses, and the dimmer of
/// each one is the first channel of its own start address.
#[tokio::test]
async fn one_frame_reaches_every_fixture_of_its_universe() {
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let mut slots = vec![0u8; 70];
    slots[0] = 255;
    slots[32] = 128;
    slots[64] = 255;
    let recorder = drive(&packet(0, 1, &slots), rig(&catalog)).await;

    assert_eq!(recorder.received, [0]);
    assert_eq!(recorder.resolved.len(), 3);
    assert!(recorder.resolved.iter().all(|(_, look)| look.on));
    assert_eq!(recorder.resolved[0].0, DeviceId::new("AA:BB:CC:DD:EE:01"));
    assert_eq!(recorder.resolved[0].1.zones.len(), 10);
    assert_eq!(recorder.resolved[2].1.brightness, Some(100));
    assert!(recorder.refused.is_empty());
}

/// A packet the protocol refuses drops, and the node keeps listening.
#[tokio::test]
async fn a_refused_packet_reaches_no_fixture() {
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let recorder = drive(&packet(0, 1, &[255, 255, 255]), rig(&catalog)).await;

    assert_eq!(recorder.refused.len(), 1, "an odd length is refused");
    assert!(recorder.resolved.is_empty());
    assert!(recorder.received.is_empty());
}

/// An opcode the node drives nothing with is reported and drives nothing.
#[tokio::test]
async fn a_packet_that_is_no_artdmx_drives_nothing() {
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let mut reply = b"Art-Net\0".to_vec();
    reply.extend_from_slice(&0x2100u16.to_le_bytes());
    reply.extend_from_slice(&[0u8; 8]);
    let recorder = drive(&reply, rig(&catalog)).await;

    assert_eq!(recorder.ignored, [0x2100]);
    assert!(recorder.resolved.is_empty());
}

/// A desk that receives no reply lists the node nowhere, so a poll is
/// answered on the socket it arrived on. The rig sits on one port-address,
/// which is one reply.
///
/// The node answers the address [`Node::replies_to`] names, and not the desk
/// that polled: the real one is the broadcast address, which no test puts a
/// packet on.
#[tokio::test]
async fn a_poll_is_answered_on_the_socket_it_arrived_on() {
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let listener = Listener::bind(loopback()).expect("the node socket binds");
    let bound = listener.local_addr().expect("a bound address");
    let desk = Listener::bind(loopback()).expect("the desk socket binds");
    let listens = desk.local_addr().expect("a bound address");

    let mut poll = b"Art-Net\0".to_vec();
    poll.extend_from_slice(&0x2000u16.to_le_bytes());
    poll.extend_from_slice(&14u16.to_be_bytes());
    poll.extend_from_slice(&[0, 0]);
    desk.send_to(&poll, bound).await.expect("the send");

    let (stop, mut stopped) = mpsc::channel(1);
    let mut recorder = Recorder::new(stop);
    let mut node = Node::dry_run(rig(&catalog))
        .named("a-desk-lists-this")
        .replies_to(listens);
    node.run(
        &listener,
        async move {
            stopped.recv().await;
        },
        &mut recorder,
    )
    .await
    .expect("the receive loop");

    assert_eq!(recorder.polled, [1]);
    let mut buffer = [0u8; MAX_DATAGRAM];
    let (read, from) = desk.receive(&mut buffer).await.expect("the reply");
    assert_eq!(from, bound);
    assert_eq!(read, REPLY);
    assert_eq!(buffer.get(..8), Some(b"Art-Net\0".as_slice()));
    assert_eq!(buffer.get(8..10), Some(0x2100u16.to_le_bytes().as_slice()));
    assert_eq!(
        buffer.get(10..14),
        Some([127, 0, 0, 1].as_slice()),
        "the address the desk must send ArtDmx to"
    );
    assert_eq!(buffer.get(26..43), Some(b"a-desk-lists-this".as_slice()));
}
