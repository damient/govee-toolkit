//! `identify`: take every device off, then light them one at a time, so a
//! person maps an identity to a fixture in the room.
//!
//! The walk drives one mode, and `lan` where the command line names none: the
//! rig it answers for is the rig on the network. It substitutes no other
//! mode, the way every other command does not.
//!
//! A device that fails stops no other one: the walk reports the failure,
//! carries on, and exits non-zero at the end.

use std::time::Duration;

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, Govee, Identify, Selector};
use serde_json::json;

use crate::output::{Failure, Writer};

/// What one walk does.
pub(super) struct Walk {
    /// What each device shows.
    pub(super) pass: Identify,
    /// How long the walk waits between two steps: after the rig goes off, and
    /// after each device lights.
    pub(super) wait: Duration,
    /// How long the last device holds the color before every device goes off.
    pub(super) hold: Duration,
    /// Leave every device on and lit at the end.
    pub(super) keep: bool,
    /// The one mode the walk drives over.
    pub(super) mode: Mode,
}

pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    named: &[String],
    walk: &Walk,
) -> Result<(), Failure> {
    let targets = targets(govee, named, walk.mode).await?;
    if targets.is_empty() {
        return Err(Failure::unreachable(format!(
            "no device answered over `{}`, so there is nothing to light",
            walk.mode
        )));
    }
    // The whole rig goes dark first. One lit device in a dark room is the
    // answer the operator reads. A device that refuses the blackout leaves
    // the walk here: a scan reports what answered it, and a handle the
    // configuration reaches through nothing is one of those.
    let mut failed = off(govee, writer, &targets, walk.mode).await;
    let targets: Vec<DeviceId> = targets
        .into_iter()
        .filter(|id| !failed.contains(id))
        .collect();
    tokio::time::sleep(walk.wait).await;
    for id in &targets {
        writer.emit(
            &json!({ "event": "identify", "device": id.to_string() }),
            &format!("identify {id}"),
        );
        if let Err(error) = govee.device_on(id, walk.mode).identify(&walk.pass).await {
            report(writer, id, &error.to_string());
            failed.push(id.clone());
        }
        tokio::time::sleep(walk.wait).await;
    }
    let mut stayed = Vec::new();
    if !walk.keep {
        tokio::time::sleep(walk.hold).await;
        stayed = off(govee, writer, &targets, walk.mode).await;
    }
    if failed.is_empty() && stayed.is_empty() {
        return Ok(());
    }
    Err(Failure::unreachable(summary(&failed, &stayed)))
}

/// What the walk failed at, as one line.
///
/// The closing blackout counts: a device that holds the color is a device the
/// operator must take off by hand. One that refused the pass and the blackout
/// is named in both lists, which are two faults the room shows.
fn summary(failed: &[DeviceId], stayed: &[DeviceId]) -> String {
    let mut parts = Vec::new();
    if !failed.is_empty() {
        parts.push(format!(
            "these devices did not take the pass: {}",
            names(failed)
        ));
    }
    if !stayed.is_empty() {
        parts.push(format!(
            "these devices did not go off at the end: {}",
            names(stayed)
        ));
    }
    parts.join("; ")
}

fn names(ids: &[DeviceId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Take every device off at once, and answer the ones that refused.
///
/// One task per device: a rig goes dark together, and a device that answers
/// slowly holds up no other one.
async fn off(govee: &Govee, writer: &Writer, targets: &[DeviceId], mode: Mode) -> Vec<DeviceId> {
    let mut passes = Vec::with_capacity(targets.len());
    for id in targets {
        let (govee, id) = (govee.clone(), id.clone());
        passes.push(tokio::spawn(async move {
            let outcome = govee.device_on(&id, mode).power(false).await;
            (id, outcome)
        }));
    }
    let mut refused = Vec::new();
    for pass in passes {
        let Ok((id, outcome)) = pass.await else {
            continue;
        };
        if let Err(error) = outcome {
            report(writer, &id, &error.to_string());
            refused.push(id);
        }
    }
    refused
}

/// The devices to walk: the ones the targets name, or every device a scan
/// over `mode` finds and the configuration enables that mode for.
///
/// A target that names a model or a name is answered from what the SDK knows,
/// so the scan runs before the selection. A target that names an identity
/// needs no scan: it addresses one device, which `ensure_known` then finds.
async fn targets(govee: &Govee, named: &[String], mode: Mode) -> Result<Vec<DeviceId>, Failure> {
    if named.is_empty() {
        let found = govee.scan_on(&[mode]).await?;
        return Ok(found
            .into_iter()
            .map(|device| device.id)
            .filter(|id| govee.device(id).modes().contains(&mode))
            .collect());
    }
    if named.iter().any(|target| !names_one_identity(target)) {
        govee.scan_on(&[mode]).await?;
    }
    let ids = govee.select(named)?;
    for id in &ids {
        govee.ensure_known(id).await?;
    }
    Ok(ids)
}

/// Whether the target addresses one device on its own. A target that reads as
/// nothing lands here as `false`, and the selection reports why.
fn names_one_identity(target: &str) -> bool {
    matches!(Selector::parse(target), Ok(Selector::Id(_)))
}

fn report(writer: &Writer, id: &DeviceId, reason: &str) {
    writer.emit(
        &json!({ "event": "failed", "device": id.to_string(), "reason": reason }),
        &format!("failed {id}  {reason}"),
    );
}
