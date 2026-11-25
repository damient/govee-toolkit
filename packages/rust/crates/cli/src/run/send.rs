//! `send`: one device file entry, named as the file names it, to every
//! device of the target. For each device, the crate resolves the mode once
//! and reads every value under the type the entry declares for it — see
//! [`govee_toolkit::DeviceHandle::resolve`].
//!
//! An entry that declares a `reply:` is read, and what the layout captured is
//! printed under the names the device file gives the fields.

use govee_toolkit::codec::{self, Mode, Supplied};
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceId, Devices, Error, Invoked, Reply};
use serde_json::json;

use crate::run::verbs::{report, settle};

/// Send the entry to every member. A member whose file does not declare it
/// fails, and the others go ahead.
pub(super) async fn run(
    writer: Writer,
    members: &Devices<'_>,
    command: &str,
    pairs: &[String],
) -> Result<(), Failure> {
    let supplied = supplied(pairs)?;
    let outcomes = members
        .each(|handle| {
            let supplied = supplied.clone();
            async move {
                handle.ensure_known().await?;
                let call = handle.resolve()?;
                let values = call.args(command, supplied)?;
                Ok((call.mode(), call.invoke(command, &values).await?))
            }
        })
        .await;

    let mut failures: Vec<(DeviceId, Failure)> = Vec::new();
    for outcome in outcomes {
        match outcome.result {
            Ok((_, Invoked::Sent(served))) => report(writer, &served),
            Ok((mode, Invoked::Read(reply))) => fields(writer, mode, command, &reply),
            Err(error) => failures.push((outcome.id, usage(error))),
        }
    }
    settle(writer, members.len(), failures)
}

/// What the `reply:` layout captured, under the names the device file gives
/// the fields.
fn fields(writer: Writer, mode: Mode, command: &str, reply: &Reply) {
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
