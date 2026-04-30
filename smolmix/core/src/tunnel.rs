// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! High-level tunnel providing TCP and UDP sockets over the Nym mixnet.
//!
//! See the [crate-level docs](crate) for the full architecture diagram.
//!
//! The returned [`TcpStream`] implements `tokio::io::AsyncRead + AsyncWrite`, so it
//! works transparently with the entire async Rust ecosystem: tokio-rustls for TLS,
//! tokio-tungstenite for WebSockets, hyper for HTTP, etc.

use std::net::SocketAddr;
use std::sync::Arc;

use futures::channel::mpsc;
pub use nym_ip_packet_requests::IpPair;
use nym_sdk::ipr_wrapper::IpMixStream;
use smoltcp::iface::Config;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use tokio::sync::Mutex;
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
/// # Shutdown
///
/// Call [`shutdown()`](Self::shutdown) for a clean disconnect. Rust has no async `Drop`,
/// so dropping without calling `shutdown()` triggers a fire-and-forget cleanup via the
/// oneshot channel — the bridge will still shut down, but the caller can't await it.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use smolmix::Tunnel;
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// let tunnel = Tunnel::new().await?;
///
/// // TCP — connect and use like any async stream
/// let mut tcp = tunnel.tcp_connect("1.1.1.1:80".parse()?).await?;
/// tcp.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n").await?;
/// let mut buf = Vec::new();
/// tcp.read_to_end(&mut buf).await?;
///
/// // UDP — datagrams over the mixnet
/// let udp = tunnel.udp_socket().await?;
/// udp.send_to(b"hello", "1.1.1.1:9999".parse()?).await?;
///
/// // Share across tasks (cheap Arc-based clone)
/// let t2 = tunnel.clone();
/// tokio::spawn(async move {
///     let _tcp2 = t2.tcp_connect("93.184.216.34:80".parse().unwrap()).await.unwrap();
/// });
///
/// tunnel.shutdown().await;
/// # Ok(())
/// # }
/// ```
///
/// See also the repository examples: `tcp`, `udp`, `websocket`.
#[derive(Clone)]
pub struct Tunnel {
    inner: Arc<TunnelInner>,
}

/// Builder for configuring and creating a [`Tunnel`].
///
/// Use [`Tunnel::builder()`] to create a new builder.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use smolmix::Tunnel;
///
/// // Auto-discover the best IPR:
/// let tunnel = Tunnel::builder().build().await?;
///
/// // Or specify an IPR exit node:
/// use smolmix::Recipient;
/// let ipr: Recipient = "gateway-address...".parse()?;
/// let tunnel = Tunnel::builder().ipr_address(ipr).build().await?;
/// # Ok(())
/// # }
/// ```
///
/// For full control over the mixnet client (credentials, gateway selection,
/// storage, etc.), configure an [`IpMixStream`] directly and pass it to
/// [`Tunnel::from_stream()`].
pub struct TunnelBuilder {
    ipr_address: Option<Recipient>,
}

impl TunnelBuilder {
    /// Target a specific IPR exit node instead of auto-discovering one.
    pub fn ipr_address(mut self, addr: Recipient) -> Self {
        self.ipr_address = Some(addr);
        self
    }

    /// Build and connect the tunnel.
    pub async fn build(self) -> Result<Tunnel, SmolmixError> {
        let stream = match self.ipr_address {
            Some(addr) => IpMixStream::new_with_ipr(addr).await?,
            None => IpMixStream::new().await?,
        };
        Tunnel::from_stream(stream).await
    }
}

