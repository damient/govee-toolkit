//! `ArtPoll` and `ArtPollReply`: how a desk finds the node.
//!
//! A desk broadcasts an `ArtPoll` every few seconds. A node that answers
//! nothing is listed nowhere, and the operator has to type an address by hand.
//! The node therefore answers every poll — see `docs/dmx.md`.
//!
//! One reply carries 4 output ports, and the 4 share a Net and a Sub-Net. The
//! node sends one reply for each group of 4 port-addresses the patch holds.
//!
//! This module builds bytes and holds no socket. Every offset below counts
//! from the first byte of the packet, and the layout is Art-Net 4.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use super::{ID, OP_POLL_REPLY, PORT, PORT_ADDRESS};

/// How many bytes an `ArtPollReply` takes.
pub const REPLY: usize = 239;
/// How many output ports one reply carries.
const PORTS: usize = 4;

/// The manufacturer code of a node that registered none.
const OEM_UNKNOWN: u16 = 0x00FF;
/// Indicators in normal mode, and every port-address set by the patch rather
/// than over the network.
const STATUS1: u8 = 0xD0;
/// The node reads the 15-bit port-address, which is Art-Net 3 and above.
const STATUS2: u8 = 0x08;
/// `StNode`: a node that converts between Art-Net and DMX.
const STYLE: u8 = 0x00;
/// The port outputs DMX512 from Art-Net.
const PORT_TYPE_OUTPUT: u8 = 0x80;
/// The port is transmitting data.
const GOOD_OUTPUT: u8 = 0x80;

/// Where each field sits in the packet.
mod at {
    /// The opcode, low byte first.
    pub(super) const OPCODE: usize = 8;
    /// The address a desk sends `ArtDmx` to.
    pub(super) const IP: usize = 10;
    /// The port a desk sends to, low byte first.
    pub(super) const PORT: usize = 14;
    /// The node's own version, high byte first.
    pub(super) const VERSION: usize = 16;
    /// The 7-bit Net the 4 ports share.
    pub(super) const NET_SWITCH: usize = 18;
    /// The 4-bit Sub-Net the 4 ports share.
    pub(super) const SUB_SWITCH: usize = 19;
    /// The manufacturer code, high byte first.
    pub(super) const OEM: usize = 20;
    /// What the node reports about itself.
    pub(super) const STATUS1: usize = 23;
    /// 18 bytes, null-terminated.
    pub(super) const SHORT_NAME: usize = 26;
    /// 64 bytes, null-terminated.
    pub(super) const LONG_NAME: usize = 44;
    /// 64 bytes: a status code and a line a person reads.
    pub(super) const NODE_REPORT: usize = 108;
    /// How many of the 4 ports the reply uses, high byte first.
    pub(super) const NUM_PORTS: usize = 172;
    /// 4 bytes: what each port is.
    pub(super) const PORT_TYPES: usize = 174;
    /// 4 bytes: what each output port is doing.
    pub(super) const GOOD_OUTPUT: usize = 182;
    /// 4 bytes: the 4-bit Universe of each output port.
    pub(super) const SW_OUT: usize = 190;
    /// The kind of device the node is.
    pub(super) const STYLE: usize = 200;
    /// The address of the reply that binds this one, which is the node's own.
    pub(super) const BIND_IP: usize = 207;
    /// Which reply this is, counting from 1.
    pub(super) const BIND_INDEX: usize = 211;
    /// What the node reads of the protocol.
    pub(super) const STATUS2: usize = 212;
}

/// What a desk asks for when it polls.
///
/// The node answers every poll the same way. The two fields are reported and
/// drive nothing: the node sends no unsolicited reply, so `talk_to_me` has
/// nothing to turn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poll {
    /// What the desk asks the node to send on its own.
    pub talk_to_me: u8,
    /// The lowest priority of diagnostic the desk wants.
    pub priority: u8,
}

/// What the node says about itself in a reply.
#[derive(Debug, Clone, Copy)]
pub struct Identity<'a> {
    /// The address a desk sends `ArtDmx` to.
    pub ip: Ipv4Addr,
    /// The name a desk lists the node under, from the patch.
    pub name: &'a str,
}

/// One reply's worth of a rig: up to 4 port-addresses that share a Net and a
/// Sub-Net.
#[derive(Debug, Default)]
struct Group {
    net: u8,
    sub_net: u8,
    /// The 4-bit Universe of each port.
    ports: Vec<u8>,
}

