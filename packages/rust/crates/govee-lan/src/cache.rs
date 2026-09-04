//! The on-disk device cache.
//!
//! Discovery is a multicast round-trip; a command must not pay for one. The
//! cache is what makes that possible across restarts: addresses learned by a
//! scan are written down, and the next process sends its first command without
//! waiting for anything (`docs/protocol/lan.md` §1, latency notes).
//!
//! It holds only what discovery reports — address, identity, SKU, firmware. It
//! is a hint, not a source of truth: a cached address that has stopped
//! answering is the circuit breaker's problem, not the cache's.
//!
//! This module carries no policy about *where* the file lives. A caller passes
//! a path or uses [`Cache::in_memory`].

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::DeviceId;
use crate::discovery::DiscoveredDevice;
use crate::error::{Error, Result};

/// Bumped when the file layout changes. A file written by a newer version is
/// discarded rather than guessed at — re-scanning costs one round-trip.
const FORMAT_VERSION: u32 = 1;

/// Distinguishes the temporary files of two concurrent writes.
static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One device as the last scan saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedDevice {
    /// The MAC the device reports.
    pub id: DeviceId,
    /// Its address at `last_seen`.
    pub ip: IpAddr,
    /// The SKU it reports.
    pub sku: String,
    /// `bleVersionHard`.
    #[serde(default)]
    pub ble_hardware: String,
    /// `bleVersionSoft`.
    #[serde(default)]
    pub ble_software: String,
    /// `wifiVersionHard`.
    #[serde(default)]
    pub wifi_hardware: String,
    /// `wifiVersionSoft`.
    #[serde(default)]
    pub wifi_software: String,
    /// When it last answered a scan, in seconds since the epoch.
    pub last_seen: u64,
}

/// What recording a scan reply changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// An identity the cache had never seen.
    New,
    /// A known device, at the same address as before.
    Refreshed,
    /// A known device that moved — a new DHCP lease, usually.
    Moved,
    /// A known device whose reported firmware changed. Worth surfacing:
    /// `docs/protocol/lan.md` §2.8, behavior can open or close with an update.
    FirmwareChanged,
}

#[derive(Debug, Serialize, Deserialize)]
struct File {
    version: u32,
    devices: Vec<CachedDevice>,
}

/// Everything discovery has learned, optionally backed by a file.
#[derive(Debug, Clone, Default)]
pub struct Cache {
    path: Option<PathBuf>,
    entries: BTreeMap<DeviceId, CachedDevice>,
}

impl Cache {
    /// A cache that is never written anywhere.
    #[must_use]
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Read the cache at `path`, if it exists.
    ///
    /// A missing file is an empty cache, not an error: the first run of any
    /// installation has no cache.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] if the file exists but cannot be read. A file that
    /// cannot be *parsed* is not an error — it is discarded and rebuilt by the
    /// next scan, since a corrupt cache must not stop an SDK from starting.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path: Some(path),
                    entries: BTreeMap::new(),
                });
            }
            Err(e) => {
                return Err(Error::Cache {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                });
            }
        };

        let entries = match serde_json::from_slice::<File>(&bytes) {
            Ok(file) if file.version == FORMAT_VERSION => file
                .devices
                .into_iter()
                .map(|d| (d.id.clone(), d))
                .collect(),
            Ok(file) => {
                tracing::warn!(
                    path = %path.display(),
                    found = file.version,
                    expected = FORMAT_VERSION,
                    "device cache written by another version, discarding it"
                );
                BTreeMap::new()
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "unreadable device cache, discarding it");
                BTreeMap::new()
            }
        };

        Ok(Self {
            path: Some(path),
            entries,
        })
    }

    /// The file this cache is backed by, if any.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Look a device up by identity.
    #[must_use]
    pub fn get(&self, id: &DeviceId) -> Option<&CachedDevice> {
        self.entries.get(id)
    }

    /// Every device known, in identity order.
    pub fn devices(&self) -> impl Iterator<Item = &CachedDevice> {
        self.entries.values()
    }

    /// How many devices are known.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is known yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Record a scan reply. Returns what it changed.
    pub fn record(&mut self, device: &DiscoveredDevice, now: SystemTime) -> Change {
        let entry = CachedDevice {
            id: device.id.clone(),
            ip: device.ip,
            sku: device.sku.clone(),
            ble_hardware: device.ble_hardware.clone(),
            ble_software: device.ble_software.clone(),
            wifi_hardware: device.wifi_hardware.clone(),
            wifi_software: device.wifi_software.clone(),
            last_seen: seconds(now),
        };

        let change = match self.entries.get(&device.id) {
            None => Change::New,
            Some(previous) if previous.ip != entry.ip => Change::Moved,
            Some(previous)
                if previous.wifi_software != entry.wifi_software
                    || previous.ble_software != entry.ble_software =>
            {
                Change::FirmwareChanged
            }
            Some(_) => Change::Refreshed,
        };

        self.entries.insert(device.id.clone(), entry);
        change
    }

    /// Forget a device.
    pub fn remove(&mut self, id: &DeviceId) -> Option<CachedDevice> {
        self.entries.remove(id)
    }

    /// Forget every device not seen for `age`. Returns what was dropped.
    ///
    /// An entry going stale says nothing about the device: it may simply be
    /// off. Pruning keeps the cache from growing without bound across a
    /// lifetime of DHCP leases — nothing more.
    pub fn prune(&mut self, now: SystemTime, age: std::time::Duration) -> Vec<CachedDevice> {
        let cutoff = seconds(now).saturating_sub(age.as_secs());
        let stale: Vec<DeviceId> = self
            .entries
            .iter()
            .filter(|(_, d)| d.last_seen < cutoff)
            .map(|(id, _)| id.clone())
            .collect();
        stale
            .into_iter()
            .filter_map(|id| self.entries.remove(&id))
            .collect()
    }

    /// Write the cache out, if it is backed by a file.
    ///
    /// Writes to a temporary file and renames it, so a process killed mid-write
    /// leaves the previous cache intact rather than a truncated one.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] if the directory cannot be created or the file cannot
    /// be written.
    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let fail = |reason: String| Error::Cache {
            path: path.display().to_string(),
            reason,
        };

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| fail(e.to_string()))?;
        }

        let file = File {
            version: FORMAT_VERSION,
            devices: self.entries.values().cloned().collect(),
        };
        let bytes = serde_json::to_vec_pretty(&file).map_err(|e| fail(e.to_string()))?;

        // Unique per write: a background save and an explicit one can overlap,
        // and a shared temporary name means one of them renames a file the
        // other has already moved.
        let temporary = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&temporary, &bytes).map_err(|e| fail(e.to_string()))?;
        std::fs::rename(&temporary, path).map_err(|e| fail(e.to_string()))?;
        Ok(())
    }
}

