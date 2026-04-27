// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! WASM mixnet tunnel — the browser-side equivalent of `smolmix::Tunnel`.
//!
//! Manages a smoltcp stack connected to the Nym mixnet via an IPR (IP Packet
//! Router), running entirely in the browser's single-threaded WASM environment.
//!
//! # Architecture differences from native smolmix
//!
//! | Concern | Native | WASM |
//! |---------|--------|------|
//! | smoltcp driver | tokio-smoltcp (tokio reactor) | direct `Interface::poll` via `spawn_local` |
//! | LP framing | `MixnetStream` (tokio AsyncWrite) | manual `lp::encode` / `lp::decode` |
//! | I/O traits | `tokio::io::{AsyncRead, AsyncWrite}` | `futures::io::{AsyncRead, AsyncWrite}` |
//! | Task spawning | `tokio::spawn` | `wasm_bindgen_futures::spawn_local` |
//! | Shared state | `Arc<Mutex<>>` | `Arc<Mutex<>>` (no-op lock on wasm32) |
//!
//! # Data flow
//!
//! ```text
//! WasmTcpStream::poll_write(data)
//!   → smoltcp tcp::Socket::send_slice(data)
//!   → reactor: Interface::poll() → IP packets in device tx queue
//!   → bridge: drain tx → bundle → LP frame → mixnet → IPR → internet
//!
//! internet → IPR → mixnet → ReconstructedMessage
//!   → bridge: LP decode → parse IPR response → unbundle
//!   → device rx queue → reactor: Interface::poll() → tcp::Socket rx buffer
//!   → WasmTcpStream::poll_read() → data to caller
//! ```

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::channel::mpsc;
use futures::io::{AsyncRead, AsyncWrite};
use smoltcp::iface::{Config, SocketHandle, SocketSet};
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::socket::udp as smoltcp_udp;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address};

use nym_ip_packet_requests::IpPair;
use nym_wasm_client_core::client::base_client::{BaseClientBuilder, ClientInput};
use nym_wasm_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReceivedBufferRequestSender,
};
use nym_wasm_client_core::config::new_base_client_config;
use nym_wasm_client_core::helpers::{add_gateway, generate_new_client_keys};
use nym_wasm_client_core::nym_task::ShutdownTracker;
use nym_wasm_client_core::storage::core_client_traits::FullWasmClientStorage;
use nym_wasm_client_core::storage::wasm_client_traits::WasmClientStorage;
use nym_wasm_client_core::storage::ClientStorage;
use nym_wasm_client_core::{QueryReqwestRpcNyxdClient, Recipient};

use crate::bridge;
use crate::device::WasmDevice;
use crate::error::FetchError;
use crate::ipr;
use crate::reactor::{self, smoltcp_now, ReactorNotify, SmoltcpStack, SocketKind, SocketWakers};

/// Starting ephemeral port for TCP/UDP sockets.
const EPHEMERAL_PORT_START: u16 = 49152;

/// Configuration parsed from the JS `setupMixTunnel(opts)` object.
pub struct TunnelOpts {
    pub ipr_address: Recipient,
    /// Client storage ID. Randomise per session to get a clean client.
    pub client_id: String,
    /// Use `wss://` for gateway connections (default: `true`).
    pub force_tls: bool,
    /// Disable Poisson-distributed dummy traffic (default: `false`).
    pub disable_poisson_traffic: bool,
    /// Disable cover traffic loop (default: `false`).
    pub disable_cover_traffic: bool,
}

