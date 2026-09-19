//! The UDP socket every input protocol receives on.
//!
//! One socket, bound once. A broadcast frame and a unicast frame both land
//! here: a desk sends one or the other, and the node accepts both.

use std::net::{IpAddr, SocketAddr};

use socket2::{Domain, Protocol, Socket, Type};
use thiserror::Error;
use tokio::net::UdpSocket;

/// The largest datagram the node reads. An `ArtDmx` packet takes 530 bytes:
/// an 18-byte header and one universe.
pub const MAX_DATAGRAM: usize = 1024;

/// Why the node does not receive.
#[derive(Debug, Error)]
pub enum Error {
    /// A socket option or the socket itself could not be set up.
    #[error("cannot open the input socket: {step} failed: {reason}")]
    Open {
        /// What was being set up.
        step: &'static str,
        /// What the operating system reported.
        reason: String,
    },
    /// The address is taken, or the process may not have it.
    #[error("cannot bind {address}: {reason}")]
    Bind {
        /// The address the node asked for.
        address: SocketAddr,
        /// What the operating system reported.
        reason: String,
    },
    /// The socket failed while the node waited for a datagram.
    #[error("cannot receive on the input socket: {reason}")]
    Receive {
        /// What the operating system reported.
        reason: String,
    },
}

/// A bound UDP socket, and the datagrams that arrive on it.
#[derive(Debug)]
pub struct Listener {
    socket: UdpSocket,
}

impl Listener {
    /// Bind `address`.
    ///
    /// Call it inside a Tokio runtime: the socket registers with the reactor
    /// here.
    ///
    /// # Errors
    ///
    /// [`Error::Open`] where a socket option is refused, and [`Error::Bind`]
    /// where the address is taken or out of reach.
    pub fn bind(address: SocketAddr) -> Result<Self, Error> {
        let open = |step: &'static str| {
            move |e: std::io::Error| Error::Open {
                step,
                reason: e.to_string(),
            }
        };
        let socket = Socket::new(
            Domain::for_address(address),
            Type::DGRAM,
            Some(Protocol::UDP),
        )
        .map_err(open("creating the socket"))?;
        // Port 6454 is fixed by the protocol, so a second Art-Net application
        // on the host must be able to bind it too.
        socket
            .set_reuse_address(true)
            .map_err(open("SO_REUSEADDR"))?;
        #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
        socket.set_reuse_port(true).map_err(open("SO_REUSEPORT"))?;
        // What `ArtPollReply` goes out as.
        socket.set_broadcast(true).map_err(open("SO_BROADCAST"))?;
        socket.bind(&address.into()).map_err(|e| Error::Bind {
            address,
            reason: e.to_string(),
        })?;
        socket
            .set_nonblocking(true)
            .map_err(open("setting the socket non-blocking"))?;
        let socket = UdpSocket::from_std(socket.into()).map_err(open("the Tokio socket"))?;
        Ok(Self { socket })
    }

    /// Where the socket is bound. `None` where the operating system does not
    /// answer, which the caller reports and carries on from.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.local_addr().ok()
    }

    /// Wait for one datagram, and answer how many bytes it put in `buffer`
    /// and where it came from.
    ///
    /// Cancel-safe: a call dropped before it returns loses no datagram.
    ///
    /// # Errors
    ///
    /// [`Error::Receive`].
    pub async fn receive(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        self.socket
            .recv_from(buffer)
            .await
            .map_err(|e| Error::Receive {
                reason: e.to_string(),
            })
    }

    /// The address of the interface that reaches `peer`.
    ///
    /// It connects a socket of its own, which puts no datagram on the network:
    /// a connected UDP socket fixes a route and nothing else. `ArtPollReply`
    /// carries this address, and the node binds `0.0.0.0`, which carries
    /// nothing a desk can send to.
    ///
    /// `None` where the host has no route to `peer`.
    #[must_use]
    pub fn local_ip_towards(peer: SocketAddr) -> Option<IpAddr> {
        let unspecified = if peer.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = std::net::UdpSocket::bind(unspecified).ok()?;
        socket.connect(peer).ok()?;
        Some(socket.local_addr().ok()?.ip())
    }

    /// Send one datagram.
    ///
    /// # Errors
    ///
    /// [`Error::Receive`] carries the failure: one socket, one report.
    pub async fn send_to(&self, bytes: &[u8], address: SocketAddr) -> Result<(), Error> {
        self.socket
            .send_to(bytes, address)
            .await
            .map_err(|e| Error::Receive {
                reason: e.to_string(),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::{Listener, MAX_DATAGRAM};

    fn loopback() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }

    #[tokio::test]
    async fn a_datagram_arrives_with_the_address_it_came_from() {
        let listener = Listener::bind(loopback()).expect("the socket binds");
        let bound = listener.local_addr().expect("a bound address");
        let sender = Listener::bind(loopback()).expect("the socket binds");
        let from = sender.local_addr().expect("a bound address");

        sender.send_to(b"Art-Net\0", bound).await.expect("the send");
        let mut buffer = [0u8; MAX_DATAGRAM];
        let (read, source) = listener.receive(&mut buffer).await.expect("the datagram");

        assert_eq!(buffer.get(..read), Some(b"Art-Net\0".as_slice()));
        assert_eq!(source, from);
    }

    /// Port 6454 is fixed by the protocol, so two nodes on one host must both
    /// start.
    #[tokio::test]
    async fn two_listeners_take_one_port() {
        let first = Listener::bind(loopback()).expect("the socket binds");
        let bound = first.local_addr().expect("a bound address");
        Listener::bind(bound).expect("the second socket binds");
    }
}
