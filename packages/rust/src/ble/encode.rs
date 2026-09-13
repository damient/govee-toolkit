//! The codec of an encoded link, and the handshake frames. No I/O.
//!
//! A device whose advertisement carries the encoding flag ([`Beacon`]) takes
//! no plaintext frame of `proType` `0x33` or `0xAA`: it answers nothing.
//! Every frame on such a link is encoded under a seed, and the seed changes
//! once per connection. `docs/protocol/ble.md` 9 describes the exchange.
//!
//! An encoded frame keeps its length. The XOR checksum of 1.2 is computed on
//! the plaintext, before the encoding.
//!
//! [`Beacon`]: super::scan::Beacon

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt as _, BlockEncrypt as _, KeyInit as _};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

/// The 16 bytes that parameterize a codec.
pub type Seed = [u8; 16];

// Do not rename, reorder or join these four parts. Their order is the seed.
const Q1: &str = "TWFraW";
const Q2: &str = "5nTGlm";
const Q3: &str = "ZVNtYX";
const Q4: &str = "J0ZQ==";

// Do not inline this function, and do not replace it with a literal. A wrong
// seed decodes no frame, and the device answers nothing.
fn q() -> Seed {
    let mut k = Seed::default();
    if let Ok(d) = BASE64.decode([Q1, Q2, Q3, Q4].concat()) {
        for (o, i) in k.iter_mut().zip(d) {
            *o = i;
        }
    }
    k
}

/// The `proType` of a handshake frame.
pub const HANDSHAKE: u8 = 0xE7;

/// The handshake frame that asks for a session seed.
pub const REQUEST: u8 = 0x01;

/// The handshake frame that confirms the session seed. The device echoes it.
pub const CONFIRM: u8 = 0x02;

/// How many bytes of a 20-byte frame the block stage covers.
const BLOCK: usize = 16;

/// The length of a handshake frame, in bytes. The same length as every
/// other frame on this wire.
const FRAME_LEN: usize = super::FRAME_LEN;

/// One seed, ready to encode and decode frames.
#[derive(Clone)]
pub struct Codec {
    aes: Aes128,
    seed: Seed,
}

impl std::fmt::Debug for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A seed is not for a log.
        f.debug_struct("Codec").finish_non_exhaustive()
    }
}

impl Codec {
    /// A codec under `seed`.
    #[must_use]
    pub fn new(seed: Seed) -> Self {
        Self {
            aes: Aes128::new(GenericArray::from_slice(&seed)),
            seed,
        }
    }

    /// The codec every handshake runs under.
    ///
    /// Its seed is the same for every device. It covers the handshake alone:
    /// what follows runs under the session seed the device hands back. Anyone
    /// who holds that seed and hears the handshake reads the session, so it is
    /// a fence against a casual listener and nothing more.
    #[must_use]
    pub fn base() -> Self {
        Self::new(q())
    }

    /// Encode a plaintext frame. Same length out as in.
    #[must_use]
    pub fn encode(&self, plain: &[u8]) -> Vec<u8> {
        self.apply(plain, true)
    }

    /// Decode an encoded frame. Same length out as in.
    #[must_use]
    pub fn decode(&self, coded: &[u8]) -> Vec<u8> {
        self.apply(coded, false)
    }

    fn apply(&self, frame: &[u8], encode: bool) -> Vec<u8> {
        let mut out = Vec::with_capacity(frame.len());
        let (blocks, tail) = frame.as_chunks::<BLOCK>();
        for chunk in blocks {
            let mut block = GenericArray::from(*chunk);
            if encode {
                self.aes.encrypt_block(&mut block);
            } else {
                self.aes.decrypt_block(&mut block);
            }
            out.extend_from_slice(&block);
        }
        out.extend(stream(&self.seed, tail));
        out
    }
}

/// The stream stage over `data`, from a fresh state built from `seed`. It is
/// its own inverse.
// Every index below is a `u8` widened into a 256-entry table, or a position
// below `seed.len()`, so none of them can be out of range.
#[allow(clippy::indexing_slicing)]
fn stream(seed: &Seed, data: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = [0; 256];
    for (i, v) in (0u8..=u8::MAX).zip(s.iter_mut()) {
        *v = i;
    }
    let mut j: u8 = 0;
    for (i, &k) in seed.iter().cycle().take(256).enumerate() {
        j = j.wrapping_add(s[i]).wrapping_add(k);
        s.swap(i, usize::from(j));
    }
    let (mut i, mut j) = (0u8, 0u8);
    data.iter()
        .map(|&byte| {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[usize::from(i)]);
            s.swap(usize::from(i), usize::from(j));
            let k = s[usize::from(s[usize::from(i)].wrapping_add(s[usize::from(j)]))];
            k ^ byte
        })
        .collect()
}

/// A handshake frame: the header, the command, noise up to the checksum,
/// and the XOR of everything before it.
///
/// The noise carries nothing. It is there so that two handshakes do not
/// encode to the same bytes.
fn handshake(command: u8, noise: &mut impl FnMut() -> u8) -> [u8; FRAME_LEN] {
    let mut frame = [0u8; FRAME_LEN];
    if let Some((checksum, body)) = frame.split_last_mut() {
        let mut bytes = body.iter_mut();
        if let Some(byte) = bytes.next() {
            *byte = HANDSHAKE;
        }
        if let Some(byte) = bytes.next() {
            *byte = command;
        }
        for byte in bytes {
            *byte = noise();
        }
        *checksum = xor(body);
    }
    frame
}

