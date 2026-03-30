// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! libp2p [`Transport`] that routes connections through the Nym mixnet.
//!
//! [`SmolmixTransport`] implements the libp2p [`Transport`] trait by parsing
//! multiaddrs, optionally resolving DNS through the tunnel, and opening TCP
//! connections via smolmix. Because smolmix provides real TCP streams (via a
//! user-space smoltcp stack), libp2p's standard noise encryption and yamux
//! multiplexing work out of the box.
//!
//! # Supported multiaddrs
//!
//! - `/ip4/<addr>/tcp/<port>`
//! - `/ip6/<addr>/tcp/<port>`
//! - `/dns4/<host>/tcp/<port>` (resolved through the tunnel)
//! - `/dns/<host>/tcp/<port>` (resolved through the tunnel)
//!
//! # Quick start
//!
//! ```ignore
//! use libp2p::{noise, yamux, SwarmBuilder};
//! use libp2p::core::{upgrade::Version, Transport};
//! use smolmix_libp2p::SmolmixTransport;
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//!
//! let swarm = SwarmBuilder::with_new_identity()
//!     .with_tokio()
//!     .with_other_transport(|keypair| {
//!         SmolmixTransport::new(&tunnel)
//!             .upgrade(Version::V1)
//!             .authenticate(noise::Config::new(keypair).expect("noise config"))
//!             .multiplex(yamux::Config::default())
//!             .boxed()
//!     })?
//!     .with_behaviour(|_| libp2p::ping::Behaviour::default())?
//!     .build();
//! ```
//!
//! # Limitations
//!
//! - **Dial-only**: listening is not supported (no inbound connections through
//!   the mixnet). `listen_on()` returns [`TransportError::MultiaddrNotSupported`].
//! - **No TLS in transport**: libp2p uses noise for encryption, so adding TLS
//!   would be redundant.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use libp2p::core::multiaddr::Protocol;
use libp2p::core::transport::{DialOpts, ListenerId, TransportError, TransportEvent};
use libp2p::core::Transport;
use libp2p::Multiaddr;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use tracing::debug;

use smolmix::Tunnel;

/// A libp2p transport that routes TCP connections through a smolmix [`Tunnel`].
///
/// Supports dialing peers by IP or hostname (DNS resolved through the tunnel).
/// Does not support listening — the mixnet doesn't provide inbound TCP yet.
pub struct SmolmixTransport {
    tunnel: Tunnel,
}

impl SmolmixTransport {
    /// Create a new transport backed by the given tunnel.
    pub fn new(tunnel: &Tunnel) -> Self {
        Self {
            tunnel: tunnel.clone(),
        }
    }
}

/// Parsed dial target from a multiaddr.
enum DialTarget {
    Ip(SocketAddr),
    Dns { host: String, port: u16 },
}

/// Extract a dial target from a multiaddr.
///
/// Accepts `/ip4/.../tcp/N`, `/ip6/.../tcp/N`, `/dns4/.../tcp/N`, and
/// `/dns/.../tcp/N`. Trailing components (like `/p2p/...`) are ignored.
fn parse_multiaddr(addr: &Multiaddr) -> Option<DialTarget> {
    let mut iter = addr.iter();
    match iter.next()? {
        Protocol::Ip4(ip) => {
            if let Some(Protocol::Tcp(port)) = iter.next() {
                Some(DialTarget::Ip(SocketAddr::new(ip.into(), port)))
            } else {
                None
            }
        }
        Protocol::Ip6(ip) => {
            if let Some(Protocol::Tcp(port)) = iter.next() {
                Some(DialTarget::Ip(SocketAddr::new(ip.into(), port)))
            } else {
                None
            }
        }
        Protocol::Dns4(host) | Protocol::Dns(host) => {
            if let Some(Protocol::Tcp(port)) = iter.next() {
                Some(DialTarget::Dns {
                    host: host.to_string(),
                    port,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

impl Transport for SmolmixTransport {
    type Output = Compat<tokio_smoltcp::TcpStream>;
    type Error = io::Error;
    type ListenerUpgrade = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;
    type Dial = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn listen_on(
        &mut self,
        _id: ListenerId,
        addr: Multiaddr,
    ) -> Result<(), TransportError<Self::Error>> {
        Err(TransportError::MultiaddrNotSupported(addr))
    }

    fn remove_listener(&mut self, _id: ListenerId) -> bool {
        false
    }

    fn dial(
        &mut self,
        addr: Multiaddr,
        _opts: DialOpts,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        let target = parse_multiaddr(&addr).ok_or(TransportError::MultiaddrNotSupported(addr))?;
        let tunnel = self.tunnel.clone();

        Ok(Box::pin(async move {
            let socket_addr = match target {
                DialTarget::Ip(addr) => addr,
                DialTarget::Dns { host, port } => {
                    debug!(host = %host, port, "resolving DNS through tunnel");
                    let addrs = smolmix_dns::resolve(&tunnel, &host, port).await?;
                    addrs.into_iter().next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::AddrNotAvailable, "DNS returned no addresses")
                    })?
                }
            };

            debug!(addr = %socket_addr, "connecting TCP through tunnel");
            let tcp = tunnel
                .tcp_connect(socket_addr)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(tcp.compat())
        }))
    }

    fn poll(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        Poll::Pending
    }
}