/// WASM mixnet tunnel — the browser-side equivalent of `smolmix::Tunnel`.
///
/// Manages a smoltcp stack connected to the Nym mixnet via an IPR,
/// running entirely in the browser's single-threaded WASM environment.
pub struct WasmTunnel {
    stack: Arc<Mutex<SmoltcpStack>>,
    client_input: Arc<ClientInput>,
    ipr_address: Recipient,
    stream_id: u64,
    /// LP frame sequence counter (shared with bridge for outgoing data).
    seq: Arc<AtomicU32>,
    notify: ReactorNotify,
    shutdown: Arc<AtomicBool>,
    /// Ephemeral port counter for new sockets.
    next_port: AtomicU16,
    allocated_ips: IpPair,
    /// DNS resolution cache — avoids re-querying the same hostname.
    dns_cache: Mutex<HashMap<String, IpAddr>>,
    /// Serialises DNS lookups so concurrent requests for the same hostname
    /// coalesce: the first caller does the actual query and populates the
    /// cache; subsequent callers acquire the lock, hit the cache, and return
    /// immediately without creating duplicate UDP sockets.
    dns_lock: futures::lock::Mutex<()>,
    /// Connection pool keyed by (host, port). Holds at most one idle
    /// connection per origin — sufficient for sequential `mixFetch` calls.
    conn_pool: Mutex<HashMap<(String, u16), PooledConn>>,
    /// Per-origin locks: serialises requests to the same (host, port) so
    /// concurrent fetches queue behind one connection instead of each opening
    /// a separate TCP+TLS connection (which triggers server rate-limiting).
    origin_locks: Mutex<HashMap<(String, u16), Arc<futures::lock::Mutex<()>>>>,
    // Keep-alive fields: dropping these would shut down the base client
    // or disconnect the received buffer pipeline.
    _request_sender: ReceivedBufferRequestSender,
    _shutdown_handle: ShutdownTracker,
}

/// A connection that can be pooled — either TLS-wrapped or plain TCP.
///
/// Implements `futures::io::{AsyncRead, AsyncWrite}` by delegating to the
/// inner stream, so `http::request()` works identically regardless of variant.
pub(crate) enum PooledConn {
    Tls(futures_rustls::client::TlsStream<WasmTcpStream>),
    Plain(WasmTcpStream),
}

/// TCP stream over the WASM tunnel.
///
/// Implements `futures::io::{AsyncRead, AsyncWrite}` (NOT tokio traits)
/// because tokio's reactor doesn't exist on wasm32. The fetch layer
/// (tls.rs, http.rs) is built on these traits via `futures-rustls`.
pub struct WasmTcpStream {
    stack: Arc<Mutex<SmoltcpStack>>,
    handle: SocketHandle,
    notify: ReactorNotify,
}

/// UDP socket over the WASM tunnel.
///
/// Provides `send_to` / `recv_from` for datagram I/O. Used by the DNS
/// resolver to query nameservers through the mixnet.
pub struct WasmUdpSocket {
    stack: Arc<Mutex<SmoltcpStack>>,
    handle: SocketHandle,
    notify: ReactorNotify,
}

