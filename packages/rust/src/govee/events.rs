//! Republish the transports' events on the facade's own stream.
//!
//! One task per transport; the tasks stop when the last [`Govee`](super::Govee)
//! clone drops. A device the catalog cannot serve is reported here, because
//! discovery is the first moment that is knowable.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::event::Event;
use crate::govee::Inner;

pub(super) struct Forwarder(pub(super) Vec<tokio::task::JoinHandle<()>>);

impl Drop for Forwarder {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// Republish one transport's events, and flag a device the catalog cannot
/// encode for.
pub(super) async fn forward(
    inner: Arc<Inner>,
    mut events: broadcast::Receiver<crate::transport::Event>,
    out: broadcast::Sender<Event>,
) {
    loop {
        match events.recv().await {
            Ok(event) => {
                if let crate::transport::Event::Discovered { mode, device, .. } = &event {
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
                        // Said once, on discovery: without a status command,
                        // commands go out unverified.
                        Ok(file) if file.status_command(*mode).is_none() => {
                            tracing::warn!(
                                id = %device.id, %sku, %mode,
                                "no entry in this mode's command table is marked `role: status`; commands will not be verified"
                            );
                        }
                        Ok(_) => {}
                    }
                }
                let _ = out.send(Event::Transport(event));
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::warn!(missed, "the facade fell behind a transport's events");
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}
