//! Everything driven by what arrives on the socket, plus the two background
//! loops that keep the device list current.
//!
//! Dispatch is by payload shape rather than by command name: this crate holds
//! no list of commands, because that list lives in `devices/*.yaml`.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::broadcast;

use super::shared::{Shared, Tracked};
use crate::codec::Mode;
use crate::lan::discovery::{DiscoveredDevice, scan_request};
use crate::lan::socket::{MAX_DATAGRAM, parse_reply};
use crate::transport::DeviceId;
use crate::transport::error::Result;
use crate::transport::events::{Change, Event};
use crate::transport::status::DeviceStatus;

impl Shared {
    pub(super) async fn scan(&self, window: Duration) -> Result<Vec<DiscoveredDevice>> {
        let mut replies = self.replies.subscribe();
        self.socket
            .send_to(&scan_request(), self.endpoints.scan_target)
            .await?;

        let deadline = tokio::time::Instant::now() + window;
        let mut found: std::collections::BTreeMap<DeviceId, DiscoveredDevice> =
            std::collections::BTreeMap::new();
        loop {
            match tokio::time::timeout_at(deadline, replies.recv()).await {
                Ok(Ok(device)) => {
                    found.insert(device.id.clone(), device);
                }
                // A slow reader missing replies is not a failed scan; the next
                // one will see the device again.
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {}
                Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
            }
        }

        if !found.is_empty() {
            self.persist_cache();
        }
        Ok(found.into_values().collect())
    }

    /// Write the cache out without blocking the runtime.
    fn persist_cache(&self) {
        let Ok(cache) = self.cache.lock() else {
            return;
        };
        if cache.path().is_none() {
            return;
        }
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = cache.save() {
                tracing::warn!(error = %e, "could not write the device cache");
            }
        });
    }

    /// Route one datagram.
    ///
    /// A payload that carries an identity, an address and a SKU is a discovery
    /// reply; anything else from a device already known is a status. Both the
    /// documented `devStatus` and the undocumented `status` of
    /// `docs/protocol/lan.md` §2.2 therefore land in the right place.
    fn dispatch(&self, from: SocketAddr, bytes: &[u8]) {
        let Some(reply) = parse_reply(from, bytes) else {
            return;
        };

        if let Some(device) = DiscoveredDevice::from_data(&reply.data) {
            self.register(&device);
            let _ = self.replies.send(device);
            return;
        }

        if !reply.data.is_object() {
            return;
        }
        let Some(id) = self.identify(from.ip()) else {
            tracing::debug!(%from, cmd = %reply.cmd, "reply from an address no device is known at");
            return;
        };

        let status = DeviceStatus::from_data(id, reply.data);
        if let Ok(devices) = self.devices.lock()
            && let Some(tracked) = devices.get(&status.id)
        {
            let _ = tracked.status.send(Some(status.clone()));
        }
        let _ = self.events.send(Event::Status {
            mode: Mode::Lan,
            status,
        });
    }

    /// Which device answers at this address.
    ///
    /// A scan over the tracked devices: a household holds a handful, and a
    /// second index would have to be kept in step on every discovery.
    fn identify(&self, ip: IpAddr) -> Option<DeviceId> {
        let devices = self.devices.lock().ok()?;
        devices
            .iter()
            .find(|(_, tracked)| tracked.ip == ip)
            .map(|(id, _)| id.clone())
    }

    /// Record a discovery reply and make the device sendable.
    fn register(&self, device: &DiscoveredDevice) {
        let change = {
            let Ok(mut cache) = self.cache.lock() else {
                return;
            };
            cache.record(device, SystemTime::now())
        };

        {
            let Ok(mut devices) = self.devices.lock() else {
                return;
            };
            match devices.get_mut(&device.id) {
                Some(tracked) => {
                    tracked.ip = device.ip;
                    tracked.sku.clone_from(&device.sku);
                }
                None => {
                    devices.insert(
                        device.id.clone(),
                        Tracked::new(device.ip, device.sku.clone(), self.policy),
                    );
                }
            }
        }

        if change != Change::Refreshed {
            tracing::info!(id = %device.id, ip = %device.ip, sku = %device.sku, ?change, "device discovered");
        }
        let _ = self.events.send(Event::Discovered {
            mode: Mode::Lan,
            device: device.reported(&self.endpoints),
            change,
        });
    }

    /// Drop cached devices that have stopped answering scans.
    fn forget_stale(&self, older_than: Duration) {
        let dropped = {
            let Ok(mut cache) = self.cache.lock() else {
                return;
            };
            cache.prune(SystemTime::now(), older_than)
        };
        if dropped.is_empty() {
            return;
        }
        if let Ok(mut devices) = self.devices.lock() {
            for device in &dropped {
                devices.remove(&device.id);
            }
        }
        for device in dropped {
            let _ = self.events.send(Event::Forgotten {
                mode: Mode::Lan,
                id: device.id,
            });
        }
        self.persist_cache();
    }
}

pub(super) async fn receive_loop(shared: Arc<Shared>) {
    let mut buf = vec![0u8; MAX_DATAGRAM];
    loop {
        match shared.socket.recv_from(&mut buf).await {
            Ok((read, from)) => {
                if let Some(bytes) = buf.get(..read) {
                    shared.dispatch(from, bytes);
                } else {
                    tracing::warn!(read, "a datagram larger than the buffer was truncated");
                }
            }
            Err(e) => {
                // The error is the socket's, not a device's, and an immediate
                // retry would spin. Slow down and keep going.
                tracing::warn!(error = %e, "the lan receive loop failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

pub(super) async fn refresh_loop(
    shared: Arc<Shared>,
    interval: Duration,
    window: Duration,
    forget_after: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick fires immediately: the startup scan is this one, so a
    // caller does not have to remember to issue it.
    loop {
        ticker.tick().await;
        if let Err(e) = shared.scan(window).await {
            tracing::warn!(error = %e, "background scan failed");
        }
        shared.forget_stale(forget_after);
    }
}
