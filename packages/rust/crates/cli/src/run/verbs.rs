//! Each verb calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name reaches this file.
//!
//! A member that fails stops no other one: its line goes to stderr, and the
//! run exits with the code of the first failure.

use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceId, Devices, Served, Verb};
use serde_json::json;

pub(super) async fn run(writer: Writer, members: &Devices<'_>, verb: Verb) -> Result<(), Failure> {
    let applied = members.apply(vec![verb]).await;
    let mut failures: Vec<(DeviceId, Failure)> = Vec::new();
    for outcome in applied.reached {
        if let Err(error) = outcome.result {
            failures.push((outcome.id, Failure::from(error)));
        }
    }
    for outcome in applied.steps.into_iter().flat_map(|step| step.outcomes) {
        match outcome.result {
            Ok(served) => report(writer, &served),
            Err(error) => failures.push((outcome.id, Failure::from(error))),
        }
    }
    verdict(writer, members.len(), failures)
}

/// One device fails with its own failure, as a command on one device does.
fn verdict(
    writer: Writer,
    count: usize,
    mut failures: Vec<(DeviceId, Failure)>,
) -> Result<(), Failure> {
    if count == 1 {
        return failures.pop().map_or(Ok(()), |(_, failure)| Err(failure));
    }
    for (id, failure) in &failures {
        writer.warn(
            &json!({
                "id": id.to_string(),
                "error": { "kind": failure.kind(), "message": failure.message() },
            }),
            &format!("{id} failed: {}", failure.message()),
        );
    }
    let names: Vec<String> = failures.iter().map(|(id, _)| id.to_string()).collect();
    let failed = failures.len();
    match failures.into_iter().next() {
        None => Ok(()),
        Some((_, first)) => Err(first.with_message(format!(
            "{failed} of {count} devices failed: {}",
            names.join(", ")
        ))),
    }
}

pub(super) fn report(writer: Writer, served: &Served) {
    writer.emit(
        &json!({
            "id": served.id.to_string(),
            "mode": served.mode.to_string(),
            "command": served.command,
        }),
        &format!("{} {} served by {}", served.id, served.command, served.mode),
    );
}
