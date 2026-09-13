//! One connection to one device: the two characteristics, and the replies.
//!
//! A device takes one connection and stops advertising while it is up, so the
//! link is opened once and kept. Replies carry no request id, so a caller
//! subscribes before it writes and matches the answer against its command's
//! `reply:` layout.
//!
//! A device that advertises the encoding flag gets the handshake of
//! [`session`] as soon as the link is up. From then on every frame written is
//! encoded and every reply is decoded before a caller sees it, so the layouts
//! above this module read plaintext either way.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures_util::StreamExt as _;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::ble::encode::Codec;
use crate::ble::wire::Peripheral;
use crate::ble::{
    FRAME_LEN, HOST_COLOR_PROTYPE, NOTIFY_CHARACTERISTIC, WRITE_CHARACTERISTIC, session,
};
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
    /// The session codec, once the handshake has run. Empty on a plaintext
    /// link. Shared with the listener, which decodes replies under it.
    session: Arc<OnceLock<Codec>>,
    _listener: Listener,
}

impl Link {
    /// Connect, discover the service, subscribe to notifications, and run the
    /// handshake if `encoded`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the device refuses the connection, if service
    /// discovery fails, if it does not carry the characteristics this
    /// protocol needs, or if it does not answer a step of the handshake
    /// within `handshake_timeout`.
    pub(crate) async fn open(
        peripheral: Arc<dyn Peripheral>,
        endpoint: &str,
        encoded: bool,
        handshake_timeout: Duration,
    ) -> Result<Self> {
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
        let session: Arc<OnceLock<Codec>> = Arc::new(OnceLock::new());
        let decoder = Arc::clone(&session);
        let listener = Listener(tokio::spawn(async move {
            while let Some(frame) = stream.next().await {
                let frame = match decoder.get() {
                    Some(codec) => codec.decode(&frame),
                    None => frame,
                };
                let _ = publish.send(frame);
            }
        }));

        if encoded {
            // Subscribed before the request goes out: the device answers
            // within milliseconds.
            let mut raw = replies.subscribe();
            let codec = session::establish(peripheral.as_ref(), &mut raw, handshake_timeout)
                .await
                .map_err(|e| adapter(endpoint, "opening the encoded link", e))?;
            let _ = session.set(codec);
        }

        Ok(Self {
            peripheral,
            replies,
            session,
            _listener: listener,
        })
    }

    /// Subscribe before you write the request, or a reply that arrives first
    /// is missed.
    pub(crate) fn replies(&self) -> broadcast::Receiver<Vec<u8>> {
        self.replies.subscribe()
    }

    /// Write one frame, without waiting for a response. On an encoded link the
    /// frame goes out encoded under the session seed.
    ///
    /// # Errors
    ///
    /// [`Error::Serialize`] if the frame is not the one length this wire
    /// carries, or [`Error::Io`] if the write fails.
    pub(crate) async fn write_frame(&self, cmd: &str, endpoint: &str, frame: &[u8]) -> Result<()> {
        check_length(cmd, frame)?;
        let out;
        let wire = match self.session.get() {
            Some(codec) => {
                out = codec.encode(frame);
                out.as_slice()
            }
            None => frame,
        };
        self.peripheral
            .write(WRITE_CHARACTERISTIC, wire)
            .await
            .map_err(|e| adapter(endpoint, "writing a frame", e))
    }
}

/// Refuse a frame this wire cannot carry.
///
/// The wire carries two shapes: [`FRAME_LEN`] bytes, and the shorter host
/// colour frame under [`HOST_COLOR_PROTYPE`].
///
/// # Errors
///
/// [`Error::Serialize`] for any other length. A short frame padded here would
/// reach the device as a command nobody wrote.
pub(crate) fn check_length(cmd: &str, frame: &[u8]) -> Result<()> {
    if frame.len() == FRAME_LEN {
        return Ok(());
    }
    if frame.first() == Some(&HOST_COLOR_PROTYPE) && !frame.is_empty() && frame.len() < FRAME_LEN {
        return Ok(());
    }
    Err(Error::Serialize {
        cmd: cmd.to_owned(),
        reason: format!(
            "the frame is {} bytes; this wire carries {FRAME_LEN}, or fewer under proType {HOST_COLOR_PROTYPE:#04x}",
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

    #[test]
    fn a_short_host_color_frame_is_written_as_it_is() {
        assert!(check_length("color", &[HOST_COLOR_PROTYPE, 0x02, 0x83, 0, 0, 0, 0x2a]).is_ok());
        let long = vec![HOST_COLOR_PROTYPE; FRAME_LEN + 1];
        let error = check_length("color", &long).expect_err("past the one length");
        assert_eq!(error.code(), "serialize");
    }
}
