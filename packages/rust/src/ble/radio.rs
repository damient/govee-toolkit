//! [`wire`](super::wire) over the platform's Bluetooth stack.
//!
//! The only module that names `btleplug`. It reports what the platform said:
//! no frame is read here, and no advertisement is parsed here.

use std::sync::Arc;

use async_trait::async_trait;
use btleplug::api::{
    Central as _, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter as Central, Manager, Peripheral as Peer};
use futures_util::StreamExt as _;
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::ble::wire::{Heard, Notifications};

/// The machine's first Bluetooth adapter.
///
/// The adapter is claimed on first use, so a transport starts on a machine
/// whose radio is off and the first command is what reports it.
#[derive(Debug, Default)]
pub struct Radio {
    central: OnceCell<Central>,
}

impl Radio {
    /// A radio that has claimed nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The adapter, claimed if it has not been already.
    async fn central(&self) -> std::io::Result<&Central> {
        self.central
            .get_or_try_init(|| async {
                let manager = Manager::new().await.map_err(|e| io(&e))?;
                manager
                    .adapters()
                    .await
                    .map_err(|e| io(&e))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "this machine reports no Bluetooth adapter",
                        )
                    })
            })
            .await
    }
}

#[async_trait]
impl crate::ble::wire::Adapter for Radio {
    async fn start_scan(&self) -> std::io::Result<()> {
        self.central()
            .await?
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| io(&e))
    }

    async fn stop_scan(&self) -> std::io::Result<()> {
        self.central().await?.stop_scan().await.map_err(|e| io(&e))
    }

    async fn heard(&self) -> std::io::Result<Vec<Heard>> {
        let peripherals = self
            .central()
            .await?
            .peripherals()
            .await
            .map_err(|e| io(&e))?;
        let mut heard = Vec::with_capacity(peripherals.len());
        for peripheral in peripherals {
            // A peripheral the platform has forgotten, or one that advertises
            // no name, is not an error. It is one fewer device on the air.
            let Ok(Some(properties)) = peripheral.properties().await else {
                continue;
            };
            let Some(name) = properties
                .local_name
                .or(properties.advertisement_name)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            heard.push(Heard {
                endpoint: peripheral.id().to_string(),
                name,
            });
        }
        Ok(heard)
    }

    async fn peripheral(
        &self,
        endpoint: &str,
    ) -> std::io::Result<Option<Arc<dyn crate::ble::wire::Peripheral>>> {
        let peripherals = self
            .central()
            .await?
            .peripherals()
            .await
            .map_err(|e| io(&e))?;
        let mut matching = peripherals
            .into_iter()
            .filter(|p| p.id().to_string().eq_ignore_ascii_case(endpoint));
        let Some(found) = matching.next() else {
            return Ok(None);
        };
        let others = matching.count();
        if others > 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "{} peripherals carry this handle, so it names none of them",
                    others + 1
                ),
            ));
        }
        Ok(Some(Arc::new(Device(found))))
    }
}

/// One peripheral the platform holds.
#[derive(Debug)]
struct Device(Peer);

impl Device {
    /// The characteristic under a UUID, among those discovery found.
    fn characteristic(&self, uuid: Uuid) -> std::io::Result<Characteristic> {
        self.0
            .characteristics()
            .into_iter()
            .find(|c| c.uuid == uuid)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    format!("the device carries no characteristic {uuid}"),
                )
            })
    }
}

#[async_trait]
impl crate::ble::wire::Peripheral for Device {
    async fn is_connected(&self) -> std::io::Result<bool> {
        self.0.is_connected().await.map_err(|e| io(&e))
    }

    async fn connect(&self) -> std::io::Result<()> {
        self.0.connect().await.map_err(|e| io(&e))
    }

    async fn discover(&self) -> std::io::Result<Vec<Uuid>> {
        self.0.discover_services().await.map_err(|e| io(&e))?;
        Ok(self.0.characteristics().iter().map(|c| c.uuid).collect())
    }

    async fn subscribe(&self, characteristic: Uuid) -> std::io::Result<Notifications> {
        let subject = self.characteristic(characteristic)?;
        self.0.subscribe(&subject).await.map_err(|e| io(&e))?;
        let stream = self.0.notifications().await.map_err(|e| io(&e))?;
        Ok(Box::pin(stream.filter_map(
            move |notification| async move {
                (notification.uuid == characteristic).then_some(notification.value)
            },
        )))
    }

    async fn write(&self, characteristic: Uuid, frame: &[u8]) -> std::io::Result<()> {
        let subject = self.characteristic(characteristic)?;
        self.0
            .write(&subject, frame, WriteType::WithoutResponse)
            .await
            .map_err(|e| io(&e))
    }
}

fn io(source: &btleplug::Error) -> std::io::Error {
    std::io::Error::other(source.to_string())
}
