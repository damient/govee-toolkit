//! What the SDK reports while it runs, and what one device answers.

use std::sync::Arc;

use govee_toolkit::{DeviceStatus as CoreStatus, Event, Govee};
use napi::Env;
use napi::bindgen_prelude::PromiseRaw;
use napi_derive::napi;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{Mutex, watch};

use crate::promise::promise;
use crate::types::DeviceStatus;

/// The events of one SDK. Iterate it with `for await`.
///
/// Every event is a plain object, and `event` says which one it is. The
/// records are the core's own, so `govee watch --json` prints the same ones.
/// A subscription that falls behind reports `{ event: "lagged", missed: n }`
/// rather than hide the gap.
#[napi]
pub struct EventStream {
    events: Arc<Mutex<Receiver<Event>>>,
}

impl EventStream {
    pub(crate) fn new(govee: &Govee) -> Self {
        Self {
            events: Arc::new(Mutex::new(govee.events())),
        }
    }
}

#[napi]
impl EventStream {
    /// The next event, or `null` once the SDK that reported them is gone.
    #[napi]
    pub fn next<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Option<serde_json::Value>>> {
        let events = Arc::clone(&self.events);
        promise(env, async move {
            Ok(match events.lock().await.recv().await {
                Ok(event) => Some(event.to_json()),
                Err(RecvError::Lagged(missed)) => Some(Event::lagged(missed)),
                Err(RecvError::Closed) => None,
            })
        })
    }
}

/// One device's status, as answers arrive. Iterate it with `for await`.
///
/// It requests nothing: it reports the answers a status request or a
/// verification already brought back.
#[napi]
pub struct StatusStream {
    statuses: Arc<Mutex<watch::Receiver<Option<CoreStatus>>>>,
}

impl StatusStream {
    pub(crate) fn new(statuses: watch::Receiver<Option<CoreStatus>>) -> Self {
        Self {
            statuses: Arc::new(Mutex::new(statuses)),
        }
    }
}

#[napi]
impl StatusStream {
    /// The next status, or `null` once the transport that heard this device
    /// is gone.
    #[napi]
    pub fn next<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Option<DeviceStatus>>> {
        let statuses = Arc::clone(&self.statuses);
        promise(env, async move {
            let mut guard = statuses.lock().await;
            loop {
                if guard.changed().await.is_err() {
                    return Ok(None);
                }
                let latest = guard.borrow_and_update().clone();
                if let Some(status) = latest {
                    return Ok(Some(DeviceStatus::from(status)));
                }
            }
        })
    }
}
