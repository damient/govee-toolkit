//! `send`: one device file entry, named as the file names it. The crate
//! resolves the mode and reads every value under the type the entry declares
//! for it — see [`govee_toolkit::DeviceHandle::args`].
//!
//! An entry that declares a `reply:` is read, and what the layout captured is
//! printed under the names the device file gives the fields.

use govee_toolkit::codec::{self, Command, Supplied};
use govee_toolkit::{DeviceId, Error, Govee};
use serde_json::json;

use crate::output::{Failure, Writer};
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
    let values = handle.args(command, supplied(pairs)?).map_err(usage)?;

    if device
        .commands
        .get(mode)
        .get(command)
        .is_some_and(Command::answers)
    {
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

/// Each `name=value` as the shape a person typed: text, which the crate reads
/// under the declared type.
fn supplied(pairs: &[String]) -> Result<Vec<(String, Supplied)>, Failure> {
    pairs
        .iter()
        .map(|pair| {
            let (name, text) = pair
                .split_once('=')
                .ok_or_else(|| Failure::usage(format!("`{pair}` is not `name=value`")))?;
            Ok((name.to_owned(), Supplied::Text(text.to_owned())))
        })
        .collect()
}

/// An argument the entry does not declare is a mistake on the command line,
/// so it exits with the usage code rather than the refusal one.
fn usage(error: Error) -> Failure {
    if matches!(&error, Error::Codec(codec::Error::UnknownArg { .. })) {
        return Failure::usage(error.to_string());
    }
    Failure::from(error)
}
