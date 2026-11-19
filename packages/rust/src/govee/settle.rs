//! When a device reports the state that a command set.
//!
//! A unit can report its previous state for a short time after a command —
//! `docs/protocol/lan.md` 1, "Read after a command". The send path records when
//! that time ends, and a status request waits for it. The record lives in this
//! process only.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;

use crate::codec::Mode;
use crate::govee::Govee;
use crate::transport::DeviceId;

/// The end of the settle time, by device and mode.
#[derive(Debug, Default)]
pub(crate) struct Settling {
    until: Mutex<HashMap<(DeviceId, Mode), Instant>>,
}

impl Settling {
    fn wrote(&self, id: &DeviceId, mode: Mode, settle: Duration) {
        if let Ok(mut until) = self.until.lock() {
            until.insert((id.clone(), mode), Instant::now() + settle);
        }
    }

    fn deadline(&self, id: &DeviceId, mode: Mode) -> Option<Instant> {
        let mut until = self.until.lock().ok()?;
        let key = (id.clone(), mode);
        let deadline = *until.get(&key)?;
        if deadline <= Instant::now() {
            until.remove(&key);
            return None;
        }
        Some(deadline)
    }
}

impl Govee {
    /// Record a command sent to `id` over `mode`. A SKU whose file records no
    /// `status_settle_ms` for the mode records nothing.
    pub(crate) fn wrote(&self, id: &DeviceId, sku: &str, mode: Mode) {
        let Ok(device) = self.inner.catalog.device(sku) else {
            return;
        };
        if let Some(settle) = device.measurements.status_settle(mode) {
            self.inner.settling.wrote(id, mode, settle);
        }
    }

    /// Wait until `id` reports the state that the last command over `mode`
    /// set. Returns at once where no settle time runs.
    pub(crate) async fn settled(&self, id: &DeviceId, mode: Mode) {
        if let Some(deadline) = self.inner.settling.deadline(id, mode) {
            tokio::time::sleep_until(deadline).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_deadline_runs_for_the_settle_time_and_then_ends() {
        let settling = Settling::default();
        let id = DeviceId::new("AA:00:00:00:00:01");
        assert!(settling.deadline(&id, Mode::Lan).is_none());

        settling.wrote(&id, Mode::Lan, Duration::from_millis(400));
        assert!(settling.deadline(&id, Mode::Lan).is_some());
        assert!(settling.deadline(&id, Mode::Ble).is_none());

        tokio::time::advance(Duration::from_millis(400)).await;
        assert!(settling.deadline(&id, Mode::Lan).is_none());
    }
}
