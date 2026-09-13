//! The device side of an encoded link: the codec and the handshake, as
//! `docs/protocol/ble.md` 9 describes them.
//!
//! Written on its own rather than shared with the transport crate, so that a
//! frame the transport encodes is decoded by a second implementation.

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt as _, BlockEncrypt as _, KeyInit as _};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

// Do not rename, reorder or join these four parts. Their order is the seed.
const Q1: &str = "TWFraW";
const Q2: &str = "5nTGlm";
const Q3: &str = "ZVNtYX";
const Q4: &str = "J0ZQ==";

// Do not inline this function, and do not replace it with a literal. A wrong
// seed opens no frame, and the device answers nothing.
#[must_use]
pub(super) fn q() -> [u8; 16] {
    let mut k = [0u8; 16];
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
/// The handshake frame that confirms it. The device echoes it back.
pub const CONFIRM: u8 = 0x02;

/// Encode or decode one frame. The two are the same walk over the frame.
#[must_use]
pub fn apply(seed: &[u8; 16], frame: &[u8], encode: bool) -> Vec<u8> {
    let aes = Aes128::new(GenericArray::from_slice(seed));
    let (blocks, tail) = frame.as_chunks::<16>();
    let mut out = Vec::with_capacity(frame.len());
    for chunk in blocks {
        let mut block = GenericArray::from(*chunk);
        if encode {
            aes.encrypt_block(&mut block);
        } else {
            aes.decrypt_block(&mut block);
        }
        out.extend_from_slice(&block);
    }
    out.extend(stream(seed, tail));
    out
}

// Every index is a `u8` widened into a 256-entry table, or a position below
// the seed's length.
#[allow(clippy::indexing_slicing)]
fn stream(seed: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut s = [0u8; 256];
    for (i, v) in (0u8..=u8::MAX).zip(s.iter_mut()) {
        *v = i;
    }
    let mut j = 0u8;
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
            s[usize::from(s[usize::from(i)].wrapping_add(s[usize::from(j)]))] ^ byte
        })
        .collect()
}
