//! The one UDP socket the transport reuses.
//!
//! Everything goes through it: the multicast `scan`, the commands to
//! `<device>:4003`, and the replies that come back. One socket rather than one
//! per send, because a fresh socket per command is a syscall and a port
//! allocation on the fast path for nothing — `docs/protocol/lan.md` §1, latency
//! notes.
//!
//! Replies to `scan` and to `devStatus` arrive on the same port, so the socket
//! is bound once and its receive loop dispatches on `msg.cmd`.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use socket2::{Domain, Protocol, Socket as Socket2, Type};
use tokio::net::UdpSocket;

use crate::lan::discovery::Endpoints;
use crate::transport::error::{Error, Result};

/// The largest datagram this protocol produces. A full-resolution segment frame
/// is a few hundred bytes base64'd into JSON; 4 KiB leaves room to spare, and a
/// larger datagram is not something this protocol sends.
pub(crate) const MAX_DATAGRAM: usize = 4096;

/// A bound, shared UDP socket.
#[derive(Debug, Clone)]
pub(crate) struct Socket {
    inner: Arc<UdpSocket>,
}

impl Socket {
    /// Bind the receive port and join the discovery group.
    ///
    /// `SO_REUSEADDR` — and `SO_REUSEPORT` where it exists — because port 4002
    /// is fixed by the protocol: without it, a second process on the host
    /// cannot start at all, and neither can two tests at once.
    pub(crate) fn bind(endpoints: &Endpoints) -> Result<Self> {
        let domain = Domain::for_address(endpoints.reply_bind);
        let socket = Socket2::new(domain, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| Error::io("creating the UDP socket", e))?;

        socket
            .set_reuse_address(true)
            .map_err(|e| Error::io("SO_REUSEADDR", e))?;
        #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
        socket
            .set_reuse_port(true)
            .map_err(|e| Error::io("SO_REUSEPORT", e))?;

        socket
            .bind(&endpoints.reply_bind.into())
            .map_err(|e| Error::io(format!("binding {}", endpoints.reply_bind), e))?;

        if let Some(group) = endpoints.multicast_group {
            // Replies are unicast, so this join is not what makes discovery
            // work; it is what lets this host see another client's scan, which
            // the simulator and any second SDK on the same machine rely on.
            if let Err(e) = socket.join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED) {
                tracing::warn!(%group, error = %e, "could not join the discovery group; replies still arrive");
            }
            socket
                .set_multicast_loop_v4(true)
                .map_err(|e| Error::io("IP_MULTICAST_LOOP", e))?;
        }

        socket
            .set_nonblocking(true)
            .map_err(|e| Error::io("setting the socket non-blocking", e))?;

        let inner = UdpSocket::from_std(socket.into()).map_err(|e| Error::io("tokio socket", e))?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Where this socket is bound. Its port is where replies come back.
    pub(crate) fn local_addr(&self) -> Result<SocketAddr> {
        self.inner
            .local_addr()
            .map_err(|e| Error::io("reading the local address", e))
    }

    /// Send one datagram.
    pub(crate) async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> Result<()> {
        self.inner
            .send_to(bytes, addr)
            .await
            .map_err(|e| Error::io(format!("sending to {addr}"), e))?;
        Ok(())
    }

    /// Wait for one datagram.
    pub(crate) async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.inner
            .recv_from(buf)
            .await
            .map_err(|e| Error::io("receiving", e))
    }
}

/// One parsed reply, before it is attributed to a device.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reply {
    /// Who sent it. For a `devStatus` answer this is the only correlation there
    /// is: the protocol carries no request id.
    pub(crate) from: SocketAddr,
    /// `msg.cmd`.
    pub(crate) cmd: String,
    /// `msg.data`.
    pub(crate) data: serde_json::Value,
}

/// Read a datagram into a [`Reply`].
///
/// Anything that is not the documented envelope is dropped. Devices on a
/// network answer discovery requests that were not this crate's, and other
/// software shares port 4002 — a datagram that does not parse is not an error.
pub(crate) fn parse_reply(from: SocketAddr, bytes: &[u8]) -> Option<Reply> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let msg = value.get_mut("msg")?.as_object_mut()?;
    Some(Reply {
        from,
        cmd: msg.get("cmd")?.as_str()?.to_owned(),
        // Moved out, not cloned: a status payload is copied enough on the way
        // to the watcher and the event stream as it is.
        data: msg.remove("data").unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn from() -> SocketAddr {
        SocketAddr::from(([192, 168, 1, 42], 4003))
    }

    #[test]
    fn reads_the_envelope() {
        let reply = parse_reply(from(), br#"{"msg":{"cmd":"devStatus","data":{"onOff":1}}}"#)
            .expect("a well-formed reply");
        assert_eq!(reply.cmd, "devStatus");
        assert_eq!(reply.data["onOff"], 1);
    }

    #[test]
    fn a_command_with_no_data_still_parses() {
        let reply = parse_reply(from(), br#"{"msg":{"cmd":"scan"}}"#).expect("cmd is enough");
        assert_eq!(reply.data, serde_json::Value::Null);
    }

    #[test]
    fn foreign_traffic_is_dropped_rather_than_an_error() {
        assert!(parse_reply(from(), b"not json at all").is_none());
        assert!(parse_reply(from(), br#"{"something":"else"}"#).is_none());
        assert!(parse_reply(from(), br#"{"msg":{"data":{}}}"#).is_none());
    }

    proptest::proptest! {
        /// Port 4002 is shared. Whatever else is on it, `parse_reply` either
        /// reads an envelope or drops the datagram — it never panics, and it
        /// never invents a `cmd`.
        #[test]
        fn arbitrary_bytes_are_read_or_dropped(bytes: Vec<u8>) {
            if let Some(reply) = parse_reply(from(), &bytes) {
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                proptest::prop_assert_eq!(&reply.cmd, value["msg"]["cmd"].as_str().unwrap());
            }
        }

        /// The same, over datagrams shaped like the protocol's own envelope,
        /// so the generator reaches past the first `get`.
        #[test]
        fn arbitrary_envelopes_are_read_or_dropped(
            value in crate::transport::arbitrary::json()
        ) {
            let bytes = serde_json::to_vec(&value).unwrap();
            let _ = parse_reply(from(), &bytes);
        }
    }

    #[tokio::test]
    async fn binds_an_ephemeral_port_without_multicast() {
        let endpoints = Endpoints {
            reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            multicast_group: None,
            ..Endpoints::default()
        };
        let socket = Socket::bind(&endpoints).expect("bind");
        assert_ne!(socket.local_addr().expect("local addr").port(), 0);
    }
}
