//! Reads one argument and turns it into the bytes of one field.

use crate::codec::args::{self, ArgValue, Args};
use crate::codec::error::{Error, Result};

pub(super) fn write_len(out: &mut [u8], pos: usize, len: usize) {
    let len = u16::try_from(len).unwrap_or(u16::MAX).to_be_bytes();
    if let Some(slot) = out.get_mut(pos..pos + 2) {
        slot.copy_from_slice(&len);
    }
}

pub(super) fn push_int(
    out: &mut Vec<u8>,
    command: &str,
    name: &str,
    value: i64,
    bits: u32,
) -> Result<()> {
    let too_wide = || Error::FrameWidth {
        command: command.to_owned(),
        arg: name.to_owned(),
        value,
        bits,
    };
    match bits {
        8 => out.push(u8::try_from(value).map_err(|_| too_wide())?),
        16 => out.extend_from_slice(&u16::try_from(value).map_err(|_| too_wide())?.to_be_bytes()),
        _ => return Err(too_wide()),
    }
    Ok(())
}

/// Zone indices as a bitmask, least significant bit first.
///
/// A zone the width cannot carry is refused: the firmware drops those bits in
/// silence, and a saturated mask looks exactly like an ignored one.
pub(super) fn mask(command: &str, zones: &[u16], name: &str, width: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; width];
    let bits = width * 8;
    for zone in zones {
        let index = usize::from(*zone);
        let Some(slot) = (index < bits).then(|| out.get_mut(index / 8)).flatten() else {
            return Err(Error::OutOfRange {
                command: command.to_owned(),
                arg: name.to_owned(),
                value: i64::from(*zone),
                min: 0,
                max: i64::try_from(bits).unwrap_or(i64::MAX) - 1,
            });
        };
        *slot |= 1u8 << (index % 8);
    }
    Ok(out)
}

/// Zero-fill up to `size`, minus the byte the checksum takes.
pub(super) fn pad(out: &mut Vec<u8>, command: &str, size: usize, has_xor: bool) -> Result<()> {
    let room = size - usize::from(has_xor);
    if out.len() > room {
        return Err(Error::FrameOverflow {
            command: command.to_owned(),
            size,
            actual: out.len() + usize::from(has_xor),
        });
    }
    out.resize(room, 0);
    Ok(())
}

pub(super) fn too_long(command: &str, name: &str, len: usize, max: usize) -> Error {
    Error::FieldTooLong {
        command: command.to_owned(),
        arg: name.to_owned(),
        len,
        max,
    }
}

fn missing(command: &str, name: &str) -> Error {
    Error::MissingArg {
        command: command.to_owned(),
        arg: name.to_owned(),
    }
}

fn wrong_type(command: &str, name: &str, expected: &'static str, got: &ArgValue) -> Error {
    Error::ArgType {
        command: command.to_owned(),
        arg: name.to_owned(),
        expected,
        got: got.type_name(),
    }
}

pub(super) fn int_arg(command: &str, args: &Args, name: &str) -> Result<i64> {
    match args.get(name) {
        Some(ArgValue::Int(v)) => Ok(*v),
        Some(other) => Err(wrong_type(command, name, args::INT, other)),
        None => Err(missing(command, name)),
    }
}

/// The borrowing extractors differ only in the variant they accept and the
/// type name the error reports.
macro_rules! arg_getter {
    ($name:ident, $variant:ident, $expected:path, $ret:ty, $out:expr) => {
        pub(super) fn $name<'a>(command: &str, args: &'a Args, name: &str) -> Result<$ret> {
            match args.get(name) {
                Some(ArgValue::$variant(v)) => Ok($out(v)),
                Some(other) => Err(wrong_type(command, name, $expected, other)),
                None => Err(missing(command, name)),
            }
        }
    };
}

arg_getter!(rgb_arg, Rgb, args::RGB_LIST, &'a [[u8; 3]], Vec::as_slice);
arg_getter!(text_arg, Text, args::TEXT, &'a str, String::as_str);
arg_getter!(zones_arg, Zones, args::ZONES, &'a [u16], Vec::as_slice);
arg_getter!(bytes_arg, Bytes, args::BYTES, &'a [u8], Vec::as_slice);
