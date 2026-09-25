//! The identify walk: take a rig off, then light one device at a time.
//!
//! The caller names two sets: the devices that go off, and the devices that
//! light. They differ where the caller narrows the walk to a part of a rig it
//! still wants dark.
//!
//! The walk drives one mode and substitutes no other. A device that fails
//! stops no other one: the walk reports it and carries on.

use std::time::Duration;

use crate::codec::Mode;
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::select::Selector;
use crate::transport::DeviceId;
use crate::verbs::{IDENTIFY_HOLD, IDENTIFY_WAIT, Identify};

/// What one walk does, beyond which devices it covers.
#[derive(Debug, Clone, Copy)]
pub struct Walk {
    /// What each device shows.
    pub pass: Identify,
    /// How long the walk waits between two steps: after the rig goes off, and
    /// after each device lights.
    pub wait: Duration,
    /// How long the last device holds the color before every device goes off.
    pub hold: Duration,
    /// Leave every device on and lit at the end.
    pub keep: bool,
    /// The one mode the walk drives over.
    pub mode: Mode,
}

impl Default for Walk {
    /// The walk `govee identify` runs: green at full brightness, over `lan`,
    /// and every device off at the end.
    fn default() -> Self {
        Self {
            pass: Identify::default(),
            wait: IDENTIFY_WAIT,
            hold: IDENTIFY_HOLD,
            keep: false,
            mode: Mode::Lan,
        }
    }
}

/// What the caller prints while the walk runs.
///
/// The walk calls these in order, from the task that drives it. A caller that
/// prints nothing passes `&()`.
pub trait WalkObserver {
    /// The walk is about to light this device. A caller names it here, so a
    /// person reads the line and the room at the same time.
    fn lighting(&self, id: &DeviceId);

    /// A device refused a command, and `reason` says what it answered. The
    /// walk carries on and names the device again in [`WalkReport`].
    fn refused(&self, id: &DeviceId, reason: &str);
}

impl WalkObserver for () {
    fn lighting(&self, _id: &DeviceId) {}
    fn refused(&self, _id: &DeviceId, _reason: &str) {}
}

/// What one walk failed at. Empty lists are a clean walk.
#[derive(Debug, Clone, Default)]
pub struct WalkReport {
    /// The devices that refused the opening blackout or the pass.
    pub failed: Vec<DeviceId>,
    /// The devices that refused the closing blackout, and hold the color.
    pub stayed: Vec<DeviceId>,
}

