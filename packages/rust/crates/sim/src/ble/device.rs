//! The peripheral: the connection, the frames it takes and the frames it
//! answers.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use uuid::Uuid;

use super::{
    BleFaults, BleOptions, FRAME_LEN, NOTIFY_CHARACTERISTIC, READ, SERVICE, WRITE,
    WRITE_CHARACTERISTIC, bcc,
};

/// How many answers a subscriber may fall behind by before losing the oldest.
const NOTIFY_BACKLOG: usize = 32;

/// A fake device. Cheap to clone; every clone is the same device.
#[derive(Debug, Clone)]
pub struct BleDevice {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    options: BleOptions,
    notify: broadcast::Sender<Vec<u8>>,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    faults: BleFaults,
    connected: bool,
    received: Vec<Vec<u8>>,
    answers: BTreeMap<u8, Vec<u8>>,
    /// When each write of the current burst window arrived.
    writes: Vec<Instant>,
    /// While this is in the future, the firmware answers nothing.
    unresponsive_until: Option<Instant>,
    stalls: u32,
    sent: u32,
}

impl BleDevice {
    /// A device on the air, answering nothing until a read is registered.
    #[must_use]
    pub fn start(options: BleOptions) -> Self {
        let faults = options.faults;
        Self {
            inner: Arc::new(Inner {
                options,
                notify: broadcast::channel(NOTIFY_BACKLOG).0,
                state: Mutex::new(State {
                    faults,
                    connected: false,
                    received: Vec::new(),
                    answers: BTreeMap::new(),
                    writes: Vec::new(),
                    unresponsive_until: None,
                    stalls: 0,
                    sent: 0,
                }),
            }),
        }
    }

    /// The handle the adapter addresses it by.
    #[must_use]
    pub fn endpoint(&self) -> String {
        self.inner.options.endpoint.clone()
    }

    /// The name it advertises.
    #[must_use]
    pub fn name(&self) -> String {
        self.inner
            .options
            .name
            .clone()
            .unwrap_or_else(|| format!("GBK_{}_0000", self.inner.options.sku))
    }

    /// Whether a link is up.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.inner.state.lock().is_ok_and(|state| state.connected)
    }

    /// Take the connection.
    ///
    /// # Errors
    ///
    /// [`std::io::ErrorKind::ConnectionRefused`] under
    /// [`BleFaults::refuse_connection`], and
    /// [`std::io::ErrorKind::AddrInUse`] if a link is already up: a device
    /// takes one at a time.
    pub fn connect(&self) -> std::io::Result<()> {
        let mut state = self.state()?;
        if state.faults.refuse_connection {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "the device refused the connection",
            ));
        }
        if state.connected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "the device already holds a connection",
            ));
        }
        state.connected = true;
        Ok(())
    }

    /// Drop the connection. The device advertises again.
    pub fn disconnect(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.connected = false;
        }
    }

    /// The characteristics it carries. Empty but for the vendor service, and
    /// empty altogether under [`BleOptions::carries_service`] set to `false`.
    #[must_use]
    pub fn characteristics(&self) -> Vec<Uuid> {
        if self.inner.options.carries_service {
            vec![SERVICE, WRITE_CHARACTERISTIC, NOTIFY_CHARACTERISTIC]
        } else {
            Vec::new()
        }
    }

    /// The frames it notifies, oldest first.
    #[must_use]
    pub fn notifications(&self) -> broadcast::Receiver<Vec<u8>> {
        self.inner.notify.subscribe()
    }

    /// Take one frame.
    ///
    /// Returns as soon as the frame is recorded. The answer, if there is one,
    /// is notified after [`BleFaults::latency`]: this wire acknowledges
    /// nothing, so a write never waits for it.
    ///
    /// # Errors
    ///
    /// [`std::io::ErrorKind::NotConnected`] with no link up,
    /// [`std::io::ErrorKind::Unsupported`] for any characteristic but the
    /// write one, and [`std::io::ErrorKind::InvalidData`] for a frame that is
    /// not [`FRAME_LEN`] bytes or whose BCC is wrong.
    pub fn write(&self, characteristic: Uuid, frame: &[u8]) -> std::io::Result<()> {
        if characteristic != WRITE_CHARACTERISTIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("the device takes no write on {characteristic}"),
            ));
        }
        check(frame)?;

        let answer = {
            let mut state = self.state()?;
            if !state.connected {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "the device holds no connection",
                ));
            }
            state.received.push(frame.to_vec());
            let now = Instant::now();
            state.burst(now);
            state.answer(frame, now)
        };

        if let Some((frame, latency)) = answer {
            let notify = self.inner.notify.clone();
            tokio::spawn(async move {
                if !latency.is_zero() {
                    tokio::time::sleep(latency).await;
                }
                let _ = notify.send(frame);
            });
        }
        Ok(())
    }

    /// What a read of `command_type` answers, as the payload after the two
    /// leading bytes. The device pads it and adds the BCC.
    pub fn set_read_answer(&self, command_type: u8, payload: &[u8]) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.answers.insert(command_type, payload.to_vec());
        }
    }

    /// Replace the faults wholesale.
    pub fn set_faults(&self, faults: BleFaults) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.faults = faults;
        }
    }

    /// Every frame received so far, oldest first.
    #[must_use]
    pub fn received(&self) -> Vec<Vec<u8>> {
        self.inner
            .state
            .lock()
            .map(|state| state.received.clone())
            .unwrap_or_default()
    }

    /// How many frames have been received.
    #[must_use]
    pub fn received_count(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| state.received.len())
            .unwrap_or_default()
    }

    /// How many times a burst has left the firmware unresponsive.
    #[must_use]
    pub fn stalls(&self) -> u32 {
        self.inner
            .state
            .lock()
            .map(|state| state.stalls)
            .unwrap_or_default()
    }

    /// Forget what has been received.
    pub fn clear(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.received.clear();
        }
    }

    fn state(&self) -> std::io::Result<std::sync::MutexGuard<'_, State>> {
        self.inner
            .state
            .lock()
            .map_err(|_| std::io::Error::other("the simulator's state is poisoned"))
    }
}

