//! The simulator, joined to the transport's `wire` traits.
//!
//! `crates/sim` speaks GATT and knows nothing of this crate. This is the whole
//! of what sits between the two: no frame is read here, and no state is kept
//! here.

#![allow(dead_code, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use govee_toolkit::ble::wire::{Adapter, Heard, Notifications, Peripheral};
use govee_toolkit_sim::ble::{BleAdapter, BleDevice};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

/// The simulated radio, as the transport reaches it.
#[derive(Debug)]
pub(crate) struct Radio {
    adapter: Arc<BleAdapter>,
    /// How long a connection takes, by handle. Nothing else in this harness
    /// spends time, and on hardware a connection costs seconds.
    connect_delay: HashMap<String, Duration>,
}

impl Radio {
    pub(crate) fn new(adapter: Arc<BleAdapter>) -> Self {
        Self {
            adapter,
            connect_delay: HashMap::new(),
        }
    }

    pub(crate) fn slow_to_connect(mut self, endpoint: &str, delay: Duration) -> Self {
        self.connect_delay.insert(endpoint.to_owned(), delay);
        self
    }
}

#[async_trait]
impl Adapter for Radio {
    async fn start_scan(&self) -> std::io::Result<()> {
        self.adapter.start_scan();
        Ok(())
    }

    async fn stop_scan(&self) -> std::io::Result<()> {
        self.adapter.stop_scan();
        Ok(())
    }

    async fn heard(&self) -> std::io::Result<Vec<Heard>> {
        Ok(self
            .adapter
            .heard()
            .into_iter()
            .map(|on_air| Heard {
                endpoint: on_air.endpoint,
                name: on_air.name,
                adverts: on_air.adverts,
            })
            .collect())
    }

    async fn peripheral(&self, endpoint: &str) -> std::io::Result<Option<Arc<dyn Peripheral>>> {
        let delay = self
            .connect_delay
            .get(endpoint)
            .copied()
            .unwrap_or_default();
        Ok(self
            .adapter
            .peripheral(endpoint)
            .map(|device| Arc::new(Peer { device, delay }) as Arc<dyn Peripheral>))
    }
}

/// One simulated device, as the link reaches it.
#[derive(Debug)]
pub(crate) struct Peer {
    device: BleDevice,
    delay: Duration,
}

#[async_trait]
impl Peripheral for Peer {
    async fn is_connected(&self) -> std::io::Result<bool> {
        Ok(self.device.is_connected())
    }

    async fn connect(&self) -> std::io::Result<()> {
        tokio::time::sleep(self.delay).await;
        self.device.connect()
    }

    async fn discover(&self) -> std::io::Result<Vec<Uuid>> {
        Ok(self.device.characteristics())
    }

    async fn subscribe(&self, characteristic: Uuid) -> std::io::Result<Notifications> {
        if characteristic != govee_toolkit_sim::ble::NOTIFY_CHARACTERISTIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("the device notifies nothing on {characteristic}"),
            ));
        }
        let replies = self.device.notifications();
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
        self.device.write(characteristic, frame)
    }
}
