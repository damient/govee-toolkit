//! Republishing the transports' events on the facade's own stream.
//!
//! One task per [`Govee`](super::Govee), stopped when the last clone is
//! dropped. It is also where a device the catalog cannot serve is reported —
//! discovery is the first moment that is knowable, and the only one where
//! saying it costs nothing.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::codec::Mode;
use crate::event::Event;
use crate::govee::Inner;

pub(super) struct Forwarder(pub(super) tokio::task::JoinHandle<()>);

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Republish transport events, and flag a device the catalog cannot encode for.
pub(super) async fn forward(inner: Arc<Inner>, out: broadcast::Sender<Event>) {
    let mut events = inner.lan.events();
    loop {
        match events.recv().await {
            Ok(event) => {
                if let crate::lan::Event::Discovered { device, .. } = &event {
                    let sku = inner
                        .config
                        .sku_for(&device.id)
                        .unwrap_or(&device.sku)
                        .to_owned();
                    match inner.catalog.device(&sku) {
                        Err(_) => {
                            tracing::warn!(id = %device.id, %sku, "no device file declares this SKU");
                            let _ = out.send(Event::UnknownSku {
                                id: device.id.clone(),
                                sku,
                            });
                        }
                        // Said once, on discovery, rather than swallowed on
                        // every send: without it, commands go out unverified.
                        Ok(file) if file.status_command(Mode::Lan).is_none() => {
                            tracing::warn!(
                                id = %device.id, %sku,
                                "no `commands.lan` entry is marked `role: status`; commands will not be verified"
                            );
                        }
                        Ok(_) => {}
                    }
                }
                let _ = out.send(Event::Lan(event));
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::warn!(missed, "the facade fell behind the transport's events");
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}
