//! `ble` as one implementation of [`crate::transport::Transport`].
//!
//! Every method forwards to the inherent one. The inherent surface says more
//! than the trait can: [`BleTransport::bind`] has no place in a trait every
//! mode implements, because no other mode addresses a device by something
//! other than its identity.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, watch};

use super::Transport as BleTransport;
use crate::codec::{Encoded, Mode};
use crate::transport::error::Result;
use crate::transport::{
    DeviceId, DeviceStatus, Discovered, Event, Health, KnownDevice, Reply, Sent, Transport, Verify,
};

#[async_trait]
impl Transport for BleTransport {
    fn mode(&self) -> Mode {
        Mode::Ble
    }

    fn events(&self) -> broadcast::Receiver<Event> {
        Self::events(self)
    }

    fn devices(&self) -> Vec<KnownDevice> {
        Self::devices(self)
    }

    fn sku(&self, id: &DeviceId) -> Option<String> {
        Self::sku(self, id)
    }

    fn health(&self, id: &DeviceId) -> Option<Health> {
        Self::health(self, id)
    }

    fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        Self::last_status(self, id)
    }

    fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        Self::watch_status(self, id)
    }

    async fn scan(&self, window: Duration) -> Result<Vec<Discovered>> {
        Self::scan(self, window).await
    }

    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent> {
        Self::send(self, id, command, verify).await
    }

    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        Self::status(self, id, request).await
    }

    async fn read(&self, id: &DeviceId, request: &Encoded) -> Result<Reply> {
        Self::read(self, id, request).await
    }

    /// Does nothing, successfully.
    ///
    /// There is no `ble` cache: an address works only while the adapter still
    /// holds the peripheral, so a saved one would promise a device that a
    /// restart cannot reach.
    fn save_cache(&self) -> Result<()> {
        Ok(())
    }
}
