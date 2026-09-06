//! Asking a device something, and reading the answer.
//!
//! A read is a run of exchanges: write one frame, wait for the notification its
//! `reply:` layout matches, write the next. The wire carries no request id, so
//! the layout is the correlation — a notification that does not match the
//! layout is not this exchange's answer, and it is skipped rather than read.
//!
//! Every caller sends its own frames, because two callers sharing one request
//! would be told apart by nothing. The write budget is what keeps that from
//! becoming a burst.

use std::time::{Duration, Instant};

use tokio::sync::broadcast::Receiver;

use super::shared::{Route, Shared};
use crate::ble::link::Link;
use crate::codec::reply::Layout;
use crate::codec::{Captured, Encoded, Mode};
use crate::transport::error::{Error, Result};
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, Reply};

impl Shared {
    /// Run every exchange the command declares and merge what they captured.
    ///
    /// # Errors
    ///
    /// [`Error::NoReplyLayout`] if the command declares no reply to read,
    /// [`Error::UnknownDevice`] or [`Error::Unavailable`] as for a write,
    /// [`Error::Io`] if the connection or a write fails, and
    /// [`Error::Unreachable`] if an exchange goes unanswered.
    pub(super) async fn read(
        &self,
        id: &DeviceId,
        request: &Encoded,
        timeout: Duration,
    ) -> Result<Reply> {
        let exchanges = request.reads();
        if exchanges.is_empty() {
            return Err(Error::NoReplyLayout {
                mode: Mode::Ble,
                reason: format!(
                    "`{}` declares no `reply:` layout, so there is nothing to read back",
                    request.cmd
                ),
            });
        }

        let route = self.route_and_claim(id, Instant::now(), false)?;
        let link = match self.link(id, &route.endpoint).await {
            Ok(link) => link,
            Err(e) => {
                // A device that will not take a connection has answered
                // nothing, which is what the breaker is counting.
                self.record(id, false, Instant::now());
                return Err(e);
            }
        };

        let mut captured = Captured::new();
        for (frame, layout) in exchanges {
            // Subscribe before writing, or a reply that arrives first is
            // missed.
            let replies = link.replies();
            self.write_frame(id, &route, &link, &request.cmd, frame)
                .await?;
            let Some(fields) = await_reply(replies, layout, &request.cmd, timeout).await else {
                self.record(id, false, Instant::now());
                return Err(Error::Unreachable {
                    id: id.clone(),
                    endpoint: route.endpoint,
                    timeout_ms: crate::transport::millis(timeout),
                });
            };
            captured.merge(fields);
        }

        self.record(id, true, Instant::now());
        Ok(Reply {
            id: id.clone(),
            fields: captured,
        })
    }

    /// Ask a device for its state and wait for the answer.
    ///
    /// The command's `reply:` layouts say which bytes carry what, and its
    /// argument roles say which of those the SDK models. No field name reaches
    /// this code.
    ///
    /// # Errors
    ///
    /// As for [`Shared::read`].
    pub(super) async fn request_status(
        &self,
        id: &DeviceId,
        request: &Encoded,
        timeout: Duration,
    ) -> Result<DeviceStatus> {
        let reply = self.read(id, request, timeout).await?;
        let status = DeviceStatus::from_captured(id.clone(), &reply.fields, &request.roles);
        self.publish_status(id, status.clone());
        Ok(status)
    }

    /// Write one frame at the device's budget, dropping the link if it fails.
    async fn write_frame(
        &self,
        id: &DeviceId,
        route: &Route,
        link: &Link,
        cmd: &str,
        frame: &[u8],
    ) -> Result<()> {
        route.pacer.acquire().await;
        if let Err(e) = link.write_frame(cmd, &route.endpoint, frame).await {
            self.drop_link(id).await;
            self.record(id, false, Instant::now());
            return Err(e);
        }
        Ok(())
    }
}

/// Wait for a notification the layout reads, or for the deadline.
async fn await_reply(
    mut replies: Receiver<Vec<u8>>,
    layout: &Layout,
    cmd: &str,
    timeout: Duration,
) -> Option<Captured> {
    tokio::time::timeout(timeout, async {
        loop {
            match replies.recv().await {
                Ok(bytes) => {
                    if let Ok(fields) = layout.read(cmd, &bytes) {
                        return Some(fields);
                    }
                }
                // Lagged behind the device, or the link is gone: either way
                // this exchange has nothing left to wait for.
                Err(_) => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
}