fn seconds(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;
    use std::time::Duration;

    fn discovered(ip: [u8; 4], firmware: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            id: DeviceId::new("aa:bb:cc:dd:ee:ff"),
            ip: IpAddr::from(ip),
            sku: "H61A0".to_owned(),
            ble_hardware: String::new(),
            ble_software: String::new(),
            wifi_hardware: "1.00.10".to_owned(),
            wifi_software: firmware.to_owned(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("govee-lan-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("devices.json")
    }

    #[test]
    fn reports_what_each_reply_changed() {
        let now = SystemTime::now();
        let mut cache = Cache::in_memory();
        assert_eq!(
            cache.record(&discovered([192, 168, 1, 42], "2.05.08"), now),
            Change::New
        );
        assert_eq!(
            cache.record(&discovered([192, 168, 1, 42], "2.05.08"), now),
            Change::Refreshed
        );
        assert_eq!(
            cache.record(&discovered([192, 168, 1, 77], "2.05.08"), now),
            Change::Moved
        );
        assert_eq!(
            cache.record(&discovered([192, 168, 1, 77], "2.06.02"), now),
            Change::FirmwareChanged
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn survives_a_restart() {
        let path = scratch("roundtrip");
        let now = SystemTime::now();

        let mut cache = Cache::load(&path).expect("empty cache");
        assert!(cache.is_empty());
        cache.record(&discovered([192, 168, 1, 42], "2.05.08"), now);
        cache.save().expect("write");

        let reloaded = Cache::load(&path).expect("reload");
        let device = reloaded
            .get(&DeviceId::new("AA:BB:CC:DD:EE:FF"))
            .expect("the device survives");
        assert_eq!(device.ip, IpAddr::from([192, 168, 1, 42]));
        assert_eq!(device.sku, "H61A0");

        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn a_corrupt_file_starts_empty_rather_than_failing() {
        let path = scratch("corrupt");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, b"{ not json").expect("write");

        let cache = Cache::load(&path).expect("a corrupt cache is not fatal");
        assert!(cache.is_empty());

        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn pruning_drops_only_stale_entries() {
        let now = SystemTime::now();
        let mut cache = Cache::in_memory();
        cache.record(
            &discovered([192, 168, 1, 42], "2.05.08"),
            now - Duration::from_secs(3600),
        );
        assert_eq!(cache.prune(now, Duration::from_secs(7200)).len(), 0);
        assert_eq!(cache.prune(now, Duration::from_secs(60)).len(), 1);
        assert!(cache.is_empty());
    }
}
