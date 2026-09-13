//! The device's mutable state: what it received, what it answers, and the
//! encoded-link handshake. Split from [`device`](super::device) for length.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::encode::{self, CONFIRM, HANDSHAKE, REQUEST};
use super::{BleFaults, FRAME_LEN, MULTI_WRITE, READ, TRANSFER_END, WRITE, bcc};

#[derive(Debug)]
pub(super) struct State {
    pub(super) faults: BleFaults,
    pub(super) connected: bool,
    pub(super) received: Vec<Vec<u8>>,
    pub(super) answers: BTreeMap<u8, Vec<u8>>,
    /// The session seed, once the handshake has run. `None` on a plaintext
    /// device, and on an encoded one until the handshake.
    pub(super) session: Option<[u8; 16]>,
    /// How many session seeds have been handed out, so each is distinct.
    pub(super) sessions: u8,
    /// When each write of the current burst window arrived.
    pub(super) writes: Vec<Instant>,
    /// While this is in the future, the firmware answers nothing.
    pub(super) unresponsive_until: Option<Instant>,
    pub(super) stalls: u32,
    pub(super) sent: u32,
    /// The status a chunked transfer is acknowledged with. `0` accepts.
    pub(super) transfer_status: u8,
}
impl State {
    /// One frame on an encoded link, encoded or not.
    ///
    /// Before the handshake, the frame is decoded under the base seed: a
    /// session seed request is answered with a seed, and anything else is
    /// recorded as it came and answered with silence, as a plaintext frame on
    /// such a device is. After it, the frame is decoded under the session seed,
    /// a confirmation is echoed back, and a command is handled as on a
    /// plaintext device, its answer encoded the same way.
    pub(super) fn encoded(&mut self, frame: &[u8], now: Instant) -> Option<(Vec<u8>, Duration)> {
        let under_base = encode::apply(&encode::q(), frame, false);
        if let [HANDSHAKE, REQUEST, ..] = under_base.as_slice()
            && under_base.len() == FRAME_LEN
        {
            // A silent device hands no seed out either.
            if self.faults.silent {
                return None;
            }
            self.sessions = self.sessions.wrapping_add(1);
            let mut seed = [0u8; 16];
            for (i, byte) in (0u8..=u8::MAX).zip(seed.iter_mut()) {
                *byte = i ^ self.sessions;
            }
            self.session = Some(seed);
            let mut answer = vec![HANDSHAKE, REQUEST];
            answer.extend_from_slice(&seed);
            answer.resize(FRAME_LEN, 0);
            let sum = bcc(&answer);
            if let Some(last) = answer.last_mut() {
                *last = sum;
            }
            return Some((
                encode::apply(&encode::q(), &answer, true),
                self.faults.latency,
            ));
        }
        if let [HANDSHAKE, CONFIRM, ..] = under_base.as_slice()
            && under_base.len() == FRAME_LEN
            && self.session.is_some()
        {
            return (!self.faults.silent).then(|| (frame.to_vec(), self.faults.latency));
        }
        let Some(seed) = self.session else {
            self.received.push(frame.to_vec());
            return None;
        };
        let plain = encode::apply(&seed, frame, false);
        if check(&plain).is_err() {
            self.received.push(frame.to_vec());
            return None;
        }
        self.received.push(plain.clone());
        self.burst(now);
        let (answer, latency) = self.answer(&plain, now)?;
        Some((encode::apply(&seed, &answer, true), latency))
    }

    /// Count this write, and stall the firmware if the burst is too fast.
    pub(super) fn burst(&mut self, now: Instant) {
        let Some(stall) = self.faults.stall else {
            return;
        };
        self.writes
            .retain(|at| now.duration_since(*at) < stall.within);
        self.writes.push(now);
        let over = u32::try_from(self.writes.len()).unwrap_or(u32::MAX) > stall.after;
        if over && self.unresponsive_until.is_none_or(|until| until <= now) {
            self.unresponsive_until = Some(now + stall.lasts);
            self.stalls = self.stalls.wrapping_add(1);
        }
    }

    /// The frame this one is answered with, and how late, or `None` for
    /// silence.
    pub(super) fn answer(&mut self, frame: &[u8], now: Instant) -> Option<(Vec<u8>, Duration)> {
        if self.faults.silent || self.unresponsive_until.is_some_and(|until| until > now) {
            return None;
        }
        let (&kind, &command_type) = (frame.first()?, frame.get(1)?);
        let mut answer = match kind {
            // Acknowledged under the two bytes it came with. See
            // `docs/protocol/ble.md` §1.4.
            WRITE => vec![WRITE, command_type, 0x00],
            // A read the device does not implement answers nothing at all.
            READ => {
                let mut bytes = vec![READ, command_type];
                bytes.extend_from_slice(self.answers.get(&command_type)?);
                bytes
            }
            // A multi-packet write is acknowledged once, on the frame that
            // closes it. Nothing here reassembles the body.
            MULTI_WRITE if frame.get(2) == Some(&TRANSFER_END) => {
                vec![MULTI_WRITE, command_type, self.transfer_status]
            }
            _ => return None,
        };
        answer.resize(FRAME_LEN, 0);
        let sum = bcc(&answer);
        if let Some(last) = answer.last_mut() {
            *last = sum;
        }

        self.sent = self.sent.wrapping_add(1);
        let dropped = crate::drops(self.faults.drop_one_in, self.sent);
        (!dropped).then_some((answer, self.faults.latency))
    }
}

/// Refuse a frame the wire cannot carry.
pub(super) fn check(frame: &[u8]) -> std::io::Result<()> {
    if frame.len() != FRAME_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "the frame is {} bytes; this wire carries {FRAME_LEN}",
                frame.len()
            ),
        ));
    }
    let sum = bcc(frame);
    if frame.last() != Some(&sum) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("the frame's BCC is {:?}, not {sum:#04x}", frame.last()),
        ));
    }
    Ok(())
}