/// The plaintext of the frame that asks for a session seed.
pub fn request(noise: &mut impl FnMut() -> u8) -> [u8; FRAME_LEN] {
    handshake(REQUEST, noise)
}

/// The plaintext of the frame that confirms the session seed.
pub fn confirm(noise: &mut impl FnMut() -> u8) -> [u8; FRAME_LEN] {
    handshake(CONFIRM, noise)
}

/// The session seed a decoded reply carries, or `None` if the reply is not
/// the answer to [`request`].
#[must_use]
pub fn session_seed(plain: &[u8]) -> Option<Seed> {
    let [HANDSHAKE, REQUEST, rest @ ..] = plain else {
        return None;
    };
    if plain.len() != FRAME_LEN {
        return None;
    }
    rest.get(..BLOCK)?.try_into().ok()
}

/// Whether a decoded reply acknowledges [`confirm`].
#[must_use]
pub fn is_confirm(plain: &[u8]) -> bool {
    plain.len() == FRAME_LEN && matches!(plain, [HANDSHAKE, CONFIRM, ..])
}

fn xor(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |acc, b| acc ^ b)
}

/// Bytes for the noise of a handshake frame. Not a secure source, and it does
/// not need to be: the noise is padding.
#[derive(Debug)]
pub struct Noise(u64);

impl Noise {
    /// A generator seeded from the clock and the process.
    #[must_use]
    pub fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        // The low 64 bits of the clock. Any non-zero seed will do.
        let clock = u64::try_from(nanos & u128::from(u64::MAX)).unwrap_or(1);
        Self(clock ^ (u64::from(std::process::id()) << 32) | 1)
    }

    /// The next byte.
    pub fn next_byte(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        // The high byte of the state, so the shift leaves at most 8 bits.
        u8::try_from(self.0 >> 56).unwrap_or(0)
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// One handshake as a bulb answered it. The wire bytes come from
    /// `tests/fixtures/ble-captures/H6008/encoded.txt`.
    #[test]
    fn encodes_the_request_the_way_the_device_decodes_it() {
        let plain = hex("e7 01 40 3c 92 24 d5 0c 2f 34 5c 71 ee 3b 8a 91 93 48 0d db");
        let wire = hex("62 50 d9 65 7e 67 28 e3 d9 84 b1 78 90 23 de 6e fb a6 4d 84");
        assert_eq!(Codec::base().encode(&plain), wire);
        assert_eq!(Codec::base().decode(&wire), plain);
    }

    #[test]
    fn reads_the_session_seed_out_of_the_answer() {
        let wire = hex("86 de b2 b9 88 89 88 3f cd 9f 73 9b d9 08 8c c8 23 97 4d e8");
        let plain = Codec::base().decode(&wire);
        assert_eq!(
            session_seed(&plain),
            Some(*b"\xd1\xff\x2c\x59\x86\xb4\xe1\x0e\x3c\x69\x96\xc4\xf1\x1e\x4b\x79")
        );
        assert!(!is_confirm(&plain));
    }

    #[test]
    fn a_command_under_the_session_seed_matches_the_capture() {
        let session =
            Codec::new(*b"\xd1\xff\x2c\x59\x86\xb4\xe1\x0e\x3c\x69\x96\xc4\xf1\x1e\x4b\x79");
        let plain = hex("33 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 32");
        let wire = hex("56 09 fb 54 ae 00 14 77 61 7a 47 b3 4b 10 11 0f 46 03 93 b6");
        assert_eq!(session.encode(&plain), wire);
        assert_eq!(session.decode(&wire), plain);
    }

    /// A frame shorter than one block takes the stream stage alone. The
    /// channel probe of `docs/protocol/ble.md` 8 answers five bytes, encoded
    /// under the base seed.
    #[test]
    fn a_short_frame_takes_the_stream_stage_alone() {
        let wire = hex("cd ec 50 5e b7");
        assert_eq!(Codec::base().decode(&wire), hex("a5 02 10 01 b6"));
    }

    #[test]
    fn a_handshake_frame_carries_its_checksum() {
        let mut noise = Noise::new();
        let frame = request(&mut || noise.next_byte());
        assert_eq!(frame[0], HANDSHAKE);
        assert_eq!(frame[1], REQUEST);
        assert_eq!(frame[19], xor(&frame[..19]));
        let frame = confirm(&mut || noise.next_byte());
        assert!(is_confirm(&frame));
        assert_eq!(session_seed(&frame), None);
    }

    #[test]
    fn a_reply_of_another_shape_carries_no_session_seed() {
        assert_eq!(session_seed(&[HANDSHAKE, REQUEST]), None);
        assert_eq!(session_seed(&[0x33; 20]), None);
        assert!(!is_confirm(&[HANDSHAKE, CONFIRM]));
    }

    #[test]
    fn the_codec_does_not_print_its_seed() {
        assert_eq!(format!("{:?}", Codec::base()), "Codec { .. }");
    }
}
