//! libp2p ping through the Nym mixnet.
//!
//! Demonstrates implementing a libp2p `Transport` over smolmix. The transport
//! is dial-only (no listening through the mixnet) and supports both IP and DNS
//! multiaddrs. DNS is resolved through the tunnel to avoid hostname leaks.
//!
//! This example includes the full `SmolmixTransport` implementation inline —
//! it's ~80 lines of glue code, intended to be copy-pasted and adapted rather
//! than imported as a dependency.
//!
//! Run with:
//!   # Terminal 1: start a clearnet libp2p listener
//!   #   (you'll need a separate binary — see the listener pattern below)
//!
//!   # Terminal 2: dial through the mixnet
//!   cargo run -p smolmix --example libp2p_ping -- <MULTIADDR>
//!   cargo run -p smolmix --example libp2p_ping -- --ipr <IPR_ADDRESS> <MULTIADDR>

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::StreamExt;
use libp2p::core::multiaddr::Protocol;
use libp2p::core::transport::{DialOpts, ListenerId, TransportError, TransportEvent};
use libp2p::core::upgrade::Version;
use libp2p::core::Transport;
use libp2p::swarm::SwarmEvent;
use libp2p::{noise, ping, yamux, Multiaddr, SwarmBuilder};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use tracing::info;

use smolmix::{Recipient, Tunnel};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// -- SmolmixTransport: inline libp2p Transport impl --------------------------

/// A libp2p transport that routes TCP connections through a smolmix Tunnel.
///
/// Dial-only — listening is not supported (no inbound connections through the
/// mixnet). Supports `/ip4/.../tcp/N`, `/ip6/.../tcp/N`, `/dns4/.../tcp/N`,
/// and `/dns/.../tcp/N` multiaddrs.
struct SmolmixTransport {
    tunnel: Tunnel,
}

impl SmolmixTransport {
    fn new(tunnel: &Tunnel) -> Self {
        Self {
            tunnel: tunnel.clone(),
        }
    }
}

enum DialTarget {
    Ip(SocketAddr),
    Dns { host: String, port: u16 },
}

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
                    let addrs = smolmix_dns::resolve(&tunnel, &host, port).await?;
                    addrs.into_iter().next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::AddrNotAvailable, "DNS returned no addresses")
                    })?
                }
            };

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

// -- Main: ping a peer through the mixnet ------------------------------------

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    nym_bin_common::logging::setup_tracing_logger();

    let args: Vec<String> = std::env::args().collect();

    // Parse optional --ipr flag
    let ipr_pos = args.iter().position(|a| a == "--ipr");
    let ipr_addr = ipr_pos.and_then(|i| args.get(i + 1));

    // The multiaddr is the last positional argument (skip --ipr and its value)
    let multiaddr_str = args
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0 && Some(*i) != ipr_pos && Some(*i) != ipr_pos.map(|p| p + 1))
        .map(|(_, s)| s)
        .last()
        .expect(
            "Usage: libp2p_ping [--ipr <IPR_ADDRESS>] <MULTIADDR>\n\
             \n\
             The MULTIADDR should point to a libp2p node running the ping protocol.\n\
             Example: /ip4/1.2.3.4/tcp/4001/p2p/12D3Koo...",
        );
    let remote: Multiaddr = multiaddr_str.parse()?;

    let tunnel = if let Some(addr) = ipr_addr {
        let recipient: Recipient = addr.parse().expect("invalid IPR address");
        Tunnel::new_with_ipr(recipient).await?
    } else {
        Tunnel::new().await?
    };

    // Build the swarm with our inline mixnet transport
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_other_transport(|keypair| {
            SmolmixTransport::new(&tunnel)
                .upgrade(Version::V1)
                .authenticate(noise::Config::new(keypair).expect("noise config"))
                .multiplex(yamux::Config::default())
                .boxed()
        })?
        .with_behaviour(|_| ping::Behaviour::default())?
        .build();

    info!("Local peer ID: {}", swarm.local_peer_id());
    info!("Dialing {remote} through the Nym mixnet...");
    swarm.dial(remote)?;

    let mut pings = 0u32;
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(ping::Event { peer, result, .. }) => match result {
                Ok(rtt) => {
                    pings += 1;
                    info!("Ping #{pings} to {peer}: {rtt:?}");
                    if pings >= 5 {
                        info!("Done — {pings} pings completed.");
                        break;
                    }
                }
                Err(e) => {
                    info!("Ping error from {peer}: {e}");
                    break;
                }
            },
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to {peer_id}");
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                info!("Connection error: {error}");
                break;
            }
            _ => {}
        }
    }

    tunnel.shutdown().await;
    Ok(())
}
