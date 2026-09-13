//! `send`: one device file entry, named as the file names it. The mode is
//! resolved before the values are read, because the same entry name can
//! declare different arguments on two modes.
//!
//! An entry that declares a `reply:` is read, and what the layout captured is
//! printed under the names the device file gives the fields.

use std::collections::BTreeMap;

use govee_toolkit::codec::{ArgSpec, Args, Mode};
use govee_toolkit::{DeviceId, Govee};
use serde_json::json;

use crate::output::{Failure, Writer};
use crate::run::args;
use crate::run::verbs::report;

pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    command: &str,
    pairs: &[String],
) -> Result<(), Failure> {
    let handle = govee.device(id);
    let mode = handle.serving_mode()?;
    let device = handle.spec()?;

    // An entry this mode does not carry goes out with no arguments, so that
    // the codec reports the unknown command rather than an unknown argument
    // of it.
    let spec = device.commands.get(mode).get(command);
    let values = match spec {
        Some(spec) => read(&spec.args, mode, command, pairs)?,
        None => Args::new(),
    };

    let answers = spec.is_some_and(|spec| {
        spec.reply.is_some() || !spec.frames.is_empty() || !spec.reads.is_empty()
    });
    if answers {
        let reply = handle.read(command, &values).await?;
        let fields = reply.fields.to_json();
        let text = fields
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .unwrap_or_default();
        writer.emit(
            &json!({
                "id": reply.id.to_string(),
                "mode": mode.to_string(),
                "command": command,
                "fields": fields,
            }),
            &format!("{}  {mode}  {command}  {text}", reply.id),
        );
        return Ok(());
    }

    let served = handle.send(command, &values).await?;
    report(writer, &served);
    Ok(())
}

fn read(
    declared: &BTreeMap<String, ArgSpec>,
    mode: Mode,
    command: &str,
    pairs: &[String],
) -> Result<Args, Failure> {
    let mut values = Args::new();
    for pair in pairs {
        let (name, text) = pair
            .split_once('=')
            .ok_or_else(|| Failure::usage(format!("`{pair}` is not `name=value`")))?;
        let spec = declared.get(name).ok_or_else(|| {
            Failure::usage(format!(
                "`commands.{mode}.{command}` declares no argument `{name}`; it declares {}",
                names(declared)
            ))
        })?;
        values.insert(name, args::parse(name, spec, text)?);
    }
    Ok(values)
}

fn names(declared: &BTreeMap<String, ArgSpec>) -> String {
    if declared.is_empty() {
        return "none".to_owned();
    }
    declared
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
