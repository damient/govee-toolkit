//! The simulator, joined to the transport's `wire` traits.
//!
//! `crates/sim` speaks GATT and knows nothing of this crate. This is the whole
//! of what sits between the two: no frame is read here, and no state is kept
//! here.

#![allow(dead_code, clippy::expect_used)]

use std::sync::Arc;

use async_trait::async_trait;
use govee_toolkit::ble::wire::{Adapter, Heard, Notifications, Peripheral};
use govee_toolkit_sim::ble::{BleAdapter, BleDevice};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

/// The simulated radio, as the transport reaches it.
#[derive(Debug)]
pub(crate) struct Radio(pub(crate) Arc<BleAdapter>);

#[async_trait]
impl Adapter for Radio {
    async fn start_scan(&self) -> std::io::Result<()> {
        self.0.start_scan();
        Ok(())
    }

    async fn stop_scan(&self) -> std::io::Result<()> {
        self.0.stop_scan();
        Ok(())
    }

    async fn heard(&self) -> std::io::Result<Vec<Heard>> {
        Ok(self
            .0
            .heard()
            .into_iter()
            .map(|(endpoint, name)| Heard { endpoint, name })
            .collect())
    }

    async fn peripheral(&self, endpoint: &str) -> std::io::Result<Option<Arc<dyn Peripheral>>> {
        Ok(self
            .0
            .peripheral(endpoint)
            .map(|device| Arc::new(Peer(device)) as Arc<dyn Peripheral>))
    }
}

/// One simulated device, as the link reaches it.
#[derive(Debug)]
pub(crate) struct Peer(pub(crate) BleDevice);

#[async_trait]
impl Peripheral for Peer {
    async fn is_connected(&self) -> std::io::Result<bool> {
        Ok(self.0.is_connected())
    }

    async fn connect(&self) -> std::io::Result<()> {
        self.0.connect()
    }

    async fn discover(&self) -> std::io::Result<Vec<Uuid>> {
        Ok(self.0.characteristics())
    }

    async fn subscribe(&self, characteristic: Uuid) -> std::io::Result<Notifications> {
        if characteristic != govee_toolkit_sim::ble::NOTIFY_CHARACTERISTIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("the device notifies nothing on {characteristic}"),
            ));
        }
        let replies = self.0.notifications();
        Ok(Box::pin(futures_util::stream::unfold(
            replies,
            |mut replies| async move {
                loop {
                    match replies.recv().await {
                        Ok(frame) => return Some((frame, replies)),
                        // A subscriber that fell behind reads the next frame.
                        Err(RecvError::Lagged(_)) => {}
                        Err(RecvError::Closed) => return None,
                    }
                }
            },
        )))
    }

    async fn write(&self, characteristic: Uuid, frame: &[u8]) -> std::io::Result<()> {
        self.0.write(characteristic, frame)
    }
}
