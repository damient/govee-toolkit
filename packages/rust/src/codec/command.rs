//! Turning a device file entry plus arguments into something sendable.
//!
//! Nothing here knows a SKU or a command name: the device file supplies the
//! `cmd`, the `payload` template and the `frame` layout, and this module only
//! validates and substitutes.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::codec::args::{ArgValue, Args};
use crate::codec::catalog::{ArgSpec, Command, Device, Mode};
use crate::codec::chunk;
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;

/// A command ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The value carried in `msg.cmd`, empty where the wire carries no
    /// envelope.
    pub cmd: String,
    /// The whole `{"msg":{"cmd":…,"data":…}}` envelope. `None` where the mode
    /// puts the frames on the wire with nothing wrapped around them.
    pub message: Option<serde_json::Value>,
    /// The raw frames, in the order they go out: one for a command that
    /// declares a `frame:`, several for a chunked one, none for a command that
    /// travels in its envelope alone. A single frame is already base64-encoded
    /// into `message` when the payload asks for it; the bytes stay here for
    /// tests and captures.
    pub frames: Vec<Vec<u8>>,
}

impl Encoded {
    /// The datagram payload: the envelope, serialized.
    ///
    /// # Errors
    ///
    /// [`Error::NoEnvelope`] for a command that carries none — a caller that
    /// asks for one is on the wrong wire, and a silent empty payload would take
    /// that as far as the device.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let message = self.message.as_ref().ok_or_else(|| Error::NoEnvelope {
            command: self.cmd.clone(),
        })?;
        serde_json::to_vec(message).map_err(|e| Error::Serialize {
            command: self.cmd.clone(),
            reason: e.to_string(),
        })
    }
}

/// Encode one command of one device.
///
/// # Errors
///
/// See [`Error`]. Out-of-range values are rejected, never clamped: the firmware
/// clamps in silence, and hiding that behind a successful call would make the
/// SDK lie about what the device did.
pub fn encode(device: &Device, mode: Mode, command: &str, args: &Args) -> Result<Encoded> {
    use crate::codec::catalog::Support;

    // `Unknown` is deliberately not refused here: nobody probed the mode, so
    // "unsupported" would be a claim. The command table answers instead, and an
    // empty one produces `UnknownCommand`.
    if device.modes.get(mode).support == Support::None {
        return Err(Error::ModeUnsupported {
            sku: device.sku.clone(),
            mode,
        });
    }

    let spec = device
        .commands
        .get(mode)
        .get(command)
        .ok_or_else(|| Error::UnknownCommand {
            sku: device.sku.clone(),
            mode,
            command: command.to_owned(),
        })?;

    let frame = match (spec.frame.as_deref(), spec.parsed_frame.get()) {
        (None, _) => None,
        (Some(_), Some(frame)) => Some(frame),
        (Some(source), None) => {
            let parsed = Frame::parse(command, source)?;
            Some(spec.parsed_frame.get_or_init(|| parsed))
        }
    };
    let chunked = match (spec.body.as_deref(), spec.chunk.as_ref()) {
        (Some(body), Some(chunk)) => Some(chunk::layout(command, body, chunk, &spec.parsed_chunk)?),
        _ => None,
    };

    let resolved = resolve(
        command,
        spec,
        frame.or(chunked.map(chunk::Layout::body)),
        args,
    )?;
    let frames = match (frame, chunked) {
        (Some(frame), _) => vec![frame.build(command, &resolved)?],
        (None, Some(layout)) => layout.build(command, &resolved)?,
        (None, None) => Vec::new(),
    };

    // `${frame}` names one frame, so a chunked command has none to name; the
    // validator refuses a payload that asks for it.
    let single = match (frame.is_some(), frames.first()) {
        (true, Some(bytes)) => Some(bytes.as_slice()),
        _ => None,
    };
    let data = substitute(command, &spec.payload, &resolved, single)?;

    Ok(Encoded {
        cmd: spec.cmd.clone(),
        message: (!spec.cmd.is_empty())
            .then(|| serde_json::json!({ "msg": { "cmd": spec.cmd, "data": data } })),
        frames,
    })
}

