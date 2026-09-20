//! Which devices hold an armed segment channel, and why the send path asks.
//!
//! A unit can answer no status from the arming frame to the disarm, whether or
//! not frames flow — `docs/protocol/lan.md` 2.3. Fire-and-verify sent there
//! can only expire: it costs one datagram per command and records against the
//! breaker a failure the device did not earn.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::codec::Mode;
use crate::govee::{Govee, Inner};
use crate::transport::DeviceId;

/// The armed channels, by mode then device, with the number of streams that
/// hold each one.
#[derive(Debug, Default)]
pub(crate) struct ArmedStreams {
    /// How many streams hold a channel armed. Read before the lock, so a send
    /// pays one atomic load while no stream runs.
    count: AtomicUsize,
    armed: Mutex<HashMap<Mode, HashMap<DeviceId, usize>>>,
}

impl ArmedStreams {
    fn arm(&self, id: &DeviceId, mode: Mode) {
        if let Ok(mut armed) = self.armed.lock() {
            *armed
                .entry(mode)
                .or_default()
                .entry(id.clone())
                .or_default() += 1;
            self.count.fetch_add(1, Ordering::Release);
        }
    }

    fn disarm(&self, id: &DeviceId, mode: Mode) {
        if let Ok(mut armed) = self.armed.lock()
            && let Some(by_device) = armed.get_mut(&mode)
            && let Some(holders) = by_device.get_mut(id)
        {
            *holders -= 1;
            if *holders == 0 {
                by_device.remove(id);
            }
            self.count.fetch_sub(1, Ordering::Release);
        }
    }

    fn contains(&self, id: &DeviceId, mode: Mode) -> bool {
        if self.count.load(Ordering::Acquire) == 0 {
            return false;
        }
        self.armed.lock().is_ok_and(|armed| {
            armed
                .get(&mode)
                .is_some_and(|by_device| by_device.contains_key(id))
        })
    }
}

impl Govee {
    /// Whether a segment channel is armed on this device over this mode.
    ///
    /// The send path reads it to skip a verification the device cannot answer.
    pub(crate) fn stream_armed(&self, id: &DeviceId, mode: Mode) -> bool {
        self.inner.streams.contains(id, mode)
    }
}

/// Marks one channel armed for as long as it lives.
///
/// The stream's shared state holds it, so the mark ends when the emitting task
/// has sent the disarming frame and dropped its handle.
pub(crate) struct ArmedGuard {
    inner: std::sync::Arc<Inner>,
    id: DeviceId,
    mode: Mode,
}

impl ArmedGuard {
    pub(crate) fn new(govee: &Govee, id: &DeviceId, mode: Mode) -> Self {
        govee.inner.streams.arm(id, mode);
        Self {
            inner: std::sync::Arc::clone(&govee.inner),
            id: id.clone(),
            mode,
        }
    }
}

impl std::fmt::Debug for ArmedGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArmedGuard")
            .field("id", &self.id)
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl Drop for ArmedGuard {
    fn drop(&mut self) {
        self.inner.streams.disarm(&self.id, self.mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> DeviceId {
        DeviceId::new("AA:BB:CC:DD:EE:FF")
    }

    #[test]
    fn an_unarmed_device_is_not_reported_armed() {
        let streams = ArmedStreams::default();
        assert!(!streams.contains(&id(), Mode::Lan));
    }

    #[test]
    fn the_mark_covers_one_mode_only() {
        let streams = ArmedStreams::default();
        streams.arm(&id(), Mode::Lan);
        assert!(streams.contains(&id(), Mode::Lan));
        assert!(!streams.contains(&id(), Mode::Ble));
    }

    #[test]
    fn the_last_stream_clears_the_mark() {
        let streams = ArmedStreams::default();
        streams.arm(&id(), Mode::Lan);
        streams.arm(&id(), Mode::Lan);
        streams.disarm(&id(), Mode::Lan);
        assert!(streams.contains(&id(), Mode::Lan));
        streams.disarm(&id(), Mode::Lan);
        assert!(!streams.contains(&id(), Mode::Lan));
    }
}