/// One `ArtPollReply` for each group of 4 port-addresses in `universes`.
///
/// A rig on no port-address answers with one reply that carries no port: the
/// desk must still list the node.
#[must_use]
pub fn replies(identity: &Identity<'_>, universes: &[u16]) -> Vec<[u8; REPLY]> {
    groups(universes)
        .iter()
        .enumerate()
        .map(|(index, group)| reply(identity, group, index))
        .collect()
}

/// The port-addresses, grouped the way a reply carries them.
fn groups(universes: &[u16]) -> Vec<Group> {
    let mut addresses: Vec<u16> = universes
        .iter()
        .map(|address| address & PORT_ADDRESS)
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    // The Net and the Sub-Net are the 11 bits above the Universe, and the 4
    // ports of one reply share them.
    let mut shared: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
    for address in addresses {
        let universe = u8::try_from(address & 0x000F).unwrap_or(0);
        shared.entry(address >> 4).or_default().push(universe);
    }
    let mut groups = Vec::new();
    for (high, ports) in shared {
        for chunk in ports.chunks(PORTS) {
            groups.push(Group {
                net: u8::try_from(high >> 4).unwrap_or(0),
                sub_net: u8::try_from(high & 0x000F).unwrap_or(0),
                ports: chunk.to_vec(),
            });
        }
    }
    if groups.is_empty() {
        groups.push(Group::default());
    }
    groups
}

/// One reply, for one group. `index` counts from 0 and `BindIndex` from 1.
fn reply(identity: &Identity<'_>, group: &Group, index: usize) -> [u8; REPLY] {
    let mut bytes = Bytes([0u8; REPLY]);
    bytes.put(0, ID.as_slice());
    bytes.put(at::OPCODE, &OP_POLL_REPLY.to_le_bytes());
    bytes.put(at::IP, &identity.ip.octets());
    bytes.put(at::PORT, &PORT.to_le_bytes());
    bytes.put(at::VERSION, &version());
    bytes.set(at::NET_SWITCH, group.net);
    bytes.set(at::SUB_SWITCH, group.sub_net);
    bytes.put(at::OEM, &OEM_UNKNOWN.to_be_bytes());
    bytes.set(at::STATUS1, STATUS1);
    bytes.name(at::SHORT_NAME, 18, identity.name);
    bytes.name(at::LONG_NAME, 64, identity.name);
    bytes.name(
        at::NODE_REPORT,
        64,
        &format!("#0001 [0000] {}", identity.name),
    );
    let used = u8::try_from(group.ports.len()).unwrap_or(0);
    bytes.put(at::NUM_PORTS, &[0, used]);
    for (port, universe) in group.ports.iter().enumerate() {
        bytes.set(at::PORT_TYPES + port, PORT_TYPE_OUTPUT);
        bytes.set(at::GOOD_OUTPUT + port, GOOD_OUTPUT);
        bytes.set(at::SW_OUT + port, *universe);
    }
    bytes.set(at::STYLE, STYLE);
    bytes.put(at::BIND_IP, &identity.ip.octets());
    bytes.set(at::BIND_INDEX, u8::try_from(index + 1).unwrap_or(u8::MAX));
    bytes.set(at::STATUS2, STATUS2);
    bytes.0
}

/// The node's own version, which a desk shows beside the name.
fn version() -> [u8; 2] {
    let part = |text: &str| text.parse::<u8>().unwrap_or(0);
    [
        part(env!("CARGO_PKG_VERSION_MAJOR")),
        part(env!("CARGO_PKG_VERSION_MINOR")),
    ]
}

/// One packet, under construction.
///
/// Every offset is a constant inside a fixed buffer, so a write past the end
/// cannot happen and is dropped rather than checked by the caller.
struct Bytes([u8; REPLY]);

impl Bytes {
    fn put(&mut self, at: usize, bytes: &[u8]) {
        if let Some(target) = self.0.get_mut(at..at + bytes.len()) {
            target.copy_from_slice(bytes);
        }
    }

    fn set(&mut self, at: usize, byte: u8) {
        if let Some(target) = self.0.get_mut(at) {
            *target = byte;
        }
    }

    /// A field of `len` bytes that holds `text` and a null terminator. A
    /// longer name is cut at a character.
    fn name(&mut self, at: usize, len: usize, text: &str) {
        let limit = len.saturating_sub(1);
        let end = text
            .char_indices()
            .map(|(start, character)| start + character.len_utf8())
            .take_while(|&end| end <= limit)
            .last()
            .unwrap_or(0);
        self.put(at, text.get(..end).unwrap_or("").as_bytes());
    }
}
