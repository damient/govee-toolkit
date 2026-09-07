//! Advertisement scans, and what the transport records from one.

use std::sync::Arc;
use std::time::Duration;

use crate::ble::link::adapter as adapter_error;
use crate::ble::pace::Pacer;
use crate::ble::scan::Advertised;
use crate::ble::transport::shared::{Shared, Tracked, id_at};
use crate::ble::wire::Adapter;
use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::error::Result;
use crate::transport::events::{Change, Discovered, Event};

impl Shared {
    /// Listen for advertisements and record what answered.
    ///
    /// A device that has just dropped a connection takes seconds to advertise
    /// again. If the first pass hears nothing, a longer second pass runs.
    ///
    /// # Errors
    ///
    /// [`Error::Io`](crate::transport::Error::Io) if no adapter is available
    /// or the scan cannot be started.
    pub(super) async fn scan(&self, window: Duration) -> Result<Vec<Discovered>> {
        let adapter = self.adapter.as_ref();
        adapter
            .start_scan()
            .await
            .map_err(|e| adapter_error("ble", "starting a scan", e))?;

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

    /// Record what a scan heard, and report what changed about each device.
    fn adopt(&self, seen: Vec<Advertised>) -> Vec<Discovered> {
        let Ok(mut devices) = self.devices.lock() else {
            return Vec::new();
        };
        let mut found = Vec::with_capacity(seen.len());
        for device in seen {
            let (id, change) = match id_at(&devices, &device.endpoint) {
                Some(id) => (id, Change::Refreshed),
                None => (DeviceId::new(&device.endpoint), Change::New),
            };
            let budget = self.budget_for(&device.sku);
            devices
                .entry(id.clone())
                .and_modify(|tracked| {
                    // A device that turns out to be another SKU must not keep a
                    // budget taken from the wrong device file. Replacing the
                    // pacer refills its bucket, so it happens only on a change.
                    if tracked.sku != device.sku {
                        device.sku.clone_into(&mut tracked.sku);
                        tracked.pacer = Arc::new(Pacer::new(budget));
                    }
                })
                .or_insert_with(|| {
                    Tracked::new(
                        device.endpoint.clone(),
                        device.sku.clone(),
                        self.options.policy,
                        budget,
                    )
                });

            let reported = Discovered {
                id,
                endpoint: device.endpoint,
                sku: device.sku,
                // An advertisement carries no version. A version needs a
                // connection and a command this layer does not name.
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

/// Read the advertisements the adapter holds, and keep the ones this
/// transport recognizes.
async fn collect(adapter: &dyn Adapter) -> Result<Vec<Advertised>> {
    let heard = adapter
        .heard()
        .await
        .map_err(|e| adapter_error("ble", "listing what the scan heard", e))?;
    Ok(heard
        .into_iter()
        .filter_map(|device| Advertised::read(device.endpoint, &device.name))
        .collect())
}
