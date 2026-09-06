//! Scanning: listening for advertisements, and what the transport does with
//! one.

use std::time::Duration;

use btleplug::api::{Central as _, Peripheral as _, ScanFilter};
use btleplug::platform::Adapter;

use crate::ble::link::adapter as adapter_error;
use crate::ble::scan::Advertised;
use crate::ble::transport::shared::{Shared, Tracked};
use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::error::Result;
use crate::transport::events::{Change, Discovered, Event};

impl Shared {
    /// Listen for advertisements and record what answered.
    ///
    /// Two passes: a device that has just dropped a connection takes seconds
    /// to advertise again, so a first pass that heard nothing is followed by a
    /// longer one rather than reported as an empty network.
    ///
    /// # Errors
    ///
    /// [`Error::Io`](crate::transport::Error::Io) if no adapter is available
    /// or the scan cannot be started.
    pub(super) async fn scan(&self, window: Duration) -> Result<Vec<Discovered>> {
        let adapter = self.adapter().await?;
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| adapter_error("ble", "starting a scan", &e))?;

        tokio::time::sleep(window).await;
        let mut seen = collect(adapter).await?;
        if seen.is_empty() {
            tokio::time::sleep(self.options.rescan_window).await;
            seen = collect(adapter).await?;
        }

        if let Err(e) = adapter.stop_scan().await {
            tracing::debug!(error = %e, "the ble scan could not be stopped");
        }
        Ok(self.adopt(seen))
    }

    /// Record what a scan heard, and report each device with what changed
    /// about it.
    fn adopt(&self, seen: Vec<Advertised>) -> Vec<Discovered> {
        let Ok(mut devices) = self.devices.lock() else {
            return Vec::new();
        };
        let mut found = Vec::with_capacity(seen.len());
        for device in seen {
            let known = devices.iter().find_map(|(id, tracked)| {
                tracked
                    .address
                    .eq_ignore_ascii_case(&device.address)
                    .then(|| id.clone())
            });
            let (id, change) = match known {
                Some(id) => (id, Change::Refreshed),
                None => (DeviceId::new(&device.address), Change::New),
            };
            devices
                .entry(id.clone())
                .and_modify(|tracked| device.sku.clone_into(&mut tracked.sku))
                .or_insert_with(|| {
                    Tracked::new(
                        device.address.clone(),
                        device.sku.clone(),
                        &self.options,
                        self.budget,
                    )
                });

            let reported = Discovered {
                id,
                endpoint: device.address,
                sku: device.sku,
                // An advertisement carries no version, and asking for one means
                // connecting and reading a command this layer does not name.
                firmware: None,
            };
            let _ = self.events.send(Event::Discovered {
                mode: Mode::Ble,
                device: reported.clone(),
                change,
            });
            found.push(reported);
        }
        found
    }
}

/// Read the advertisements the adapter has collected so far.
async fn collect(adapter: &Adapter) -> Result<Vec<Advertised>> {
    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| adapter_error("ble", "listing what the scan heard", &e))?;

    let mut seen = Vec::new();
    for peripheral in peripherals {
        // A peripheral the platform has since forgotten is not an error: it is
        // one fewer device on the air.
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
        if let Some(device) = Advertised::read(properties.address.to_string(), &name) {
            seen.push(device);
        }
    }
    Ok(seen)
}
