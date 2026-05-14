// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! WASM mixnet tunnel. Manages a smoltcp TCP/IP stack connected to the Nym
//! mixnet via an IPR (IP Packet Router), running in a browser Web Worker.
//!
//! Data flow:
//! ```text
//! poll_write → smoltcp → device tx → bridge → LP frame → mixnet → IPR → internet
//! internet → IPR → mixnet → bridge → LP decode → device rx → smoltcp → poll_read
//! ```

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures::channel::mpsc;
use smoltcp::iface::{Config, SocketSet};
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::socket::udp as smoltcp_udp;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};

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
use crate::reactor::{self, smoltcp_now, ReactorNotify, SmoltcpStack};
use crate::stream::{to_smoltcp_endpoint, PooledConn, WasmTcpStream, WasmUdpSocket};

/// Starting ephemeral port for TCP/UDP sockets.
const EPHEMERAL_PORT_START: u16 = 49152;

/// Configuration for `setupMixTunnel(opts)`.
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
    /// Reply-SURB counts for the LP Open frame and each Data frame the
    /// bridge sends. See [`ipr::SurbsConfig`]. Defaults to open=5, data=2.
    pub surbs: ipr::SurbsConfig,
}

/// The mixnet tunnel. Owns the smoltcp stack, base client, and connection pool.
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
    dns_cache: Mutex<HashMap<String, IpAddr>>,
    /// Serialises DNS lookups so concurrent requests coalesce.
    dns_lock: futures::lock::Mutex<()>,
    /// One idle connection per (host, port).
    conn_pool: Mutex<HashMap<(String, u16), PooledConn>>,
    /// Per-origin locks to avoid stampeding parallel TCP+TLS handshakes.
    origin_locks: Mutex<HashMap<(String, u16), Arc<futures::lock::Mutex<()>>>>,
    // Dropping these shuts down the base client.
    _request_sender: ReceivedBufferRequestSender,
    _shutdown_handle: ShutdownTracker,
}

/// Handles the Nym base client hands back after `start_base()`.
struct ClientHandles {
    client_input: Arc<ClientInput>,
    reconstructed_receiver: ipr::ReconstructedReceiver,
    request_sender: ReceivedBufferRequestSender,
    shutdown_handle: ShutdownTracker,
}

/// smoltcp handles returned by `init_network_stack` (reactor + bridge already spawned).
struct NetworkStack {
    stack: Arc<Mutex<SmoltcpStack>>,
    seq: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    notify: ReactorNotify,
}

