//! Answering the two sockets, and the socket setup itself.
//!
//! The awkward parts of the real protocol are deliberately reproduced: replies
//! carry no request id, and nothing is acknowledged.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use socket2::{Domain, Protocol, Socket as Socket2, Type};
use tokio::net::UdpSocket;

use crate::{Inner, Received};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Listen {
    Scan,
    Control,
}

impl Inner {
    pub(crate) async fn serve(self: Arc<Self>, which: Listen) {
        let socket = match which {
            Listen::Scan => &self.scan,
            Listen::Control => &self.control,
        };
        let mut buf = vec![0u8; 4096];
        loop {
            let Ok((read, from)) = socket.recv_from(&mut buf).await else {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            };
            let Some(bytes) = buf.get(..read) else {
                continue;
            };
            let Some((cmd, data)) = envelope(bytes) else {
                continue;
            };

            let request = Received {
                from,
                cmd,
                data: data.clone(),
            };
            let is_status = request.is_status_request();

            let (answer, faults) = {
                let Ok(mut state) = self.state.lock() else {
                    continue;
                };
                state.received.push(request.clone());

                // Discovery is always answered; on the control port only a
                // request for status draws a reply. Writes are silent.
                let answer = match which {
                    Listen::Scan => Some(self.scan_reply(&request.cmd)),
                    Listen::Control if is_status => Some(serde_json::json!({
                        "msg": { "cmd": request.cmd, "data": state.status }
                    })),
                    Listen::Control => None,
                };
                if answer.is_some() {
                    state.replies = state.replies.wrapping_add(1);
                    let drop = state.faults.silent
                        || state
                            .faults
                            .drop_one_in
                            .is_some_and(|n| n > 0 && state.replies % n == 0);
                    if drop {
                        continue;
                    }
                }
                (answer, state.faults)
            };

            let Some(answer) = answer else {
                continue;
            };
            if !faults.latency.is_zero() {
                tokio::time::sleep(faults.latency).await;
            }

            let to = SocketAddr::new(from.ip(), self.options.reply_port.unwrap_or(from.port()));
            let bytes = serde_json::to_vec(&answer).unwrap_or_default();
            if let Err(e) = socket.send_to(&bytes, to).await {
                tracing::warn!(%to, error = %e, "simulator could not answer");
            }
        }
    }

    fn scan_reply(&self, cmd: &str) -> serde_json::Value {
        let f = &self.options.firmware;
        serde_json::json!({
            "msg": {
                "cmd": cmd,
                "data": {
                    "ip": self.options.advertised_ip.to_string(),
                    "device": self.options.id,
                    "sku": self.options.sku,
                    "bleVersionHard": f.ble_hardware,
                    "bleVersionSoft": f.ble_software,
                    "wifiVersionHard": f.wifi_hardware,
                    "wifiVersionSoft": f.wifi_software,
                }
            }
        })
    }
}

fn envelope(bytes: &[u8]) -> Option<(String, serde_json::Value)> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let msg = value.get("msg")?;
    Some((
        msg.get("cmd")?.as_str()?.to_owned(),
        msg.get("data").cloned().unwrap_or(serde_json::Value::Null),
    ))
}

pub(crate) fn bind(addr: SocketAddr, group: Option<Ipv4Addr>) -> std::io::Result<UdpSocket> {
    let socket = Socket2::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
    socket.set_reuse_port(true)?;
    socket.bind(&addr.into())?;
    if let Some(group) = group {
        socket.join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED)?;
        socket.set_multicast_loop_v4(true)?;
    }
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}
