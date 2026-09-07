//! A fake Govee peripheral on GATT, and the adapter that finds it, so the
//! `ble` transport can be tested in CI. It can be told to go silent, to answer
//! late, to drop answers, to refuse the connection, and to stall under a burst
//! (`docs/protocol/ble.md` §5).
//!
//! It plays the wire and not the firmware, as the `lan` device does: it checks
//! the length and the BCC and acknowledges a write (§1.4), but reads no
//! payload. Set what a read answers with [`BleDevice::set_read_answer`], and
//! assert on [`BleDevice::received`].

mod device;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uuid::Uuid;

pub use self::device::BleDevice;

/// The vendor service, repeated here so a simulator can be started without
/// depending on the transport crate. See `docs/protocol/ble.md` §1.1.
pub const SERVICE: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_1910);
/// The characteristic frames are written to.
pub const WRITE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b11);
/// The characteristic answers are notified on.
pub const NOTIFY_CHARACTERISTIC: Uuid = Uuid::from_u128(0x0001_0203_0405_0607_0809_0a0b_0c0d_2b10);
/// The length of every frame on this wire, in bytes, checksum included.
pub const FRAME_LEN: usize = 20;
/// The `proType` of a single write.
pub const WRITE: u8 = 0x33;
/// The `proType` of a single read.
pub const READ: u8 = 0xaa;

/// How many writes a second the firmware takes, and what it does past that.
///
/// A device written to too fast does not refuse a frame: it stops answering
/// for seconds. The numbers are one unit's, so a test states its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stall {
    /// How many frames within [`Stall::within`] the firmware takes.
    pub after: u32,
    /// The window those frames are counted over.
    pub within: Duration,
    /// How long the firmware then answers nothing.
    pub lasts: Duration,
}

/// Ways to make the device behave badly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BleFaults {
    /// Answer nothing at all. Frames are still recorded.
    pub silent: bool,
    /// Drop one answer in every `n`. `Some(1)` drops all of them.
    pub drop_one_in: Option<u32>,
    /// Wait this long before answering. A write returns before the answer:
    /// this wire acknowledges nothing.
    pub latency: Duration,
    /// Refuse the connection.
    pub refuse_connection: bool,
    /// Stop answering under a burst. `None` takes any rate.
    pub stall: Option<Stall>,
}

/// How the device is set up.
#[derive(Debug, Clone)]
pub struct BleOptions {
    /// The handle the adapter addresses it by, and what a scan reports.
    pub endpoint: String,
    /// The SKU its advertised name carries.
    pub sku: String,
    /// The name it advertises. `None` builds `GBK_<SKU>_0000`, the shape
    /// `docs/protocol/ble.md` §1.3 documents.
    pub name: Option<String>,
    /// Whether it carries the vendor service. `false` advertises and connects
    /// but leaves the link nothing to write to.
    pub carries_service: bool,
    /// How it misbehaves.
    pub faults: BleFaults,
}

impl BleOptions {
    /// A device that answers, at a handle of its own.
    #[must_use]
    pub fn new(endpoint: impl Into<String>, sku: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            sku: sku.into(),
            name: None,
            carries_service: true,
            faults: BleFaults::default(),
        }
    }
}

/// A fake radio, holding the devices on the air.
///
/// A device is reachable only once a scan has heard it, as on hardware, and a
/// connected device stops advertising (`docs/protocol/ble.md` §1.1).
#[derive(Debug, Default)]
pub struct BleAdapter {
    devices: Mutex<Vec<BleDevice>>,
    state: Mutex<AdapterState>,
}

#[derive(Debug, Default)]
struct AdapterState {
    scanning: bool,
    scans: u32,
    /// The handles a scan has heard.
    held: BTreeSet<String>,
}

impl BleAdapter {
    /// A radio with nothing on the air.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// A radio carrying these devices.
    #[must_use]
    pub fn holding(devices: impl IntoIterator<Item = BleDevice>) -> Arc<Self> {
        let adapter = Self::new();
        for device in devices {
            adapter.add(device);
        }
        adapter
    }

    /// Put one more device on the air.
    pub fn add(&self, device: BleDevice) {
        if let Ok(mut devices) = self.devices.lock() {
            devices.push(device);
        }
    }

    /// Start listening.
    pub fn start_scan(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.scanning = true;
            state.scans = state.scans.wrapping_add(1);
        }
    }

    /// Stop listening. What was heard stays reachable.
    pub fn stop_scan(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.scanning = false;
        }
    }

    /// How many scans have been started.
    #[must_use]
    pub fn scans(&self) -> u32 {
        self.state.lock().map(|s| s.scans).unwrap_or_default()
    }

    /// The advertisements heard so far, as a handle and a name.
    ///
    /// Empty until a scan is running. A device holding a connection is not
    /// advertising and is not reported.
    #[must_use]
    pub fn heard(&self) -> Vec<(String, String)> {
        let Ok(mut state) = self.state.lock() else {
            return Vec::new();
        };
        if !state.scanning {
            return Vec::new();
        }
        let Ok(devices) = self.devices.lock() else {
            return Vec::new();
        };
        devices
            .iter()
            .filter(|device| !device.is_connected())
            .map(|device| {
                state.held.insert(device.endpoint());
                (device.endpoint(), device.name())
            })
            .collect()
    }

    /// The device behind a handle, or `None` if no scan has heard it.
    #[must_use]
    pub fn peripheral(&self, endpoint: &str) -> Option<BleDevice> {
        let held = self
            .state
            .lock()
            .is_ok_and(|state| state.held.iter().any(|h| h.eq_ignore_ascii_case(endpoint)));
        if !held {
            return None;
        }
        self.devices.lock().ok().and_then(|devices| {
            devices
                .iter()
                .find(|device| device.endpoint().eq_ignore_ascii_case(endpoint))
                .cloned()
        })
    }
}

/// The BCC of a frame: the XOR of every byte but the last.
///
/// # Panics
///
/// Never: an empty frame answers 0.
#[must_use]
pub fn bcc(frame: &[u8]) -> u8 {
    frame
        .iter()
        .take(frame.len().saturating_sub(1))
        .fold(0u8, |sum, byte| sum ^ byte)
}
