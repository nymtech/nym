// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! High-level tunnel providing TCP and UDP sockets over the Nym mixnet.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  User code                                                       │
//! │  tunnel.tcp_connect() → TcpStream (AsyncRead + AsyncWrite)       │
//! │  tunnel.udp_socket()  → UdpSocket (send_to / recv_from)          │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  tokio-smoltcp::Net                                              │
//! │  Owns the smoltcp Interface + SocketSet + async poll loop.       │
//! │  Manages TCP state machines, retransmits, port allocation.       │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  NymAsyncDevice  (this module's device.rs)                       │
//! │  Adapts mpsc channels into Stream + Sink of raw IP packets.      │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  NymIprBridge  (bridge.rs)                                       │
//! │  Shuttles packets between the channels and the mixnet.           │
//! │  Bundles outgoing packets with MultiIpPacketCodec for the IPR.   │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  IpMixStream → MixnetStream → Nym mixnet → IPR exit node         │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key insight is that tokio-smoltcp handles all the hard parts (smoltcp polling,
//! TCP state machines, port allocation, waker management) — we just need to give it
//! a device that produces and consumes raw IP packets. Our [`NymAsyncDevice`] does
//! exactly that by wrapping the mpsc channels that [`NymIprBridge`] already uses.
//!
//! The returned [`TcpStream`] implements `tokio::io::AsyncRead + AsyncWrite`, so it
//! works transparently with the entire async Rust ecosystem: tokio-rustls for TLS,
//! tokio-tungstenite for WebSockets, hyper for HTTP, etc. Code using these sockets
//! doesn't need to know it's going through the mixnet.

mod device;

use std::net::SocketAddr;
use std::sync::Arc;

use nym_ip_packet_requests::IpPair;
use nym_sdk::stream_wrapper::IpMixStream;
use smoltcp::iface::Config;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::info;

use crate::bridge::{BridgeShutdownHandle, NymIprBridge};
use crate::SmolmixError;
use device::NymAsyncDevice;
use tokio_smoltcp::{Net, NetConfig};

// Re-export so users only need `use smolmix::*` — no direct dep on nym-sdk or tokio-smoltcp.
pub use nym_sdk::stream_wrapper::NetworkEnvironment;
pub use tokio_smoltcp::{TcpStream, UdpSocket};

struct ShutdownState {
    bridge_shutdown: BridgeShutdownHandle,
    bridge_handle: JoinHandle<Result<(), SmolmixError>>,
}

struct TunnelInner {
    /// tokio-smoltcp network stack. Its methods take &self, so multiple tasks can
    /// open sockets concurrently without locking.
    net: Net,
    allocated_ips: IpPair,
    /// Mutex only protects shutdown — called once, not on the hot path.
    shutdown: Mutex<Option<ShutdownState>>,
}

/// A mixnet tunnel providing TCP and UDP socket access.
///
/// `Tunnel` manages a smoltcp network stack connected to the Nym mixnet via an IPR
/// (Internet Packet Router). It spawns a background bridge task and a network reactor,
/// then provides familiar socket APIs on top.
///
/// Cloning a `Tunnel` is cheap (Arc-based) and all clones share the same underlying
/// connection. Multiple tasks can open sockets concurrently.
#[derive(Clone)]
pub struct Tunnel {
    inner: Arc<TunnelInner>,
}

impl Tunnel {
    /// Create a new tunnel connected to the given network.
    ///
    /// This is the simplest entry point — one line gets you a working tunnel:
    /// ```ignore
    /// let tunnel = Tunnel::new(NetworkEnvironment::Mainnet).await?;
    /// let tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
    /// ```
    pub async fn new(env: NetworkEnvironment) -> Result<Self, SmolmixError> {
        let ipr_stream = IpMixStream::new(env).await?;
        Self::from_stream(ipr_stream).await
    }

    /// Create a tunnel from a pre-configured [`IpMixStream`].
    ///
    /// Use this if you need to customize the mixnet client (e.g. custom gateway,
    /// storage path, etc.) before creating the tunnel.
    pub async fn from_stream(mut ipr_stream: IpMixStream) -> Result<Self, SmolmixError> {
        if !ipr_stream.is_connected() {
            ipr_stream.connect_tunnel().await?;
        }

        let allocated_ips = *ipr_stream
            .allocated_ips()
            .ok_or(SmolmixError::NotConnected)?;

        // Wire up two channel pairs connecting the bridge (async mixnet I/O) to the
        // async device adapter (which tokio-smoltcp polls for raw IP packets):
        //
        //   outgoing: smoltcp → NymAsyncDevice.Sink → outgoing_tx → outgoing_rx → Bridge → mixnet
        //   incoming: mixnet → Bridge → incoming_tx → incoming_rx → NymAsyncDevice.Stream → smoltcp
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();

        // Bridge runs as a background task, shuttling packets between channels and IpMixStream.
        let (bridge, bridge_shutdown) = NymIprBridge::new(ipr_stream, outgoing_rx, incoming_tx);
        let bridge_handle = tokio::spawn(bridge.run());

        // NymAsyncDevice wraps the channel ends as Stream + Sink, which is all
        // tokio-smoltcp needs to drive the smoltcp Interface internally.
        let device = NymAsyncDevice::new(incoming_rx, outgoing_tx);

        // Configure smoltcp: raw IP mode (no Ethernet), /32 for our allocated IP,
        // default route via unspecified (the IPR handles actual routing).
        let iface_config = Config::new(HardwareAddress::Ip);
        let net_config = NetConfig::new(
            iface_config,
            IpCidr::new(IpAddress::from(allocated_ips.ipv4), 32),
            vec![IpAddress::from(Ipv4Address::UNSPECIFIED)],
        );

        // Net::new spawns the smoltcp reactor as a background task. From here on,
        // tcp_connect/udp_bind create sockets managed by that reactor.
        let net = Net::new(device, net_config);

        info!("Tunnel ready, allocated IP: {}", allocated_ips.ipv4);

        Ok(Self {
            inner: Arc::new(TunnelInner {
                net,
                allocated_ips,
                shutdown: Mutex::new(Some(ShutdownState {
                    bridge_shutdown,
                    bridge_handle,
                })),
            }),
        })
    }

    /// Open a TCP connection to `addr` through the mixnet.
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<TcpStream, SmolmixError> {
        Ok(self.inner.net.tcp_connect(addr).await?)
    }

    /// Create a UDP socket bound to an ephemeral port.
    pub async fn udp_socket(&self) -> Result<UdpSocket, SmolmixError> {
        let addr: SocketAddr = ([0, 0, 0, 0], 0).into();
        Ok(self.inner.net.udp_bind(addr).await?)
    }

    /// Create a UDP socket bound to a specific port.
    pub async fn udp_socket_on(&self, port: u16) -> Result<UdpSocket, SmolmixError> {
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        Ok(self.inner.net.udp_bind(addr).await?)
    }

    /// The IP addresses allocated to this tunnel by the IPR.
    pub fn allocated_ips(&self) -> IpPair {
        self.inner.allocated_ips
    }

    /// Gracefully shut down the tunnel.
    ///
    /// Signals the bridge to disconnect from the mixnet and waits for it to finish.
    /// The smoltcp reactor stops when all `Tunnel` clones are dropped.
    pub async fn shutdown(&self) {
        let mut state = self.inner.shutdown.lock().await;
        if let Some(s) = state.take() {
            info!("Shutting down tunnel");
            s.bridge_shutdown.shutdown();
            let _ = s.bridge_handle.await;
            info!("Tunnel shut down");
        }
    }
}
