//! `identify`: take every device off, then light them one at a time, so a
//! person maps an identity to a fixture in the room.
//!
//! The walk drives one mode, and `lan` where the command line names none: the
//! rig it answers for is the rig on the network. It substitutes no other
//! mode, the way every other command does not.
//!
//! The blackout covers the walked devices and no other one. A target list is
//! the scope the person typed, so a device outside it keeps the look it
//! holds.
//!
//! [`Govee::identify_walk`] runs the walk itself.

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, Govee, Selector, Walk, WalkObserver};
use serde_json::json;

use crate::output::{Failure, Writer};

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
    let report = govee
        .identify_walk(&targets, &targets, walk, &Lines(writer))
        .await?;
    match report.summary("device") {
        Some(summary) => Err(Failure::unreachable(summary)),
        None => Ok(()),
    }
}

/// What the walk prints, as one line or one JSON record per step.
struct Lines<'a>(&'a Writer);

impl WalkObserver for Lines<'_> {
    fn lighting(&self, id: &DeviceId) {
        self.0.emit(
            &json!({ "event": "identify", "device": id.to_string() }),
            &format!("identify {id}"),
        );
    }

    fn refused(&self, id: &DeviceId, reason: &str) {
        self.0.emit(
            &json!({ "event": "failed", "device": id.to_string(), "reason": reason }),
            &format!("failed {id}  {reason}"),
        );
    }
}

/// The devices to walk: the ones the targets name, or every device a scan
/// over `mode` finds and the configuration enables that mode for.
///
/// A target that names a model or a name is answered from what the SDK knows,
/// so the scan runs before the selection. A target that names an identity
/// needs no scan: it addresses one device, which `ensure_known` then finds.
///
/// [`Govee::select`] takes the mode, so a model and a name answer the devices
/// that enable it. A named identity stays in the list, and the walk reports
/// it: the person asked for that device by its identity.
async fn targets(govee: &Govee, named: &[String], mode: Mode) -> Result<Vec<DeviceId>, Failure> {
    if named.is_empty() {
        let found = govee.scan_on(&[mode]).await?;
        return Ok(found
            .into_iter()
            .map(|device| device.id)
            .filter(|id| govee.device(id).modes().contains(&mode))
            .collect());
    }
    if named
        .iter()
        .any(|target| !names_one_identity(govee, target))
    {
        govee.scan_on(&[mode]).await?;
    }
    let ids = govee.select(named, Some(mode))?;
    for id in &ids {
        govee.ensure_known(id).await?;
    }
    Ok(ids)
}

/// Whether the target addresses one device on its own. A target that reads as
/// nothing lands here as `false`, and the selection reports why.
fn names_one_identity(govee: &Govee, target: &str) -> bool {
    matches!(
        Selector::parse(target, govee.catalog()),
        Ok(Selector::Id(_))
    )
}
