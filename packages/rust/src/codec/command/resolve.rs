//! Fills in derivable arguments, checks declared ones, and substitutes them
//! into the `payload:` template.

use std::collections::BTreeSet;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::codec::args::{ArgValue, Args};
use crate::codec::catalog::{ArgSpec, Command};
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;

/// Fill in derivable arguments, then validate every declared one.
///
/// `sends` are the layouts that go out: a repeat count comes from the list it
/// counts there. `captured` names the arguments a `reply:` reads back; the
/// device fills those in, so the caller supplies none.
pub(super) fn resolve(
    command: &str,
    spec: &Command,
    sends: &[&Frame],
    captured: &BTreeSet<&str>,
    args: &Args,
) -> Result<Args> {
    for name in args.names() {
        if !spec.args.contains_key(name) {
            return Err(Error::UnknownArg {
                command: command.to_owned(),
                arg: name.to_owned(),
            });
        }
    }

    // A repeat count is redundant with the length of the list it counts: derive
    // it when the caller supplied none, and refuse the two when they disagree.
    let mut derived: Vec<(String, i64)> = Vec::new();
    for frame in sends {
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
        let Some(value) = supplied else {
            if captured.contains(name.as_str()) {
                continue;
            }
            return Err(Error::MissingArg {
                command: command.to_owned(),
                arg: name.clone(),
            });
        };

        check(command, name, arg_spec, &value)?;
        out.insert(name.clone(), value);
    }
    Ok(out)
}

/// One value against what the device file declares for it.
fn check(command: &str, name: &str, spec: &ArgSpec, value: &ArgValue) -> Result<()> {
    match (spec, value) {
        (ArgSpec::Int { range, .. }, ArgValue::Int(v)) => {
            let (min, max) = (range[0], range[1]);
            if *v < min || *v > max {
                return Err(Error::OutOfRange {
                    command: command.to_owned(),
                    arg: name.to_owned(),
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
                        arg: name.to_owned(),
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
                arg: name.to_owned(),
                expected: spec.type_name(),
                got: got.type_name(),
            });
        }
    }
    Ok(())
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
pub(super) fn substitute(
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
