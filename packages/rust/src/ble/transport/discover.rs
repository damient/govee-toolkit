//! Advertisement scans, and what the transport records from one.

use std::collections::HashSet;
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

    /// Listen for advertisements until one device is heard.
    ///
    /// The adapter reports what it heard so far, so this reads it every
    /// [`POLL`] rather than at the end of the window. A second pass runs under
    /// the same rule as [`Shared::scan`]: only where the first pass heard
    /// nothing at all.
    ///
    /// A caller that drops this future leaves the adapter scanning. The next
    /// scan starts it again, which the platform accepts.
    ///
    /// # Errors
    ///
    /// As for [`Shared::scan`].
    pub(super) async fn scan_for(
        &self,
        id: &DeviceId,
        window: Duration,
    ) -> Result<Option<Discovered>> {
        let adapter = self.adapter.as_ref();
        adapter
            .start_scan()
            .await
            .map_err(|e| adapter_error("ble", "starting a scan", e))?;

        let mut adopted = HashSet::new();
        let (mut found, heard) = self.listen_for(id, window, &mut adopted).await?;
        if found.is_none() && !heard {
            (found, _) = self
                .listen_for(id, self.options.rescan_window, &mut adopted)
                .await?;
        }

        if let Err(e) = adapter.stop_scan().await {
            tracing::debug!(error = %e, "the ble scan could not be stopped");
        }
        Ok(found)
    }

    /// Read what the adapter heard until the device appears or `window` runs
    /// out. Answers the device, and whether anything at all was heard.
    ///
    /// `adopted` carries the endpoints already recorded, so a device heard
    /// over two passes is reported once.
    async fn listen_for(
        &self,
        id: &DeviceId,
        window: Duration,
        adopted: &mut HashSet<String>,
    ) -> Result<(Option<Discovered>, bool)> {
        let adapter = self.adapter.as_ref();
        let deadline = tokio::time::Instant::now() + window;
        let mut heard = false;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Ok((None, heard));
            }
            tokio::time::sleep(POLL.min(left)).await;

            let seen = collect(adapter).await?;
            heard |= !seen.is_empty();
            let fresh: Vec<Advertised> = seen
                .into_iter()
                .filter(|device| adopted.insert(device.endpoint.clone()))
                .collect();
            if let Some(device) = self.adopt(fresh).into_iter().find(|found| &found.id == id) {
                return Ok((Some(device), heard));
            }
        }
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
            devices
                .entry(id.clone())
                .and_modify(|tracked| {
                    // A device that turns out to be another SKU must not keep a
                    // budget taken from the wrong device file. Replacing the
                    // pacer refills its bucket, so it happens only on a change.
                    if tracked.sku != device.sku {
                        device.sku.clone_into(&mut tracked.sku);
                        tracked.pacer = Arc::new(Pacer::new(self.budget_for(&device.sku)));
                    }
                })
                .or_insert_with(|| {
                    Tracked::new(
                        device.endpoint.clone(),
                        device.sku.clone(),
                        self.options.policy,
                        self.budget_for(&device.sku),
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

/// How often a scan that looks for one device reads what the adapter heard.
/// The radio hears an advertisement when it arrives; this is only how soon the
/// transport can act on one.
const POLL: Duration = Duration::from_millis(200);

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
