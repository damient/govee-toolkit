//! `identify`: take every device off, then light them one at a time, so a
//! person maps an identity to a fixture in the room.
//!
//! The walk drives one mode, and `lan` where the command line names none: the
//! rig it answers for is the rig on the network.
//!
//! The blackout covers the walked devices and no other one. A target list is
//! the scope the person typed, so a device outside it keeps the look it
//! holds.
//!
//! [`Govee::identify`] reads the targets and runs the walk.

use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceId, Govee, Walk, WalkObserver};
use serde_json::json;

pub(super) async fn run(
    govee: &Govee,
    writer: Writer,
    named: &[String],
    walk: &Walk,
) -> Result<(), Failure> {
    let targets = (!named.is_empty()).then_some(named);
    let report = govee.identify(targets, walk, &Lines(writer)).await?;
    if report.lit.is_empty() {
        return Err(Failure::unreachable(format!(
            "no device answered over `{}`, so there is nothing to light",
            walk.mode
        )));
    }
    match report.summary("device") {
        Some(summary) => Err(Failure::unreachable(summary)),
        None => Ok(()),
    }
}

/// What the walk prints, as one line or one JSON record per step.
struct Lines(Writer);

impl WalkObserver for Lines {
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