impl WasmTunnel {
    /// Connect to the mixnet and establish an IPR tunnel.
    pub async fn new(opts: TunnelOpts) -> Result<Self, FetchError> {
        let ipr_address = opts.ipr_address;
        nym_wasm_utils::console_log!("[smolmix] starting tunnel...");

        let ClientHandles {
            client_input,
            mut reconstructed_receiver,
            request_sender,
            shutdown_handle,
        } = Self::start_nym_client(&opts).await?;

        let stream_id: u64 = rand::random();
        let allocated_ips = Self::ipr_handshake(
            &client_input,
            &mut reconstructed_receiver,
            &ipr_address,
            stream_id,
            opts.surbs,
        )
        .await?;

        let NetworkStack {
            stack,
            seq,
            shutdown,
            notify,
        } = Self::init_network_stack(
            allocated_ips,
            client_input.clone(),
            reconstructed_receiver,
            ipr_address,
            stream_id,
            opts.surbs.data,
        );

        nym_wasm_utils::console_log!("[smolmix-wasm] tunnel ready");

        Ok(Self {
            stack,
            client_input,
            ipr_address,
            stream_id,
            seq,
            notify,
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

    /// Configure storage, generate identity keys if needed, register a gateway,
    /// and start the Nym base client. Returns the producer/consumer channels.
    async fn start_nym_client(opts: &TunnelOpts) -> Result<ClientHandles, FetchError> {
        let mut config = new_base_client_config(
            opts.client_id.clone(),
            env!("CARGO_PKG_VERSION").to_string(),
            None, // nym_api: use default
            None, // nyxd: use default
            None, // debug: use default
        )
        .map_err(|e| FetchError::Tunnel(format!("config error: {e}")))?;

        config.debug.topology.ignore_egress_epoch_role = true;
        config
            .debug
            .traffic
            .disable_main_poisson_packet_distribution = opts.disable_poisson_traffic;
        config.debug.cover_traffic.disable_loop_cover_traffic_stream = opts.disable_cover_traffic;

        let client_store = ClientStorage::new_async(&opts.client_id, None)
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

        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .map_err(|_| FetchError::Tunnel("failed to register message receiver".into()))?;

        Ok(ClientHandles {
            client_input,
            reconstructed_receiver,
            request_sender: client_output.received_buffer_request_sender,
            shutdown_handle: started_client.shutdown_handle,
        })
    }

    /// Open the LP stream + run the IPR v9 connect handshake. Returns the
    /// IPs the IPR allocated for this tunnel.
    async fn ipr_handshake(
        client_input: &Arc<ClientInput>,
        receiver: &mut ipr::ReconstructedReceiver,
        ipr_address: &Recipient,
        stream_id: u64,
        surbs: ipr::SurbsConfig,
    ) -> Result<IpPair, FetchError> {
        nym_wasm_utils::console_log!("[smolmix] connecting to IPR...");
        let allocated_ips =
            ipr::open_and_connect(client_input, receiver, ipr_address, stream_id, surbs).await?;
        nym_wasm_utils::console_log!("[smolmix] IPR connected");
        crate::util::debug_log!(
            "[smolmix] allocated IPv4: {}, IPv6: {}",
            allocated_ips.ipv4,
            allocated_ips.ipv6,
        );
        Ok(allocated_ips)
    }

    /// Build the smoltcp interface, spawn the reactor + bridge, and return
    /// the shared handles the tunnel keeps to drive the stack.
    fn init_network_stack(
        allocated_ips: IpPair,
        client_input: Arc<ClientInput>,
        reconstructed_receiver: ipr::ReconstructedReceiver,
        ipr_address: Recipient,
        stream_id: u64,
        data_surbs: u32,
    ) -> NetworkStack {
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

        let stack = Arc::new(Mutex::new(SmoltcpStack {
            iface,
            sockets: SocketSet::new(Vec::new()),
            device,
        }));

        // Bridge starts at seq=1 (ConnectRequest was Data seq=0).
        let seq = Arc::new(AtomicU32::new(1));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (notify_tx, notify_rx) = mpsc::unbounded();

        reactor::start_reactor(stack.clone(), notify_rx, shutdown.clone());
        bridge::start_bridge(
            stack.clone(),
            client_input,
            reconstructed_receiver,
            ipr_address,
            stream_id,
            seq.clone(),
            notify_tx.clone(),
            shutdown.clone(),
            data_surbs,
        );

        NetworkStack {
            stack,
            seq,
            shutdown,
            notify: notify_tx,
        }
    }

    /// Allocate the next ephemeral port (wraps at range boundary).
    ///
    /// wasm32-unknown-unknown is single-threaded, so a plain load/store is
    /// race-free; the atomic only exists to satisfy `Sync` on `WasmTunnel`.
    fn next_ephemeral_port(&self) -> u16 {
        let current = self.next_port.load(Ordering::Relaxed);
        let next = if current >= u16::MAX {
            EPHEMERAL_PORT_START
        } else {
            current + 1
        };
        self.next_port.store(next, Ordering::Relaxed);
        current
    }

    /// Open a TCP connection through the tunnel (SYN → established).
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<WasmTcpStream, io::Error> {
        let remote = to_smoltcp_endpoint(addr);
        let local_port = self.next_ephemeral_port();
        let tcp_rx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
        let tcp_tx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
        let mut socket = smoltcp_tcp::Socket::new(tcp_rx, tcp_tx);
        // Keepalive probes keep the entire tunnel path alive (gateway WS, mixnet, IPR).
        socket.set_keep_alive(Some(smoltcp::time::Duration::from_millis(10_000)));

        let handle = {
            let mut s = self.stack.lock().unwrap();
            let handle = s.sockets.add(socket);

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

        // Wait for the connection to be established. smoltcp's `set_state`
        // fires both rx and tx wakers on every state transition, so registering
        // on the recv waker is enough to observe Established, CloseWait, and
        // Closed alike.
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
                    crate::util::debug_error!("[tunnel] TCP state: Closed, connection failed");
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        "TCP connection failed",
                    )))
                }
                _ => {
                    socket.register_recv_waker(cx.waker());
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
        let local_port = self.next_ephemeral_port();
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
            s.sockets.add(socket)
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
        // Wake the reactor immediately so teardown doesn't sit out the
        // current `poll_delay` sleep (up to `MAX_IDLE`).
        let _ = self.notify.unbounded_send(());
        nym_wasm_utils::console_log!("[smolmix] tunnel shut down");
    }

    /// The IP addresses allocated to this tunnel by the IPR.
    pub fn allocated_ips(&self) -> IpPair {
        self.allocated_ips
    }

    /// DNS resolution cache, checked by `dns::resolve` before querying.
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