impl State {
    /// Count this write, and stall the firmware if the burst is too fast.
    fn burst(&mut self, now: Instant) {
        let Some(stall) = self.faults.stall else {
            return;
        };
        self.writes
            .retain(|at| now.duration_since(*at) < stall.within);
        self.writes.push(now);
        let over = u32::try_from(self.writes.len()).unwrap_or(u32::MAX) > stall.after;
        if over && self.unresponsive_until.is_none_or(|until| until <= now) {
            self.unresponsive_until = Some(now + stall.lasts);
            self.stalls = self.stalls.wrapping_add(1);
        }
    }

    /// The frame this one is answered with, and how late, or `None` for
    /// silence.
    fn answer(&mut self, frame: &[u8], now: Instant) -> Option<(Vec<u8>, Duration)> {
        if self.faults.silent || self.unresponsive_until.is_some_and(|until| until > now) {
            return None;
        }
        let (&kind, &command_type) = (frame.first()?, frame.get(1)?);
        let mut answer = match kind {
            // A write is acknowledged under the two bytes it came with, and
            // the status says the firmware took the frame, never that it
            // applied it. See `docs/protocol/ble.md` §1.4.
            WRITE => vec![WRITE, command_type, 0x00],
            // A read the device does not implement answers nothing at all,
            // which is a probe that fails and a capability nobody has.
            READ => {
                let mut bytes = vec![READ, command_type];
                bytes.extend_from_slice(self.answers.get(&command_type)?);
                bytes
            }
            // A multi-packet write is answered once it is complete, and
            // nothing here reassembles one.
            _ => return None,
        };
        answer.resize(FRAME_LEN, 0);
        if let Some(last) = answer.last_mut() {
            *last = 0;
        }
        let sum = bcc(&answer);
        if let Some(last) = answer.last_mut() {
            *last = sum;
        }

        self.sent = self.sent.wrapping_add(1);
        let dropped = self
            .faults
            .drop_one_in
            .is_some_and(|n| n > 0 && self.sent.is_multiple_of(n));
        (!dropped).then_some((answer, self.faults.latency))
    }
}

/// Refuse a frame the wire cannot carry.
fn check(frame: &[u8]) -> std::io::Result<()> {
    if frame.len() != FRAME_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "the frame is {} bytes; this wire carries {FRAME_LEN}",
                frame.len()
            ),
        ));
    }
    let sum = bcc(frame);
    if frame.last() != Some(&sum) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("the frame's BCC is {:?}, not {sum:#04x}", frame.last()),
        ));
    }
    Ok(())
}