impl WasmTunnel {
    /// Create a new tunnel connected to the specified IPR exit node.
    ///
    /// This is the full startup sequence:
    /// 1. Start a Nym base client (connects to gateway)
    /// 2. Open an LP stream to the IPR
    /// 3. Perform the v9 IPR connect handshake (allocates virtual IPs)
    /// 4. Configure a smoltcp network stack
    /// 5. Start the bridge (shuttles packets) and reactor (drives smoltcp)
    pub async fn new(opts: TunnelOpts) -> Result<Self, FetchError> {
        let ipr_address = opts.ipr_address;
        nym_wasm_utils::console_log!("[smolmix] starting tunnel...");

        // -- 1. Start the Nym base client --
        let client_id = opts.client_id;

        let mut config = new_base_client_config(
            client_id.clone(),
            env!("CARGO_PKG_VERSION").to_string(),
            None, // nym_api: use default
            None, // nyxd: use default
            None, // debug: use default
        )
        .map_err(|e| FetchError::Tunnel(format!("config error: {e}")))?;

        // Required for current network topology handling.
        config.debug.topology.ignore_egress_epoch_role = true;

        config
            .debug
            .traffic
            .disable_main_poisson_packet_distribution = opts.disable_poisson_traffic;
        config.debug.cover_traffic.disable_loop_cover_traffic_stream = opts.disable_cover_traffic;

        let client_store = ClientStorage::new_async(&client_id, None)
            .await
            .map_err(|e| FetchError::Tunnel(format!("storage error: {e}")))?;

        if !client_store
            .has_identity_key()
            .await
            .map_err(|e| FetchError::Tunnel(format!("storage error: {e}")))?
        {
            generate_new_client_keys(&client_store)
                .await
                .map_err(|e| FetchError::Tunnel(format!("keygen error: {e}")))?;
        }

        // Check if we have an active gateway; if not, add one.
        let has_gateway = client_store
            .get_active_gateway_id()
            .await
            .map(|r| r.active_gateway_id_bs58.is_some())
            .unwrap_or(false);

        if !has_gateway {
            let user_agent = nym_bin_common::bin_info!().into();
            add_gateway(
                None, // preferred_gateway
                None, // latency_based_selection
                opts.force_tls,
                &config.client.nym_api_urls,
                user_agent,
                config.debug.topology.minimum_gateway_performance,
                config.debug.topology.ignore_ingress_epoch_role,
                &client_store,
            )
            .await
            .map_err(|e| FetchError::Tunnel(format!("gateway selection error: {e}")))?;
        }

        let storage = FullWasmClientStorage::new(&config, client_store);

        let base_builder =
            BaseClientBuilder::<QueryReqwestRpcNyxdClient, _>::new(config, storage, None);

        let mut started_client = base_builder
            .start_base()
            .await
            .map_err(|e| FetchError::Tunnel(format!("client start error: {e}")))?;

        let client_input = Arc::new(started_client.client_input.register_producer());
        let client_output = started_client.client_output.register_consumer();
        let shutdown_handle = started_client.shutdown_handle;

        // -- 2. Set up message receiver --
        let (reconstructed_sender, mut reconstructed_receiver) = mpsc::unbounded();
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .map_err(|_| FetchError::Tunnel("failed to register message receiver".into()))?;

        let request_sender = client_output.received_buffer_request_sender;

        // -- 3. Open LP stream + IPR connect handshake --
        let stream_id: u64 = rand::random();
        nym_wasm_utils::console_log!("[smolmix] connecting to IPR...");

        let allocated_ips = ipr::open_and_connect(
            &client_input,
            &mut reconstructed_receiver,
            &ipr_address,
            stream_id,
        )
        .await?;
        nym_wasm_utils::console_log!(
            "[smolmix] IPR connected — IPv4: {}, IPv6: {}",
            allocated_ips.ipv4,
            allocated_ips.ipv6
        );

        // -- 4. Configure smoltcp --
        let mut device = WasmDevice::new();
        let iface_config = Config::new(HardwareAddress::Ip);
        let mut iface = smoltcp::iface::Interface::new(iface_config, &mut device, smoltcp_now());

        iface.update_ip_addrs(|addrs| {
            addrs
                .push(IpCidr::new(IpAddress::from(allocated_ips.ipv4), 32))
                .unwrap();
        });

        iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
            .unwrap();

        let sockets = SocketSet::new(Vec::new());

        let stack = Arc::new(Mutex::new(SmoltcpStack {
            iface,
            sockets,
            device,
            wakers: Default::default(),
        }));

        // -- 5. Start bridge + reactor --
        // Data frames use a separate sequence space from Open frames (the
        // receiver's reorder buffer only tracks Data).  ConnectRequest was
        // Data seq=0, so the bridge continues from seq=1.
        let seq = Arc::new(AtomicU32::new(1));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (notify_tx, notify_rx) = mpsc::unbounded();

        reactor::start_reactor(stack.clone(), notify_rx, shutdown.clone());

        bridge::start_bridge(
            stack.clone(),
            client_input.clone(),
            reconstructed_receiver,
            ipr_address,
            stream_id,
            seq.clone(),
            notify_tx.clone(),
            shutdown.clone(),
        );

        nym_wasm_utils::console_log!("[smolmix-wasm] tunnel ready");

        Ok(Self {
            stack,
            client_input,
            ipr_address,
            stream_id,
            seq,
            notify: notify_tx,
            shutdown,
            next_port: AtomicU16::new(EPHEMERAL_PORT_START),
            allocated_ips,
            dns_cache: Mutex::new(HashMap::new()),
            dns_lock: futures::lock::Mutex::new(()),
            conn_pool: Mutex::new(HashMap::new()),
            origin_locks: Mutex::new(HashMap::new()),
            _request_sender: request_sender,
            _shutdown_handle: shutdown_handle,
        })
    }