impl WalkReport {
    /// What the walk failed at, as one line, and `None` where it failed at
    /// nothing. `noun` names one device the way the caller does, such as
    /// `"fixture"`; the line adds the plural `s`.
    ///
    /// A device that refused the pass and the closing blackout is named in
    /// both lists.
    #[must_use]
    pub fn summary(&self, noun: &str) -> Option<String> {
        let mut parts = Vec::new();
        if !self.failed.is_empty() {
            parts.push(format!(
                "these {noun}s did not take the pass: {}",
                names(&self.failed)
            ));
        }
        if !self.stayed.is_empty() {
            parts.push(format!(
                "these {noun}s did not go off at the end: {}",
                names(&self.stayed)
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }
}

fn names(ids: &[DeviceId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

impl Govee {
    /// Take `blackout` off, light each of `lit` in turn, then take `blackout`
    /// off again.
    ///
    /// `lit` gives the order a person reads. A device in both sets is normal:
    /// it goes dark with the rig and lights in its turn. A device that
    /// refuses the opening blackout is dropped from `lit` and from the
    /// closing blackout, because the walk never made it dark.
    ///
    /// The call holds for the length of the walk, which is
    /// `wait * (lit.len() + 1)` plus `hold`.
    ///
    /// # Errors
    ///
    /// [`Error::ModeNotEnabled`] where the configuration does not enable
    /// `walk.mode` for a device of either set. The walk drives what the user
    /// enabled and nothing else, so it reports this before it sends
    /// anything. A device that fails during the walk is in the report
    /// instead, not an error.
    pub async fn identify_walk(
        &self,
        blackout: &[DeviceId],
        lit: &[DeviceId],
        walk: &Walk,
        observer: &(dyn WalkObserver + Sync),
    ) -> Result<WalkReport> {
        self.enabled_for(blackout, walk.mode)?;
        self.enabled_for(lit, walk.mode)?;

        let dark = self.take_off(blackout, walk.mode, observer).await;
        let mut failed = dark.clone();
        tokio::time::sleep(walk.wait).await;
        for id in lit.iter().filter(|id| !dark.contains(id)) {
            observer.lighting(id);
            if let Err(error) = self.device_on(id, walk.mode).identify(&walk.pass).await {
                observer.refused(id, &error.to_string());
                failed.push(id.clone());
            }
            tokio::time::sleep(walk.wait).await;
        }
        let mut stayed = Vec::new();
        if !walk.keep {
            tokio::time::sleep(walk.hold).await;
            let rest: Vec<DeviceId> = blackout
                .iter()
                .filter(|id| !dark.contains(id))
                .cloned()
                .collect();
            stayed = self.take_off(&rest, walk.mode, observer).await;
        }
        Ok(WalkReport { failed, stayed })
    }

    /// The devices a walk covers: the ones the targets name, in the order
    /// they were written, or every device a scan over `mode` finds and the
    /// configuration enables that mode for.
    ///
    /// A target is an identity, a SKU, a name or a group, as for
    /// [`Govee::select`]. A SKU, a name and a group are answered from what the
    /// SDK knows, so the scan runs before the selection. An identity needs no
    /// scan: [`Govee::ensure_known`] finds that one device. A named identity
    /// stays in the list, and the walk reports it.
    ///
    /// # Errors
    ///
    /// [`Error::ModeNotEnabled`] where the configuration does not enable
    /// `mode` for a device a target names, before any scan for it. What
    /// [`Govee::select`] reports for a target, and what [`Govee::scan_on`] and
    /// [`Govee::ensure_known`] report.
    pub async fn walk_targets<T: AsRef<str>>(
        &self,
        named: &[T],
        mode: Mode,
    ) -> Result<Vec<DeviceId>> {
        if named.is_empty() {
            let found = self.scan_on(&[mode]).await?;
            return Ok(found
                .into_iter()
                .map(|device| device.id)
                .filter(|id| self.device(id).modes().contains(&mode))
                .collect());
        }
        let identity = |target: &T| {
            matches!(
                Selector::parse(target.as_ref(), self.catalog()),
                Ok(Selector::Id(_))
            )
        };
        if !named.iter().all(identity) {
            self.scan_on(&[mode]).await?;
        }
        let ids = self.select(named, Some(mode))?;
        self.enabled_for(&ids, mode)?;
        for id in &ids {
            self.ensure_known(id).await?;
        }
        Ok(ids)
    }

    /// Every device must enable the mode the walk drives.
    fn enabled_for(&self, ids: &[DeviceId], mode: Mode) -> Result<()> {
        for id in ids {
            if !self.inner.config.modes_for(id).contains(&mode) {
                return Err(Error::ModeNotEnabled {
                    id: id.clone(),
                    mode,
                });
            }
        }
        Ok(())
    }

    /// Take every device off at once, and answer the ones that refused.
    ///
    /// One task per device: a rig goes dark together, and a device that
    /// answers slowly holds up no other one.
    async fn take_off(
        &self,
        ids: &[DeviceId],
        mode: Mode,
        observer: &(dyn WalkObserver + Sync),
    ) -> Vec<DeviceId> {
        let mut passes = Vec::with_capacity(ids.len());
        for id in ids {
            let (govee, id) = (self.clone(), id.clone());
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
                observer.refused(&id, &error.to_string());
                refused.push(id);
            }
        }
        refused
    }
}
