//! The parser, checked against the captures and against every refusal.
//!
//! A packet built here is built from the layout `docs/dmx.md` documents, and
//! is not evidence of what a sender puts on the wire. The captures under
//! `tests/fixtures/artnet/` are, and
//! [`every_capture_produces_the_channels_the_sender_showed`] is what reads
//! them.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use super::{Dmx, Error, Identity, Packet, REPLY, Sequence, parse, replies};

fn source() -> SocketAddr {
    "192.0.2.2:6454".parse().expect("a socket address")
}

/// An `ArtDmx` packet, in the documented layout.
fn packet(universe: u16, sequence: u8, data: &[u8]) -> Vec<u8> {
    let length = u16::try_from(data.len()).unwrap_or(0);
    let mut bytes = b"Art-Net\0".to_vec();
    bytes.extend_from_slice(&0x5000u16.to_le_bytes());
    bytes.extend_from_slice(&14u16.to_be_bytes());
    bytes.push(sequence);
    bytes.push(0);
    let [net, sub_uni] = universe.to_be_bytes();
    bytes.push(sub_uni);
    bytes.push(net);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn dmx(bytes: &[u8]) -> Dmx {
    match parse(bytes, source()) {
        Ok(Packet::Dmx(dmx)) => dmx,
        other => panic!("{other:?}"),
    }
}

fn refusal(bytes: &[u8]) -> Error {
    parse(bytes, source()).expect_err("the packet is refused")
}

#[test]
fn a_packet_carries_its_channels_and_its_address() {
    let dmx = dmx(&packet(0x0103, 12, &[255, 128, 0, 7]));
    assert_eq!(dmx.frame.universe, 0x0103);
    assert_eq!(dmx.frame.len(), 4);
    assert_eq!(dmx.frame.values(), [255, 128, 0, 7]);
    assert_eq!(dmx.sequence, 12);
    assert_eq!(dmx.frame.source, source());
    assert_eq!(dmx.frame.priority, None, "Art-Net carries no priority");
}

/// An operator reads the dimmer at channel 1, so the parser must put slot 1
/// at address 1.
#[test]
fn a_channel_reads_at_the_address_a_desk_shows() {
    let dmx = dmx(&packet(0, 1, &[10, 20, 30, 40]));
    assert_eq!(dmx.frame.slot(1), Some(10));
    assert_eq!(dmx.frame.slot(4), Some(40));
    assert_eq!(dmx.frame.slot(5), Some(0), "an unaddressed slot reads 0");
    assert_eq!(dmx.frame.slot(0), None);
    assert_eq!(dmx.frame.slot(513), None);
}

/// Net and `SubUni` are two bytes of one 15-bit port-address, and the patch
/// spells that address either way.
#[test]
fn net_and_sub_uni_number_one_port_address() {
    let dmx = dmx(&packet(0x0113, 1, &[0, 0]));
    assert_eq!(dmx.frame.universe, 275);
}

#[test]
fn a_packet_that_is_not_art_net_is_dropped() {
    let mut bytes = packet(0, 1, &[0, 0]);
    bytes[0] = b'X';
    assert_eq!(refusal(&bytes), Error::NotArtNet);
}

#[test]
fn a_datagram_under_a_header_is_dropped() {
    assert_eq!(
        refusal(b"Art-Net\0"),
        Error::TooShort {
            carried: 8,
            need: 12
        }
    );
}

#[test]
fn a_protocol_version_under_14_is_dropped() {
    let mut bytes = packet(0, 1, &[0, 0]);
    bytes[10..12].copy_from_slice(&13u16.to_be_bytes());
    assert_eq!(refusal(&bytes), Error::Version { version: 13 });
}

#[test]
fn an_odd_length_is_dropped() {
    let mut bytes = packet(0, 1, &[0, 0, 0, 0]);
    bytes[16..18].copy_from_slice(&3u16.to_be_bytes());
    assert_eq!(refusal(&bytes), Error::OddLength { length: 3 });
}

/// The packet is never truncated to make it fit.
#[test]
fn a_length_over_one_universe_is_dropped() {
    let mut bytes = packet(0, 1, &[0; 512]);
    bytes[16..18].copy_from_slice(&514u16.to_be_bytes());
    assert_eq!(refusal(&bytes), Error::Length { length: 514 });
}

#[test]
fn a_packet_shorter_than_the_length_it_declares_is_dropped() {
    let mut bytes = packet(0, 1, &[1, 2, 3, 4]);
    bytes[16..18].copy_from_slice(&512u16.to_be_bytes());
    assert_eq!(
        refusal(&bytes),
        Error::Truncated {
            length: 512,
            carried: 4
        }
    );
}

/// A whole universe is what a desk sends, and it must arrive whole.
#[test]
fn a_full_universe_arrives_whole() {
    let data: Vec<u8> = (0..512)
        .map(|slot| u8::try_from(slot % 256).unwrap_or(0))
        .collect();
    let dmx = dmx(&packet(0, 1, &data));
    assert_eq!(dmx.frame.len(), 512);
    assert_eq!(dmx.frame.values(), data.as_slice());
}

/// An opcode the node drives nothing with reads as a packet rather than as a
/// refusal: a desk sends more than `ArtDmx` on the port it sends `ArtDmx` on.
#[test]
fn another_opcode_is_no_refusal() {
    let mut bytes = packet(0, 1, &[0, 0]);
    bytes[8..10].copy_from_slice(&0x2100u16.to_le_bytes());
    assert_eq!(
        parse(&bytes, source()),
        Ok(Packet::Other { opcode: 0x2100 })
    );
}

/// An `ArtPoll` packet, in the documented layout. It takes 14 bytes, under
/// the 18 an `ArtDmx` header takes.
fn poll(talk_to_me: u8, priority: u8) -> Vec<u8> {
    let mut bytes = b"Art-Net\0".to_vec();
    bytes.extend_from_slice(&0x2000u16.to_le_bytes());
    bytes.extend_from_slice(&14u16.to_be_bytes());
    bytes.push(talk_to_me);
    bytes.push(priority);
    bytes
}

/// A poll is 4 bytes shorter than an `ArtDmx` header, and it must still
/// parse: a desk that is answered nothing lists the node nowhere.
#[test]
fn a_poll_carries_what_the_desk_asks_for() {
    let bytes = poll(0x02, 0x10);
    assert_eq!(bytes.len(), 14);
    match parse(&bytes, source()) {
        Ok(Packet::Poll(poll)) => {
            assert_eq!(poll.talk_to_me, 0x02);
            assert_eq!(poll.priority, 0x10);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_poll_under_its_own_length_is_dropped() {
    let mut bytes = poll(0, 0);
    bytes.truncate(13);
    assert_eq!(
        refusal(&bytes),
        Error::TooShort {
            carried: 13,
            need: 14
        }
    );
}

fn identity() -> Identity<'static> {
    Identity {
        ip: Ipv4Addr::new(192, 0, 2, 7),
        name: "govee-toolkit",
    }
}

/// What a desk reads out of one reply.
fn reply_of(universes: &[u16], index: usize) -> [u8; REPLY] {
    let built = replies(&identity(), universes);
    built.get(index).copied().unwrap_or([0u8; REPLY])
}

/// A desk matches the identifier, the opcode and the address it must send
/// `ArtDmx` to.
#[test]
fn a_reply_names_the_node_and_the_address_to_send_to() {
    let reply = reply_of(&[0], 0);
    assert_eq!(reply.len(), 239);
    assert_eq!(&reply[0..8], b"Art-Net\0");
    assert_eq!(&reply[8..10], 0x2100u16.to_le_bytes());
    assert_eq!(&reply[10..14], [192, 0, 2, 7]);
    assert_eq!(&reply[14..16], 6454u16.to_le_bytes());
    assert_eq!(&reply[26..39], b"govee-toolkit");
    assert_eq!(reply[39], 0, "the name is null-terminated");
    assert_eq!(&reply[44..57], b"govee-toolkit");
    assert_eq!(reply[200], 0, "a node, not a desk");
    assert_eq!(reply[211], 1, "the first reply binds at 1");
}

/// One port-address is one output port, and the desk reads the Net, the
/// Sub-Net and the Universe apart.
#[test]
fn one_port_address_is_one_output_port() {
    let reply = reply_of(&[0x0113], 0);
    assert_eq!(reply[18], 0x01, "the Net");
    assert_eq!(reply[19], 0x01, "the Sub-Net");
    assert_eq!(&reply[172..174], [0, 1], "one port");
    assert_eq!(reply[174], 0x80, "the port outputs DMX from Art-Net");
    assert_eq!(reply[182], 0x80, "the port is transmitting");
    assert_eq!(reply[190], 0x03, "the Universe");
}

/// One reply carries 4 ports, so 6 port-addresses take 2 replies and the
/// second one binds at 2.
#[test]
fn five_port_addresses_take_two_replies() {
    let built = replies(&identity(), &[0, 1, 2, 3, 4, 5]);
    assert_eq!(built.len(), 2);
    assert_eq!(built[0][172..174], [0, 4]);
    assert_eq!(built[0][190..194], [0, 1, 2, 3]);
    assert_eq!(built[0][211], 1);
    assert_eq!(built[1][172..174], [0, 2]);
    assert_eq!(built[1][190..194], [4, 5, 0, 0]);
    assert_eq!(built[1][211], 2);
}

/// The 4 ports of one reply share a Net and a Sub-Net, so two Sub-Nets take
/// two replies however few ports each one holds.
#[test]
fn two_sub_nets_take_two_replies() {
    let built = replies(&identity(), &[0x0000, 0x0010]);
    assert_eq!(built.len(), 2);
    assert_eq!(built[0][19], 0x00);
    assert_eq!(built[1][19], 0x01);
}

/// A rig on no port-address still answers: the desk must list the node.
#[test]
fn a_rig_on_no_port_address_answers_once() {
    let built = replies(&identity(), &[]);
    assert_eq!(built.len(), 1);
    assert_eq!(built[0][172..174], [0, 0]);
}

/// A name over the 18 bytes the short field holds is cut, and the field stays
/// null-terminated.
#[test]
fn a_long_name_is_cut_to_fit() {
    let long = "a-node-name-that-runs-well-past-eighteen-bytes";
    let built = replies(
        &Identity {
            ip: Ipv4Addr::LOCALHOST,
            name: long,
        },
        &[0],
    );
    assert_eq!(&built[0][26..43], &long.as_bytes()[..17]);
    assert_eq!(built[0][43], 0);
}

/// UDP delivers out of order, and the older packet must not overwrite the
/// newer one.
#[test]
fn a_packet_delivered_late_is_dropped() {
    let mut gate = Sequence::new();
    let new = dmx(&packet(0, 10, &[200, 0]));
    let late = dmx(&packet(0, 9, &[1, 0]));
    assert!(gate.accept(new.sequence));
    assert!(!gate.accept(late.sequence));
}

/// What one capture says the sender showed.
#[derive(serde::Deserialize)]
struct Expected {
    sender: String,
    universe: u16,
    sequence: u8,
    physical: u8,
    length: u16,
    channels: BTreeMap<u16, u8>,
}

fn capture_dir() -> PathBuf {
    // crates/dmx -> crates -> rust -> packages -> the repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("tests/fixtures/artnet")
}

/// Every capture under `tests/fixtures/artnet/` parses to the channels the
/// sender showed. The directory carries no capture yet, and the test is what
/// the first one lands against.
#[test]
fn every_capture_produces_the_channels_the_sender_showed() {
    let dir = capture_dir();
    let entries = std::fs::read_dir(&dir).expect("the capture directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "bin") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("the capture");
        let text = std::fs::read_to_string(path.with_extension("json"))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let expected: Expected =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = format!("{} ({})", path.display(), expected.sender);
        let dmx = match parse(&bytes, source()) {
            Ok(Packet::Dmx(dmx)) => dmx,
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(dmx.frame.universe, expected.universe, "{name}");
        assert_eq!(dmx.sequence, expected.sequence, "{name}");
        assert_eq!(dmx.physical, expected.physical, "{name}");
        assert_eq!(dmx.frame.len(), expected.length, "{name}");
        for (address, value) in expected.channels {
            assert_eq!(
                dmx.frame.slot(address),
                Some(value),
                "{name} channel {address}"
            );
        }
    }
}
