//! `status`: the state of every device of the target, one line each.

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceId, DeviceStatus, Devices};
use serde_json::{Value, json};

use crate::run::verbs::settle;

pub(super) async fn run(writer: Writer, members: &Devices<'_>) -> Result<(), Failure> {
    let outcomes = members
        .each(|handle| async move {
            handle.ensure_known().await?;
            let call = handle.resolve()?;
            Ok((call.mode(), call.status().await?))
        })
        .await;

    let mut failures: Vec<(DeviceId, Failure)> = Vec::new();
    for outcome in outcomes {
        match outcome.result {
            Ok((mode, status)) => writer.emit(&as_json(&status, mode), &as_text(&status, mode)),
            Err(error) => failures.push((outcome.id, Failure::from(error))),
        }
    }
    settle(writer, members.len(), failures)
}

// The core renders the fields, so this line and the `status` event line agree.
fn as_json(status: &DeviceStatus, mode: Mode) -> Value {
    let mut record = status.to_json();
    if let Some(fields) = record.as_object_mut() {
        fields.insert("mode".to_owned(), json!(mode.to_string()));
    }
    record
}

fn as_text(status: &DeviceStatus, mode: Mode) -> String {
    format!("{}  {mode}  {status}", status.id)
}
