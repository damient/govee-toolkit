//! The parser, checked against real packets.
//!
//! A packet built in [`super::tests`] is built from the layout `docs/dmx.md`
//! documents, and is not evidence of what a sender puts on the wire. The
//! captures under `tests/fixtures/artnet/` are.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use super::{Packet, parse};

fn source() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(192, 0, 2, 2), 6454))
}

/// What one `ArtDmx` capture says the sender showed.
#[derive(serde::Deserialize)]
struct Expected {
    sender: String,
    universe: u16,
    sequence: u8,
    physical: u8,
    length: u16,
    channels: BTreeMap<u16, u8>,
}

/// What one `ArtPoll` capture carries.
#[derive(serde::Deserialize)]
struct Polled {
    sender: String,
    talk_to_me: u8,
    priority: u8,
}

fn capture_dir() -> PathBuf {
    // crates/dmx -> crates -> rust -> packages -> the repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("tests/fixtures/artnet")
}

/// Every `<name>.bin` in `dir`, with the `<name>.json` beside it read into `T`.
fn captures<T: serde::de::DeserializeOwned>(dir: &Path) -> Vec<(String, Vec<u8>, T)> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let mut captures = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "bin") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let text = std::fs::read_to_string(path.with_extension("json"))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let expected = serde_json::from_str(&text)
            .unwrap_or_else(|e: serde_json::Error| panic!("{}: {e}", path.display()));
        captures.push((path.display().to_string(), bytes, expected));
    }
    captures
}

#[test]
fn every_capture_produces_the_channels_the_sender_showed() {
    for (path, bytes, expected) in captures::<Expected>(&capture_dir()) {
        let name = format!("{path} ({})", expected.sender);
        let dmx = match parse(&bytes, source()) {
            Ok(Packet::Dmx(dmx)) => dmx,
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(dmx.frame.universe, expected.universe, "{name}");
        assert_eq!(dmx.sequence, expected.sequence, "{name}");
        assert_eq!(dmx.physical, expected.physical, "{name}");
        assert_eq!(dmx.frame.len(), expected.length, "{name}");
        assert!(!expected.channels.is_empty(), "{name}: no channel is read");
        for (address, value) in expected.channels {
            assert_eq!(
                dmx.frame.slot(address),
                Some(value),
                "{name} channel {address}"
            );
        }
    }
}

#[test]
fn every_poll_capture_reads_as_a_poll() {
    for (path, bytes, expected) in captures::<Polled>(&capture_dir().join("poll")) {
        let name = format!("{path} ({})", expected.sender);
        let poll = match parse(&bytes, source()) {
            Ok(Packet::Poll(poll)) => poll,
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(poll.talk_to_me, expected.talk_to_me, "{name}");
        assert_eq!(poll.priority, expected.priority, "{name}");
    }
}
