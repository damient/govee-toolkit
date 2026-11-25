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
    settle(writer, members.len(), failures)
}

/// Report the failures of a run over `members` devices. A set of one member
/// exits with the error itself. A larger set writes one line per failure to
/// stderr, and exits with the code of the first one.
pub(super) fn settle(
    writer: Writer,
    members: usize,
    mut failures: Vec<(DeviceId, Failure)>,
) -> Result<(), Failure> {
    if failures.is_empty() {
        return Ok(());
    }
    if members == 1 {
        return Err(failures.remove(0).1);
    }
    let mut ids: Vec<String> = Vec::new();
    for (id, failure) in &failures {
        writer.warn(
            &json!({
                "id": id.to_string(),
                "error": { "kind": failure.kind(), "message": failure.message() },
            }),
            &format!("{id} failed: {}", failure.message()),
        );
        if !ids.contains(&id.to_string()) {
            ids.push(id.to_string());
        }
    }
    let line = format!(
        "{} of {members} devices failed: {}",
        ids.len(),
        ids.join(", ")
    );
    Err(failures.remove(0).1.with_message(line))
}

pub(super) fn report(writer: Writer, served: &Served) {
    writer.emit(
        &served.to_json(),
        &format!("{} {} served by {}", served.id, served.command, served.mode),
    );
}
