// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The `WgPacketTransport` seam: one WireGuard packet per send/recv.
//!
//! Split into independent sender/receiver halves so the datapath task can await
//! a receive while still sending on another `select!` branch. Two data-plane
//! implementations exist: [`WgSender::Direct`]/[`WgReceiver::Direct`] (a real
//! UDP socket to the entry gateway) and the QUIC bridge (see [`crate::bridge`]),
//! which only fronts the two-hop entry leg.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;

use crate::bridge::{QuicBridgeReceiver, QuicBridgeSender};
use crate::error::{DvpnError, Result};

/// Size of the reusable Direct-UDP receive buffer: the maximum a UDP datagram can be (64 KiB). The
/// `Direct` receiver allocates it once and reuses it across recvs, so — unlike the previous per-recv
/// `vec![0u8; 65535]` — there is no per-packet allocation. Sized to the maximum so a datagram is
/// never truncated regardless of the configured MTU.
const MAX_WG_PACKET: usize = 65535;

/// A hook invoked with a freshly-created socket's file descriptor so the host
/// can protect it from the tunnel's own routes (Linux/Android). No-op on other
/// platforms.
#[derive(Clone)]
pub struct SocketProtector(Arc<dyn Fn(i32) + Send + Sync>);

impl SocketProtector {
    /// Wrap a protection callback.
    pub fn new(f: impl Fn(i32) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    #[cfg_attr(not(unix), allow(dead_code))]
    fn protect(&self, fd: i32) {
        (self.0)(fd)
    }
}

impl std::fmt::Debug for SocketProtector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SocketProtector(..)")
    }
}

/// Sending half of the active WireGuard packet transport. One instance per
/// tunnel, so the inter-variant size difference does not matter.
#[allow(clippy::large_enum_variant)]
pub(crate) enum WgSender {
    Direct(Arc<UdpSocket>),
    Quic(QuicBridgeSender),
}

/// Receiving half of the active WireGuard packet transport.
#[allow(clippy::large_enum_variant)]
pub(crate) enum WgReceiver {
    Direct {
        sock: Arc<UdpSocket>,
        /// Reused across recvs so a fresh buffer isn't allocated per packet.
        buf: Box<[u8]>,
    },
    Quic(QuicBridgeReceiver),
}

impl WgSender {
    /// Send exactly one WireGuard packet.
    pub(crate) async fn send(&mut self, packet: &[u8]) -> Result<()> {
        match self {
            WgSender::Direct(sock) => {
                sock.send(packet).await?;
                Ok(())
            }
            WgSender::Quic(sender) => sender.send(packet).await,
        }
    }
}

impl WgReceiver {
    /// Whether this is the QUIC bridge transport (whose recv errors are fatal — a closed stream
    /// cannot recover), as opposed to Direct UDP (whose recv errors are transient).
    pub(crate) fn is_bridge(&self) -> bool {
        matches!(self, WgReceiver::Quic(_))
    }

    /// Receive exactly one WireGuard packet.
    pub(crate) async fn recv(&mut self) -> Result<Vec<u8>> {
        match self {
            WgReceiver::Direct { sock, buf } => {
                let n = sock.recv(buf).await?;
                Ok(buf[..n].to_vec())
            }
            WgReceiver::Quic(receiver) => receiver.recv().await,
        }
    }
}

/// Build a `Direct` UDP transport bound locally and connected to the entry
/// gateway endpoint. Invokes the socket protector (if any) before any traffic.
pub(crate) async fn direct_transport(
    entry_endpoint: SocketAddr,
    protector: Option<&SocketProtector>,
) -> Result<(WgSender, WgReceiver)> {
    let bind: SocketAddr = if entry_endpoint.is_ipv4() {
        "0.0.0.0:0".parse().expect("valid bind addr")
    } else {
        "[::]:0".parse().expect("valid bind addr")
    };
    let sock = UdpSocket::bind(bind).await?;

    #[cfg(unix)]
    if let Some(p) = protector {
        use std::os::fd::AsRawFd;
        p.protect(sock.as_raw_fd());
    }
    #[cfg(not(unix))]
    let _ = protector;

    sock.connect(entry_endpoint)
        .await
        .map_err(|e| DvpnError::Transport(format!("connect to entry gateway failed: {e}")))?;

    let sock = Arc::new(sock);
    Ok((
        WgSender::Direct(sock.clone()),
        WgReceiver::Direct {
            sock,
            buf: vec![0u8; MAX_WG_PACKET].into_boxed_slice(),
        },
    ))
}
