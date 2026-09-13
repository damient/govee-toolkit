//! The exchange that opens an encoded link: a session seed, then its
//! confirmation. `docs/protocol/ble.md` 9.
//!
//! Both frames go out under [`Codec::base`], and the device answers each on
//! the notify characteristic, encoded the same way. What follows on the link
//! runs under the seed the device handed back.

use std::time::Duration;

use tokio::sync::broadcast;

use crate::ble::WRITE_CHARACTERISTIC;
use crate::ble::encode::{self, Codec, Noise};
use crate::ble::wire::Peripheral;

/// Run the handshake and answer the session codec.
///
/// `replies` must already be subscribed: the device answers within
/// milliseconds, and a reply that arrives before the subscription is lost.
///
/// # Errors
///
/// [`std::io::ErrorKind::TimedOut`] if the device does not answer a step
/// within `timeout`, [`std::io::ErrorKind::BrokenPipe`] if the reply stream
/// closes, and whatever the platform reports for a write.
pub(crate) async fn establish(
    peripheral: &dyn Peripheral,
    replies: &mut broadcast::Receiver<Vec<u8>>,
    timeout: Duration,
) -> std::io::Result<Codec> {
    let base = Codec::base();
    let mut noise = Noise::new();

    let request = encode::request(&mut || noise.next_byte());
    peripheral
        .write(WRITE_CHARACTERISTIC, &base.encode(&request))
        .await?;
    let seed = await_reply(replies, timeout, "the session seed", |plain| {
        encode::session_seed(plain)
    })
    .await?;
    let session = Codec::new(seed);

    let confirm = encode::confirm(&mut || noise.next_byte());
    peripheral
        .write(WRITE_CHARACTERISTIC, &base.encode(&confirm))
        .await?;
    await_reply(replies, timeout, "the confirmation", |plain| {
        encode::is_confirm(plain).then_some(())
    })
    .await?;

    Ok(session)
}

/// The first reply that `read` accepts once decoded under the base seed,
/// within `timeout`. Every other reply is skipped: the device may notify
/// something else in between.
async fn await_reply<T>(
    replies: &mut broadcast::Receiver<Vec<u8>>,
    timeout: Duration,
    what: &str,
    read: impl Fn(&[u8]) -> Option<T>,
) -> std::io::Result<T> {
    let base = Codec::base();
    let waited = tokio::time::timeout(timeout, async {
        loop {
            match replies.recv().await {
                Ok(coded) => {
                    if let Some(value) = read(&base.decode(&coded)) {
                        return Ok(value);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("the link closed before {what} arrived"),
                    ));
                }
            }
        }
    })
    .await;
    match waited {
        Ok(result) => result,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!(
                "the device did not answer {what} within {} ms",
                timeout.as_millis()
            ),
        )),
    }
}
