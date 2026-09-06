//! `lan` as one implementation of [`crate::transport::Transport`].
//!
//! Every method here forwards to the inherent one above it. They are kept
//! separate because the inherent surface says more than the trait can: a `lan`
//! caller gets [`DiscoveredDevice`] with the address and the four firmware
//! strings a reply carries, where the trait reports the shape every mode can
//! fill in.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, watch};

use super::Transport as LanTransport;
use crate::codec::{Encoded, Mode};
use crate::transport::error::Result;
use crate::transport::{
    DeviceId, DeviceStatus, Discovered, Event, Health, KnownDevice, Sent, Transport, Verify,
};

#[async_trait]
impl Transport for LanTransport {
    fn mode(&self) -> Mode {
        Mode::Lan
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
        let endpoints = self.endpoints();
        Ok(Self::scan(self, window)
            .await?
            .iter()
            .map(|device| device.reported(&endpoints))
            .collect())
    }

    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent> {
        Self::send(self, id, command, verify).await
    }

    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        Self::status(self, id, request).await
    }

    fn save_cache(&self) -> Result<()> {
        Self::save_cache(self)
    }
}
