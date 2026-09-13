//! The peripheral: the connection, the frames it takes and the frames it
//! answers.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::broadcast;
use uuid::Uuid;

use super::state::State;
use super::{BleFaults, BleOptions, NOTIFY_CHARACTERISTIC, SERVICE, WRITE_CHARACTERISTIC};

/// The flags byte of the advertisement data, with and without the encoding
/// bit. The low nibble is the layout version.
const FLAGS_ENCODED: u8 = 0x43;
const FLAGS_PLAIN: u8 = 0x03;

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
                    session: None,
                    sessions: 0,
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

    /// The advertisement data it advertises, as a platform splits it: the
    /// layout of `docs/protocol/ble.md` §1.5, `pactType` 1 and `pactCode` 1,
    /// with the encoding bit set under [`BleOptions::encoded`].
    #[must_use]
    pub fn adverts(&self) -> Vec<(u16, Vec<u8>)> {
        let flags = if self.inner.options.encoded {
            FLAGS_ENCODED
        } else {
            FLAGS_PLAIN
        };
        let prefix = u16::from_le_bytes([flags, 0x88]);
        vec![(prefix, vec![0xEC, 0x00, 0x01, 0x01])]
    }

    /// The session seed the handshake handed out, if one has.
    #[must_use]
    pub fn session_seed(&self) -> Option<[u8; 16]> {
        self.inner.state.lock().ok().and_then(|state| state.session)
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

    /// Drop the connection. The device advertises again, and the session seed
    /// is gone with the link.
    pub fn disconnect(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.connected = false;
            state.session = None;
        }
    }

    /// The characteristics it carries. Empty but for the service, and
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
    /// Returns as soon as the frame is recorded; any answer is notified after
    /// [`BleFaults::latency`]. This wire acknowledges nothing.
    ///
    /// An encoded device decodes the frame first. A handshake frame is
    /// answered as `docs/protocol/ble.md` §9 says, a plaintext frame is
    /// recorded as it came and answered with silence, and an encoded frame is
    /// recorded decoded.
    ///
    /// # Errors
    ///
    /// [`std::io::ErrorKind::NotConnected`] with no link up,
    /// [`std::io::ErrorKind::Unsupported`] for any characteristic but the
    /// write one, and [`std::io::ErrorKind::InvalidData`] for a plaintext
    /// frame that is not [`FRAME_LEN`](super::FRAME_LEN) bytes or whose BCC is
    /// wrong.
    pub fn write(&self, characteristic: Uuid, frame: &[u8]) -> std::io::Result<()> {
        if characteristic != WRITE_CHARACTERISTIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("the device takes no write on {characteristic}"),
            ));
        }
        if !self.inner.options.encoded {
            super::state::check(frame)?;
        }

        let answer = {
            let mut state = self.state()?;
            if !state.connected {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "the device holds no connection",
                ));
            }
            let now = Instant::now();
            if self.inner.options.encoded {
                state.encoded(frame, now)
            } else {
                state.received.push(frame.to_vec());
                state.burst(now);
                state.answer(frame, now)
            }
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