/// Fill in derivable arguments, then validate every declared one.
fn resolve(command: &str, spec: &Command, frame: Option<&Frame>, args: &Args) -> Result<Args> {
    for name in args.names() {
        if !spec.args.contains_key(name) {
            return Err(Error::UnknownArg {
                command: command.to_owned(),
                arg: name.to_owned(),
            });
        }
    }

    // A repeat count is redundant with the length of the list it counts: derive
    // it when it was not supplied, and refuse the two disagreeing.
    let mut derived: Vec<(String, i64)> = Vec::new();
    if let Some(frame) = frame {
        for (list, count) in frame.repeat_groups() {
            let Some(ArgValue::Rgb(items)) = args.get(list) else {
                continue;
            };
            let actual = items.len();
            match args.get(count) {
                Some(ArgValue::Int(declared)) => {
                    if usize::try_from(*declared).ok() != Some(actual) {
                        return Err(Error::RepeatCountMismatch {
                            command: command.to_owned(),
                            count_arg: count.to_owned(),
                            list_arg: list.to_owned(),
                            declared: usize::try_from(*declared).unwrap_or(0),
                            actual,
                        });
                    }
                }
                Some(_) => {}
                None => derived.push((count.to_owned(), i64::try_from(actual).unwrap_or(i64::MAX))),
            }
        }
    }

    let mut out = Args::new();
    for (name, arg_spec) in &spec.args {
        let supplied = args.get(name).cloned().or_else(|| {
            derived
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| ArgValue::Int(*v))
        });
        let value = supplied.ok_or_else(|| Error::MissingArg {
            command: command.to_owned(),
            arg: name.clone(),
        })?;

        match (arg_spec, &value) {
            (ArgSpec::Int { range, .. }, ArgValue::Int(v)) => {
                let (min, max) = (range[0], range[1]);
                if *v < min || *v > max {
                    return Err(Error::OutOfRange {
                        command: command.to_owned(),
                        arg: name.clone(),
                        value: *v,
                        min,
                        max,
                    });
                }
            }
            (ArgSpec::RgbList { max_len, .. }, ArgValue::Rgb(items)) => {
                check_len(command, name, items.len(), *max_len)?;
            }
            (ArgSpec::String { max_len, .. }, ArgValue::Text(text)) => {
                check_len(command, name, text.len(), *max_len)?;
            }
            (ArgSpec::Bytes { max_len, .. }, ArgValue::Bytes(bytes)) => {
                check_len(command, name, bytes.len(), *max_len)?;
            }
            (ArgSpec::Zones { count, .. }, ArgValue::Zones(zones)) => {
                if let Some(count) = count {
                    let max = i64::try_from(*count).unwrap_or(i64::MAX) - 1;
                    if let Some(zone) = zones.iter().find(|z| i64::from(**z) > max) {
                        return Err(Error::OutOfRange {
                            command: command.to_owned(),
                            arg: name.clone(),
                            value: i64::from(*zone),
                            min: 0,
                            max,
                        });
                    }
                }
            }
            (spec, got) => {
                return Err(Error::ArgType {
                    command: command.to_owned(),
                    arg: name.clone(),
                    expected: spec.type_name(),
                    got: got.type_name(),
                });
            }
        }
        out.insert(name.clone(), value);
    }
    Ok(out)
}

/// A length against the cap the device file declares, in the units the wire
/// counts: bytes of UTF-8 for a string, items for a list.
fn check_len(command: &str, name: &str, len: usize, max_len: Option<usize>) -> Result<()> {
    let Some(max) = max_len else { return Ok(()) };
    if len > max {
        return Err(Error::OutOfRange {
            command: command.to_owned(),
            arg: name.to_owned(),
            value: i64::try_from(len).unwrap_or(i64::MAX),
            min: 0,
            max: i64::try_from(max).unwrap_or(i64::MAX),
        });
    }
    Ok(())
}

/// Replace `"${name}"` placeholders in the `payload:` template.
///
/// Only a whole string is a placeholder. `${frame}` is reserved: it resolves to
/// the base64 of the frame the command declares.
fn substitute(
    command: &str,
    template: &serde_json::Value,
    args: &Args,
    frame: Option<&[u8]>,
) -> Result<serde_json::Value> {
    use serde_json::Value;

    Ok(match template {
        Value::String(s) => {
            let Some(name) = placeholder(s) else {
                if s.contains("${") {
                    return Err(Error::UnresolvedPlaceholder {
                        command: command.to_owned(),
                        name: s.clone(),
                    });
                }
                return Ok(Value::String(s.clone()));
            };
            match (name, args.get(name)) {
                ("frame", _) if frame.is_some() => {
                    Value::String(BASE64.encode(frame.unwrap_or_default()))
                }
                (_, Some(ArgValue::Int(v))) => Value::from(*v),
                _ => {
                    return Err(Error::UnresolvedPlaceholder {
                        command: command.to_owned(),
                        name: name.to_owned(),
                    });
                }
            }
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| Ok((k.clone(), substitute(command, v, args, frame)?)))
                .collect::<Result<_>>()?,
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| substitute(command, v, args, frame))
                .collect::<Result<_>>()?,
        ),
        other => other.clone(),
    })
}

/// The name inside a whole-string `${name}` placeholder.
///
/// An empty or nested name is not a placeholder; the validator and the encoder
/// share this so a payload cannot pass validation and then fail to encode.
pub(crate) fn placeholder(s: &str) -> Option<&str> {
    let inner = s.strip_prefix("${")?.strip_suffix('}')?;
    (!inner.is_empty() && !inner.contains("${")).then_some(inner)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use crate::codec::Catalog;
    use crate::codec::args::Args;
    use crate::codec::catalog::Mode;

    /// A mode whose frames are a fixed size, with no envelope around them.
    const CHUNKED: &str = r#"
schema_version: 1
sku: HTEST
family: test
name: Test
capabilities: {}
commands:
  ble:
    provision:
      documented: true
      body: "${ssid:str8}"
      chunk:
        size: 16
        header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>"
        data: "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>"
        footer: "A1 <op:11> FF <pad:20> <xor>"
      args:
        ssid: { type: string, max_len: 32 }
"#;

    #[test]
    fn a_chunked_command_encodes_to_a_frame_per_slice_and_no_envelope() {
        let catalog = Catalog::from_sources([("HTEST.yaml", CHUNKED)]).expect("the file parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        let encoded = super::encode(
            device,
            Mode::Ble,
            "provision",
            &Args::new().text("ssid", "Test"),
        )
        .expect("the command encodes");

        assert_eq!(encoded.message, None);
        assert_eq!(encoded.frames.len(), 3);
        assert!(encoded.frames.iter().all(|f| f.len() == 20));
        assert_eq!(encoded.to_bytes().unwrap_err().code(), "no_envelope");
    }

    #[test]
    fn a_string_past_the_cap_the_file_declares_is_refused() {
        let catalog = Catalog::from_sources([("HTEST.yaml", CHUNKED)]).expect("the file parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        let err = super::encode(
            device,
            Mode::Ble,
            "provision",
            &Args::new().text("ssid", "0".repeat(33)),
        )
        .expect_err("the cap is enforced");
        assert_eq!(err.code(), "out_of_range");
    }
}
