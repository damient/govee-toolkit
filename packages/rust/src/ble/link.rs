//! One connection to one device: the two characteristics, and the replies.
//!
//! A device takes one connection and stops advertising while it is up, so the
//! link is opened once and kept. Replies carry no request id, so a caller
//! subscribes before it writes and matches the answer against its command's
//! `reply:` layout.

use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::ble::wire::Peripheral;
use crate::ble::{FRAME_LEN, NOTIFY_CHARACTERISTIC, WRITE_CHARACTERISTIC};
use crate::transport::error::{Error, Result};

/// How many replies a subscriber may fall behind by before losing the oldest.
const REPLY_BACKLOG: usize = 32;

/// The notification task, stopped when the link is dropped.
#[derive(Debug)]
struct Listener(JoinHandle<()>);

impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Debug)]
pub(crate) struct Link {
    peripheral: Arc<dyn Peripheral>,
    replies: broadcast::Sender<Vec<u8>>,
    _listener: Listener,
}

impl Link {
    /// Connect, discover the vendor service and subscribe to notifications.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the device refuses the connection, if service
    /// discovery fails, or if it does not carry the characteristics this
    /// protocol needs.
    pub(crate) async fn open(peripheral: Arc<dyn Peripheral>, endpoint: &str) -> Result<Self> {
        if !peripheral
            .is_connected()
            .await
            .map_err(|e| adapter(endpoint, "reading the connection state", e))?
        {
            peripheral
                .connect()
                .await
                .map_err(|e| adapter(endpoint, "connecting", e))?;
        }
        let characteristics = peripheral
            .discover()
            .await
            .map_err(|e| adapter(endpoint, "discovering services", e))?;
        for uuid in [WRITE_CHARACTERISTIC, NOTIFY_CHARACTERISTIC] {
            if !characteristics.contains(&uuid) {
                return Err(Error::io(
                    format!("{endpoint}: the device carries no characteristic {uuid}"),
                    std::io::ErrorKind::Unsupported.into(),
                ));
            }
        }

        let mut stream = peripheral
            .subscribe(NOTIFY_CHARACTERISTIC)
            .await
            .map_err(|e| adapter(endpoint, "subscribing to notifications", e))?;

        let (replies, _) = broadcast::channel(REPLY_BACKLOG);
        let publish = replies.clone();
        let listener = Listener(tokio::spawn(async move {
            while let Some(frame) = stream.next().await {
                let _ = publish.send(frame);
            }
        }));

        Ok(Self {
            peripheral,
            replies,
            _listener: listener,
        })
    }

    /// Subscribe before you write the request, or a reply that arrives first
    /// is missed.
    pub(crate) fn replies(&self) -> broadcast::Receiver<Vec<u8>> {
        self.replies.subscribe()
    }

    pub(crate) async fn is_live(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }

    /// Write one frame, without waiting for a response.
    ///
    /// # Errors
    ///
    /// [`Error::Serialize`] if the frame is not the one length this wire
    /// carries, or [`Error::Io`] if the write fails.
    pub(crate) async fn write_frame(&self, cmd: &str, endpoint: &str, frame: &[u8]) -> Result<()> {
        check_length(cmd, frame)?;
        self.peripheral
            .write(WRITE_CHARACTERISTIC, frame)
            .await
            .map_err(|e| adapter(endpoint, "writing a frame", e))
    }
}

/// Refuse a frame this wire cannot carry.
///
/// # Errors
///
/// [`Error::Serialize`] for anything but exactly [`FRAME_LEN`] bytes. A short
/// frame padded here would reach the device as a command nobody wrote.
pub(crate) fn check_length(cmd: &str, frame: &[u8]) -> Result<()> {
    if frame.len() == FRAME_LEN {
        return Ok(());
    }
    Err(Error::Serialize {
        cmd: cmd.to_owned(),
        reason: format!(
            "the frame is {} bytes; this wire carries {FRAME_LEN}",
            frame.len()
        ),
    })
}

pub(crate) fn adapter(endpoint: &str, doing: &str, source: std::io::Error) -> Error {
    Error::io(format!("{endpoint}: {doing}"), source)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn only_a_frame_of_the_one_length_is_written() {
        assert!(check_length("power", &[0; FRAME_LEN]).is_ok());
        for len in [0, FRAME_LEN - 1, FRAME_LEN + 1] {
            let error = check_length("power", &vec![0; len]).expect_err("the wrong length");
            assert_eq!(error.code(), "serialize");
        }
    }
}
