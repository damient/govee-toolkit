//! How long a link stays open after the last frame written on it.
//!
//! The write characteristic takes no acknowledgement, so nothing reports a
//! frame that the link dropped before it left. A device file records the wait
//! its unit needed as `measurements.ble.write_drain_ms`; a file that records
//! none falls back to [`Options::write_drain`](super::Options::write_drain).

use std::collections::BTreeMap;
use std::time::Duration;

use crate::codec::Catalog;

/// The wait each device file records, by SKU.
#[derive(Debug, Default)]
pub(super) struct Drains(BTreeMap<String, Duration>);

impl Drains {
    /// Read `measurements.ble.write_drain_ms` off every device file.
    pub(super) fn from_catalog(catalog: &Catalog) -> Self {
        let mut waits = BTreeMap::new();
        for sku in catalog.skus() {
            if let Ok(device) = catalog.device(sku)
                && let Some(ms) = device.measurements.ble.write_drain_ms
            {
                waits.insert(sku.to_owned(), Duration::from_millis(ms));
            }
        }
        Self(waits)
    }

    /// The wait recorded for a SKU, or `fallback` where its file records none.
    pub(super) fn get(&self, sku: Option<&str>, fallback: Duration) -> Duration {
        sku.and_then(|sku| self.0.get(sku))
            .copied()
            .unwrap_or(fallback)
    }

    /// The longest wait among the SKUs, so links closed together are held open
    /// once rather than one after the other.
    pub(super) fn longest<'a>(
        &self,
        skus: impl IntoIterator<Item = Option<&'a str>>,
        fallback: Duration,
    ) -> Duration {
        skus.into_iter()
            .map(|sku| self.get(sku, fallback))
            .max()
            .unwrap_or(fallback)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn drains() -> Drains {
        Drains(BTreeMap::from([
            ("SLOW".to_owned(), Duration::from_millis(300)),
            ("FAST".to_owned(), Duration::from_millis(10)),
        ]))
    }

    #[test]
    fn a_sku_whose_file_records_no_wait_takes_the_fallback() {
        let fallback = Duration::from_millis(50);
        assert_eq!(
            drains().get(Some("SLOW"), fallback),
            Duration::from_millis(300)
        );
        assert_eq!(drains().get(Some("OTHER"), fallback), fallback);
        assert_eq!(drains().get(None, fallback), fallback);
    }

    #[test]
    fn links_closed_together_wait_for_the_slowest_of_them() {
        let fallback = Duration::from_millis(50);
        let waits = drains().longest([Some("FAST"), Some("SLOW"), None], fallback);
        assert_eq!(waits, Duration::from_millis(300));
    }

    #[test]
    fn closing_nothing_waits_the_fallback() {
        let fallback = Duration::from_millis(50);
        assert_eq!(drains().longest([], fallback), fallback);
    }
}
