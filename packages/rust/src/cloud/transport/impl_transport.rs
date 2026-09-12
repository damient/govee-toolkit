//! `cloud` as one implementation of [`crate::transport::Transport`]: every
//! method forwards to the inherent one on [`super::Transport`].

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, watch};

use super::Transport as CloudTransport;
use crate::codec::{Encoded, Mode};
use crate::transport::error::{Error, Result};
use crate::transport::{
    DeviceId, DeviceStatus, Discovered, Event, Health, KnownDevice, Reply, Sent, Transport, Verify,
};

#[async_trait]
impl Transport for CloudTransport {
    fn mode(&self) -> Mode {
        Mode::Cloud
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

    fn scan_window(&self) -> Duration {
        Self::scan_window(self)
    }

    /// Lists the account's devices. The window is ignored: there is nothing
    /// to listen for.
    async fn scan(&self, _window: Duration) -> Result<Vec<Discovered>> {
        Self::scan(self).await
    }

    /// The window is ignored, for the reason [`Transport::scan`] gives.
    async fn scan_for(&self, id: &DeviceId, _window: Duration) -> Result<Option<Discovered>> {
        Self::scan_for(self, id).await
    }

    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify) -> Result<Sent> {
        Self::send(self, id, command, verify).await
    }

    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        Self::status(self, id, request).await
    }

    /// Always fails: this mode answers in JSON, so no command declares a
    /// `reply:` layout for it. What the API reported reaches a caller whole,
    /// under [`DeviceStatus::raw`].
    async fn read(&self, _id: &DeviceId, request: &Encoded) -> Result<Reply> {
        Err(Error::no_reply_layout(Mode::Cloud, &request.cmd))
    }
}
