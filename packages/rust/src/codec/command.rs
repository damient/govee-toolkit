//! Turning a device file entry plus arguments into something sendable.
//!
//! Nothing here knows a SKU or a command name: the device file supplies the
//! `cmd`, the `payload` template and the `frame` layout, and this module only
//! validates and substitutes.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::codec::args::{ArgValue, Args};
use crate::codec::catalog::{ArgSpec, Command, Device, Mode};
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;

/// A command ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The value carried in `msg.cmd`.
    pub cmd: String,
    /// The whole `{"msg":{"cmd":…,"data":…}}` envelope.
    pub message: serde_json::Value,
    /// The raw frame, for commands that declare one. Already base64-encoded
    /// into `message`; exposed for tests and captures.
    pub frame: Option<Vec<u8>>,
}

impl Encoded {
    /// The UDP payload: the envelope, serialized.
    ///
    /// # Errors
    ///
    /// Only if the envelope cannot be serialized, which cannot happen for a
    /// value this crate built.
    pub fn to_bytes(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(&self.message)
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

    let resolved = resolve(command, spec, frame, args)?;
    let bytes = frame.map(|f| f.build(command, &resolved)).transpose()?;
    let data = substitute(command, &spec.payload, &resolved, bytes.as_deref())?;

    Ok(Encoded {
        cmd: spec.cmd.clone(),
        message: serde_json::json!({ "msg": { "cmd": spec.cmd, "data": data } }),
        frame: bytes,
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
            (ArgSpec::Int { range }, ArgValue::Int(v)) => {
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
            (ArgSpec::RgbList { max_len }, ArgValue::Rgb(items)) => {
                if let Some(max) = max_len
                    && items.len() > *max
                {
                    return Err(Error::OutOfRange {
                        command: command.to_owned(),
                        arg: name.clone(),
                        value: i64::try_from(items.len()).unwrap_or(i64::MAX),
                        min: 0,
                        max: i64::try_from(*max).unwrap_or(i64::MAX),
                    });
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