    /// Open a TCP connection through the mixnet tunnel.
    ///
    /// Creates a smoltcp TCP socket, initiates the three-way handshake via
    /// the tunnel, and returns a stream implementing `futures::io::AsyncRead
    /// + AsyncWrite` once the connection is established.
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<WasmTcpStream, io::Error> {
        let remote = to_smoltcp_endpoint(addr);
        let local_port = self.next_port.fetch_add(1, Ordering::Relaxed);
        // 64 KiB buffers: TCP window ≈ rx buffer size, so with ~300 ms
        // mixnet RTT this gives ~213 KB/s throughput per connection
        // (vs 27 KB/s with the previous 8 KiB buffers).
        let tcp_rx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
        let tcp_tx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
        let mut socket = smoltcp_tcp::Socket::new(tcp_rx, tcp_tx);
        // Keepalive probes every 10s prevent the IPR from timing out idle
        // sessions. The probes are real IP packets that flow through the
        // tunnel, keeping the entire path alive (gateway WS, mixnet, IPR).
        socket.set_keep_alive(Some(smoltcp::time::Duration::from_millis(10_000)));

        let handle = {
            let mut s = self.stack.lock().unwrap();
            let handle = s.sockets.add(socket);
            s.wakers.insert(handle, SocketWakers::new(SocketKind::Tcp));

            // Split the borrow so we can access iface and sockets simultaneously.
            let SmoltcpStack {
                ref mut iface,
                ref mut sockets,
                ..
            } = *s;
            sockets
                .get_mut::<smoltcp_tcp::Socket>(handle)
                .connect(iface.context(), remote, local_port)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e:?}")))?;

            handle
        };

        // Notify reactor to send the SYN.
        let _ = self.notify.unbounded_send(());

        // Wait for the connection to be established.
        let stack = self.stack.clone();
        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(handle);
            let state = socket.state();
            match state {
                smoltcp_tcp::State::Established | smoltcp_tcp::State::CloseWait => {
                    Poll::Ready(Ok(()))
                }
                smoltcp_tcp::State::Closed => {
                    nym_wasm_utils::console_error!(
                        "[tunnel] TCP state: Closed — connection failed"
                    );
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        "TCP connection failed",
                    )))
                }
                _ => {
                    s.wakers.get_mut(&handle).unwrap().connect = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await?;

        Ok(WasmTcpStream {
            stack: self.stack.clone(),
            handle,
            notify: self.notify.clone(),
        })
    }

    /// Create a UDP socket bound to an ephemeral port.
    pub async fn udp_socket(&self) -> Result<WasmUdpSocket, io::Error> {
        let local_port = self.next_port.fetch_add(1, Ordering::Relaxed);
        let udp_rx = smoltcp_udp::PacketBuffer::new(
            vec![smoltcp_udp::PacketMetadata::EMPTY; 16],
            vec![0; 65535],
        );
        let udp_tx = smoltcp_udp::PacketBuffer::new(
            vec![smoltcp_udp::PacketMetadata::EMPTY; 16],
            vec![0; 65535],
        );
        let mut socket = smoltcp_udp::Socket::new(udp_rx, udp_tx);
        socket
            .bind(local_port)
            .map_err(|_| io::Error::new(io::ErrorKind::AddrInUse, "UDP bind failed"))?;

        let handle = {
            let mut s = self.stack.lock().unwrap();
            let handle = s.sockets.add(socket);
            s.wakers.insert(handle, SocketWakers::new(SocketKind::Udp));
            handle
        };

        Ok(WasmUdpSocket {
            stack: self.stack.clone(),
            handle,
            notify: self.notify.clone(),
        })
    }

    /// Gracefully disconnect from the Nym mixnet.
    ///
    /// Signals the bridge and reactor to stop. The base client shuts down
    /// when the `ShutdownTracker` is dropped.
    pub async fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        nym_wasm_utils::console_log!("[smolmix] tunnel shut down");
    }

    /// The IP addresses allocated to this tunnel by the IPR.
    pub fn allocated_ips(&self) -> IpPair {
        self.allocated_ips
    }

    /// DNS resolution cache — checked by `dns::resolve` before querying.
    pub(crate) fn dns_cache(&self) -> &Mutex<HashMap<String, IpAddr>> {
        &self.dns_cache
    }

    /// Async lock that serialises DNS lookups for request coalescing.
    pub(crate) fn dns_lock(&self) -> &futures::lock::Mutex<()> {
        &self.dns_lock
    }

    /// Get (or create) the per-origin lock for serialising concurrent requests.
    pub(crate) fn origin_lock(&self, host: &str, port: u16) -> Arc<futures::lock::Mutex<()>> {
        self.origin_locks
            .lock()
            .unwrap()
            .entry((host.to_string(), port))
            .or_insert_with(|| Arc::new(futures::lock::Mutex::new(())))
            .clone()
    }

    /// Take an idle connection from the pool (if one exists for this origin).
    pub(crate) fn take_pooled(&self, host: &str, port: u16) -> Option<PooledConn> {
        self.conn_pool
            .lock()
            .unwrap()
            .remove(&(host.to_string(), port))
    }

    /// Return a reusable connection to the pool for later use.
    pub(crate) fn return_to_pool(&self, host: String, port: u16, conn: PooledConn) {
        self.conn_pool.lock().unwrap().insert((host, port), conn);
    }
}

