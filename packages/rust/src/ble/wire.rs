//! The two seams between the protocol and a Bluetooth stack.
//!
//! Above these traits everything is protocol; below them is one platform's
//! radio. [`Transport::with_adapter`] takes an adapter that is not a radio,
//! which is how `crates/sim` runs the send path on a machine with none.
//!
//! A method answers [`std::io::Error`]: an implementation reports what the
//! platform said, and the caller names the device.
//!
//! [`Transport::with_adapter`]: super::Transport::with_adapter

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream::Stream;
use uuid::Uuid;

/// The frames a device notifies, in the order they arrive.
pub type Notifications = Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;

/// One advertisement, before anything reads a SKU out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heard {
    /// The handle to connect to. See [`Advertised::endpoint`].
    ///
    /// [`Advertised::endpoint`]: super::Advertised::endpoint
    pub endpoint: String,
    /// The name the device advertises.
    pub name: String,
}

/// One peripheral, as the link uses it.
#[async_trait]
pub trait Peripheral: std::fmt::Debug + Send + Sync {
    /// Whether the link is up.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports.
    async fn is_connected(&self) -> std::io::Result<bool>;

    /// Open the link. A device takes one connection at a time.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports.
    async fn connect(&self) -> std::io::Result<()>;

    /// Discover the services, and answer every characteristic they carry.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports.
    async fn discover(&self) -> std::io::Result<Vec<Uuid>>;

    /// Subscribe to a characteristic, and answer the frames it notifies.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports, and [`std::io::ErrorKind::Unsupported`]
    /// if the device carries no such characteristic.
    async fn subscribe(&self, characteristic: Uuid) -> std::io::Result<Notifications>;

    /// Write one frame, without waiting for a response.
    ///
    /// # Errors
    ///
    /// As for [`Peripheral::subscribe`].
    async fn write(&self, characteristic: Uuid, frame: &[u8]) -> std::io::Result<()>;
}

/// One radio, as the transport uses it.
#[async_trait]
pub trait Adapter: std::fmt::Debug + Send + Sync {
    /// Start listening for advertisements.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports, including having no radio at all.
    async fn start_scan(&self) -> std::io::Result<()>;

    /// Stop listening.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports.
    async fn stop_scan(&self) -> std::io::Result<()>;

    /// Every advertisement the radio holds. A scan reads this, so it answers
    /// what has been heard so far and does not wait.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports.
    async fn heard(&self) -> std::io::Result<Vec<Heard>>;

    /// The peripheral behind a handle, or `None` if the radio holds none.
    ///
    /// # Errors
    ///
    /// Whatever the platform reports, and
    /// [`std::io::ErrorKind::InvalidData`] if several peripherals carry the
    /// handle, because it then names none of them.
    async fn peripheral(&self, endpoint: &str) -> std::io::Result<Option<Arc<dyn Peripheral>>>;
}
