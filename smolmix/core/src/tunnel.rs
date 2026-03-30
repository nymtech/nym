// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! High-level tunnel providing TCP and UDP sockets over the Nym mixnet.
//!
//! See the [crate-level docs](crate) for the full architecture diagram.
//!
//! The returned [`TcpStream`] implements `tokio::io::AsyncRead + AsyncWrite`, so it
//! works transparently with the entire async Rust ecosystem: tokio-rustls for TLS,
//! tokio-tungstenite for WebSockets, hyper for HTTP, etc.

use std::net::SocketAddr;
use std::sync::Arc;

pub use nym_ip_packet_requests::IpPair;
use nym_sdk::ipr_wrapper::IpMixStream;
use smoltcp::iface::Config;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::info;

use crate::bridge::{BridgeShutdownHandle, NymIprBridge};
use crate::device::NymAsyncDevice;
use crate::SmolmixError;
use tokio_smoltcp::{Net, NetConfig};

pub use nym_sdk::mixnet::Recipient;
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
///
/// # Shutdown
///
/// Call [`shutdown()`](Self::shutdown) for a clean disconnect. Rust has no async `Drop`,
/// so dropping without calling `shutdown()` triggers a fire-and-forget cleanup via the
/// oneshot channel — the bridge will still shut down, but the caller can't await it.
#[derive(Clone)]
pub struct Tunnel {
    inner: Arc<TunnelInner>,
}

impl Tunnel {
    /// Create a new tunnel, automatically discovering the best IPR exit node.
    ///
    /// This is the simplest entry point — one line gets you a working tunnel:
    /// ```ignore
    /// let tunnel = Tunnel::new().await?;
    /// let tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
    /// ```
    pub async fn new() -> Result<Self, SmolmixError> {
        let ipr_stream = IpMixStream::new().await?;
        Self::from_stream(ipr_stream).await
    }

    /// Create a new tunnel connected to a specific IPR exit node.
    ///
    /// Use this for testing against a known exit gateway, or when you want to
    /// bypass automatic IPR discovery:
    /// ```ignore
    /// let ipr: Recipient = "gateway-address...".parse()?;
    /// let tunnel = Tunnel::new_with_ipr(ipr).await?;
    /// ```
    pub async fn new_with_ipr(ipr_address: Recipient) -> Result<Self, SmolmixError> {
        let ipr_stream = IpMixStream::new_with_ipr(ipr_address).await?;
        Self::from_stream(ipr_stream).await
    }

    /// Create a tunnel from a pre-configured [`IpMixStream`].
    ///
    /// Use this if you need to customize the mixnet client (e.g. custom gateway,
    /// storage path, etc.) before creating the tunnel.
    pub async fn from_stream(ipr_stream: IpMixStream) -> Result<Self, SmolmixError> {
        ipr_stream
            .check_connected()
            .map_err(|_| SmolmixError::NotConnected)?;

        let allocated_ips = *ipr_stream.allocated_ips();

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
    ///
    /// If the `Tunnel` is dropped without calling `shutdown()`, cleanup still happens:
    /// dropping the `Arc<TunnelInner>` drops the oneshot sender inside `ShutdownState`,
    /// which resolves the bridge's `shutdown_rx` and triggers its shutdown path. However,
    /// the drop path is fire-and-forget — call `shutdown()` explicitly if you need to
    /// wait for the mixnet disconnect to complete.
    ///
    /// After shutdown, new socket operations (`tcp_connect`, `udp_socket`) will fail
    /// with IO errors — the bridge channels are closed.
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