// ---------------------------------------------------------------------------
// WasmTcpStream — futures::io::{AsyncRead, AsyncWrite}
// ---------------------------------------------------------------------------

impl AsyncRead for WasmTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);

        if socket.can_recv() {
            let n = socket
                .recv_slice(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            Poll::Ready(Ok(n))
        } else if !socket.is_open() {
            Poll::Ready(Ok(0))
        } else {
            s.wakers.get_mut(&self.handle).unwrap().read = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl AsyncWrite for WasmTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);

        if socket.can_send() {
            let n = socket
                .send_slice(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            let _ = self.notify.unbounded_send(());
            Poll::Ready(Ok(n))
        } else if !socket.is_open() {
            Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "socket closed",
            )))
        } else {
            s.wakers.get_mut(&self.handle).unwrap().write = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // smoltcp flushes on each poll — nothing extra to do.
        let _ = self.notify.unbounded_send(());
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);
        socket.close();
        let _ = self.notify.unbounded_send(());
        Poll::Ready(Ok(()))
    }
}

impl Unpin for WasmTcpStream {}

impl Drop for WasmTcpStream {
    fn drop(&mut self) {
        let mut s = self.stack.lock().unwrap();
        s.sockets
            .get_mut::<smoltcp_tcp::Socket>(self.handle)
            .abort();
        s.sockets.remove(self.handle);
        s.wakers.remove(&self.handle);
    }
}

// ---------------------------------------------------------------------------
// PooledConn — AsyncRead + AsyncWrite delegation
// ---------------------------------------------------------------------------

impl AsyncRead for PooledConn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_read(cx, buf),
            PooledConn::Plain(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for PooledConn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_write(cx, buf),
            PooledConn::Plain(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_flush(cx),
            PooledConn::Plain(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_close(cx),
            PooledConn::Plain(s) => Pin::new(s).poll_close(cx),
        }
    }
}

impl Unpin for PooledConn {}

// ---------------------------------------------------------------------------
// WasmUdpSocket — send_to / recv_from
// ---------------------------------------------------------------------------

impl WasmUdpSocket {
    /// Send a datagram to the given address.
    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        let endpoint = to_smoltcp_endpoint(target);
        let stack = self.stack.clone();
        let handle = self.handle;
        let notify = self.notify.clone();

        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

            if socket.can_send() {
                socket
                    .send_slice(buf, endpoint)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                let _ = notify.unbounded_send(());
                Poll::Ready(Ok(buf.len()))
            } else {
                s.wakers.get_mut(&handle).unwrap().write = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    /// Receive a datagram, returning (bytes_read, source_address).
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let stack = self.stack.clone();
        let handle = self.handle;

        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

            if socket.can_recv() {
                let (n, meta) = socket
                    .recv_slice(buf)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                let src = from_smoltcp_endpoint(meta.endpoint);
                Poll::Ready(Ok((n, src)))
            } else {
                s.wakers.get_mut(&handle).unwrap().read = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }
}

impl Drop for WasmUdpSocket {
    fn drop(&mut self) {
        let mut s = self.stack.lock().unwrap();
        s.sockets.remove(self.handle);
        s.wakers.remove(&self.handle);
    }
}

// ---------------------------------------------------------------------------
// Address conversion helpers
// ---------------------------------------------------------------------------

fn to_smoltcp_endpoint(addr: SocketAddr) -> IpEndpoint {
    let ip = match addr.ip() {
        IpAddr::V4(v4) => IpAddress::Ipv4(v4),
        IpAddr::V6(v6) => IpAddress::Ipv6(v6),
    };
    IpEndpoint::new(ip, addr.port())
}

fn from_smoltcp_endpoint(ep: IpEndpoint) -> SocketAddr {
    let ip = match ep.addr {
        IpAddress::Ipv4(v4) => IpAddr::V4(v4),
        IpAddress::Ipv6(v6) => IpAddr::V6(v6),
    };
    SocketAddr::new(ip, ep.port)
}
