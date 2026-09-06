//! One connection to one device: the two characteristics, and the replies.
//!
//! A device accepts a single connection and stops advertising while it is up,
//! so the link is opened once and kept. Replies arrive on the notify
//! characteristic with nothing to correlate them by except their own leading
//! bytes — [`answers`] is that correlation, and it is why a caller must send
//! its request and read the reply under one subscription.

use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use futures_util::StreamExt as _;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::ble::{FRAME_LEN, NOTIFY_CHARACTERISTIC, WRITE_CHARACTERISTIC};
use crate::transport::error::{Error, Result};

/// How many replies a subscriber may fall behind by before losing the oldest.
const REPLY_BACKLOG: usize = 32;

/// Whether a notification answers a request.
///
/// The wire carries no request id, so a reply is matched by its first two
/// bytes repeating the request's. Unverified: nothing in this repository has
/// captured a reply — see [`crate::ble`]. TODO: confirm the correlation, or
/// replace it, once `docs/protocol/ble.md` describes a reply.
#[must_use]
pub fn answers(reply: &[u8], request: &[u8]) -> bool {
    match (reply.get(..2), request.get(..2)) {
        (Some(reply), Some(request)) => reply == request,
        _ => false,
    }
}

/// The notification task, stopped when the link is dropped.
#[derive(Debug)]
struct Listener(JoinHandle<()>);

impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A connected device.
#[derive(Debug)]
pub(crate) struct Link {
    peripheral: Peripheral,
    write: Characteristic,
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
    pub(crate) async fn open(peripheral: Peripheral, endpoint: &str) -> Result<Self> {
        if !peripheral
            .is_connected()
            .await
            .map_err(|e| adapter(endpoint, "reading the connection state", &e))?
        {
            peripheral
                .connect()
                .await
                .map_err(|e| adapter(endpoint, "connecting", &e))?;
        }
        peripheral
            .discover_services()
            .await
            .map_err(|e| adapter(endpoint, "discovering services", &e))?;

        let characteristics = peripheral.characteristics();
        let find = |uuid| {
            characteristics
                .iter()
                .find(|c| c.uuid == uuid)
                .cloned()
                .ok_or_else(|| {
                    Error::io(
                        format!("{endpoint}: the device carries no characteristic {uuid}"),
                        std::io::ErrorKind::Unsupported.into(),
                    )
                })
        };
        let write = find(WRITE_CHARACTERISTIC)?;
        let notify = find(NOTIFY_CHARACTERISTIC)?;

        peripheral
            .subscribe(&notify)
            .await
            .map_err(|e| adapter(endpoint, "subscribing to notifications", &e))?;
        let mut stream = peripheral
            .notifications()
            .await
            .map_err(|e| adapter(endpoint, "opening the notification stream", &e))?;

        let (replies, _) = broadcast::channel(REPLY_BACKLOG);
        let publish = replies.clone();
        let listener = Listener(tokio::spawn(async move {
            while let Some(notification) = stream.next().await {
                if notification.uuid == NOTIFY_CHARACTERISTIC {
                    let _ = publish.send(notification.value);
                }
            }
        }));

        Ok(Self {
            peripheral,
            write,
            replies,
            _listener: listener,
        })
    }

    /// Subscribe to what the device notifies.
    ///
    /// Subscribe before writing the request, or a reply that arrives first is
    /// missed.
    pub(crate) fn replies(&self) -> broadcast::Receiver<Vec<u8>> {
        self.replies.subscribe()
    }

    /// Whether the connection is still up.
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
            .write(&self.write, frame, WriteType::WithoutResponse)
            .await
            .map_err(|e| adapter(endpoint, "writing a frame", &e))
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

/// What the adapter reported, as a transport error.
pub(crate) fn adapter(endpoint: &str, doing: &str, source: &btleplug::Error) -> Error {
    Error::io(
        format!("{endpoint}: {doing}"),
        std::io::Error::other(source.to_string()),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_reply_is_matched_by_the_two_bytes_that_name_it() {
        assert!(answers(&[0xaa, 0x04, 0x64], &[0xaa, 0x04, 0x00]));
        assert!(!answers(&[0xaa, 0x01, 0x01], &[0xaa, 0x04, 0x00]));
        assert!(!answers(&[0x33, 0x04, 0x64], &[0xaa, 0x04, 0x00]));
    }

    #[test]
    fn only_a_frame_of_the_one_length_is_written() {
        assert!(check_length("power", &[0; FRAME_LEN]).is_ok());
        for len in [0, FRAME_LEN - 1, FRAME_LEN + 1] {
            let error = check_length("power", &vec![0; len]).expect_err("the wrong length");
            assert_eq!(error.code(), "serialize");
        }
    }

    #[test]
    fn a_frame_too_short_to_name_itself_answers_nothing() {
        assert!(!answers(&[0xaa], &[0xaa, 0x04]));
        assert!(!answers(&[], &[]));
    }
}