impl Tunnel {
    /// Create a [`TunnelBuilder`] for configuring the tunnel before connecting.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use smolmix::Tunnel;
    /// let tunnel = Tunnel::builder().build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> TunnelBuilder {
        TunnelBuilder { ipr_address: None }
    }

    /// Create a new tunnel, automatically discovering the best IPR exit node.
    ///
    /// Shorthand for `Tunnel::builder().build().await`.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use smolmix::Tunnel;
    /// let tunnel = Tunnel::new().await?;
    /// let tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new() -> Result<Self, SmolmixError> {
        Self::builder().build().await
    }

    /// Create a new tunnel connected to a specific IPR exit node.
    ///
    /// Shorthand for `Tunnel::builder().ipr_address(addr).build().await`.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use smolmix::{Recipient, Tunnel};
    /// let ipr: Recipient = "gateway-address...".parse()?;
    /// let tunnel = Tunnel::new_with_ipr(ipr).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new_with_ipr(ipr_address: Recipient) -> Result<Self, SmolmixError> {
        Self::builder().ipr_address(ipr_address).build().await
    }

    /// Create a tunnel from a pre-configured [`IpMixStream`].
    ///
    /// Use this for full control over the mixnet client (credentials, gateway
    /// selection, storage, etc.) — configure the `IpMixStream` upstream and
    /// pass it in directly.
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
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded();
        let (incoming_tx, incoming_rx) = mpsc::unbounded();

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
    ///
    /// The returned [`TcpStream`] implements `tokio::io::AsyncRead + AsyncWrite`,
    /// so it works transparently with tokio-rustls, hyper, tokio-tungstenite, and
    /// any other async I/O consumer.
    ///
    /// # Errors
    ///
    /// Returns [`SmolmixError::Io`] if the TCP handshake fails (connection
    /// refused, timeout, etc.) or if the tunnel has been shut down.
    ///
    /// # Examples
    ///
    /// Raw HTTP request:
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let tunnel = smolmix::Tunnel::new().await?;
    /// use tokio::io::{AsyncReadExt, AsyncWriteExt};
    ///
    /// let mut tcp = tunnel.tcp_connect("1.1.1.1:80".parse()?).await?;
    /// tcp.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n").await?;
    ///
    /// let mut response = Vec::new();
    /// tcp.read_to_end(&mut response).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// TLS via tokio-rustls (the stream is a drop-in for `tokio::net::TcpStream`):
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let tunnel = smolmix::Tunnel::new().await?;
    /// use rustls::pki_types::ServerName;
    /// use tokio_rustls::TlsConnector;
    ///
    /// let tcp = tunnel.tcp_connect("93.184.216.34:443".parse()?).await?;
    /// # let connector: TlsConnector = todo!();
    /// let tls = connector.connect(ServerName::try_from("example.com")?.to_owned(), tcp).await?;
    /// // `tls` implements AsyncRead + AsyncWrite — use with hyper, tungstenite, etc.
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<TcpStream, SmolmixError> {
        Ok(self.inner.net.tcp_connect(addr).await?)
    }

    /// Create a UDP socket bound to an ephemeral port.
    ///
    /// The port is chosen by smoltcp's allocator. Use [`udp_socket_on`](Self::udp_socket_on)
    /// if you need a specific port (e.g. for a protocol that expects replies on
    /// a known port).
    ///
    /// The returned [`UdpSocket`] supports `send_to` / `recv_from` for datagram I/O.
    ///
    /// # Examples
    ///
    /// Send a DNS query and read the response:
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let tunnel = smolmix::Tunnel::new().await?;
    /// let udp = tunnel.udp_socket().await?;
    ///
    /// // Send a raw DNS query to Cloudflare
    /// let query = b"\x12\x34\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\
    ///               \x07example\x03com\x00\x00\x01\x00\x01";
    /// udp.send_to(query, "1.1.1.1:53".parse()?).await?;
    ///
    /// let mut buf = [0u8; 512];
    /// let (len, _src) = udp.recv_from(&mut buf).await?;
    /// println!("Got {} bytes back", len);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn udp_socket(&self) -> Result<UdpSocket, SmolmixError> {
        let addr: SocketAddr = ([0, 0, 0, 0], 0).into();
        Ok(self.inner.net.udp_bind(addr).await?)
    }

    /// Create a UDP socket bound to a specific local port.
    ///
    /// Binds to `0.0.0.0:<port>` on the tunnel's virtual interface. Use this when
    /// the remote side expects replies on a well-known port, or when you need
    /// multiple sockets on distinct ports.
    pub async fn udp_socket_on(&self, port: u16) -> Result<UdpSocket, SmolmixError> {
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        Ok(self.inner.net.udp_bind(addr).await?)
    }

    /// The IPv4/IPv6 address pair allocated to this tunnel by the IPR.
    ///
    /// Available immediately after construction. The IPv4 address is assigned as
    /// a /32 on the tunnel's virtual interface — all traffic to/from external
    /// hosts appears to originate from this IP at the exit gateway.
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
