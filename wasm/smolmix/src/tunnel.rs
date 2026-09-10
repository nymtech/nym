// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
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
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::channel::mpsc;
use smoltcp::iface::Config;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};
use tokio::sync::Notify;

use nym_ip_packet_requests::IpPair;
use nym_wasm_client_core::client::base_client::{BaseClientBuilder, ClientInput};
use nym_wasm_client_core::client::received_buffer::ReceivedBufferMessage;
use nym_wasm_client_core::config::new_base_client_config;
use nym_wasm_client_core::helpers::{add_gateway, generate_new_client_keys};
use nym_wasm_client_core::nym_task::ShutdownTracker;
use nym_wasm_client_core::storage::ClientStorage;
use nym_wasm_client_core::storage::core_client_traits::FullWasmClientStorage;
use nym_wasm_client_core::storage::wasm_client_traits::WasmClientStorage;
use nym_wasm_client_core::{QueryReqwestRpcNyxdClient, Recipient};

use crate::bridge;
use crate::device::WasmDevice;
#[cfg(any(feature = "fetch", feature = "websocket"))]
use crate::dns::default_doh_endpoints;
use crate::error::FetchError;

// Without the DNS stack there is no DoH, so the endpoint list is never read; an
// empty default keeps the crate compiling with neither `fetch` nor `websocket`.
#[cfg(not(any(feature = "fetch", feature = "websocket")))]
fn default_doh_endpoints() -> Vec<url::Url> {
    Vec::new()
}
use crate::ipr;
use crate::reactor::{self, ReactorNotify, SmoltcpStack, smoltcp_now};
use crate::state;
use crate::stream::{self, PooledConn, WasmTcpStream, WasmUdpSocket};

/// Configuration for `setupMixTunnel(opts)`.
///
/// Construct directly or via [`TunnelOpts::builder`] for chainable configuration.
/// Performance/timeout tuning lives in [`TuningOpts`] under the `tuning` field.
pub struct TunnelOpts {
    /// `None` triggers performance-weighted auto-discovery via `ipr::discover_ipr`.
    pub ipr_address: Option<Recipient>,
    /// Identity key (base58) of the entry gateway to register with. `None`
    /// triggers performance-weighted random selection. Only consulted on the
    /// first registration for a `client_id`; an already-registered client keeps
    /// its stored gateway (registration derives a shared key, so it cannot be
    /// repointed without re-registering).
    pub preferred_gateway: Option<String>,
    /// Client storage ID. Randomise per session to get a clean client.
    pub client_id: String,
    /// Use `wss://` for gateway connections (default: `true`).
    pub force_tls: bool,
    /// Disable Poisson-distributed dummy traffic (default: `false`).
    pub disable_poisson_traffic: bool,
    /// Disable cover traffic loop (default: `false`).
    pub disable_cover_traffic: bool,
    /// Reply-SURB counts for the LP Open frame and each Data frame the
    /// bridge sends. See [`ipr::SurbsConfig`] for the values and rationale.
    pub surbs: ipr::SurbsConfig,
    /// DoH resolver endpoints, tried in order. `None` falls back to
    /// [`dns::default_doh_endpoints`] (Cloudflare, Quad9, Google).
    pub doh_endpoints: Option<Vec<url::Url>>,
    /// Passphrase used to encrypt the client's persistent storage (identity
    /// keys, gateway details, etc). `None` means plaintext storage. The same
    /// passphrase must be supplied on subsequent loads to read the same keys.
    pub storage_passphrase: Option<String>,
    /// Timeouts + buffer sizes + redirect limits. See [`TuningOpts`].
    pub tuning: TuningOpts,
}

/// Performance + protocol-limit tuning knobs.
///
/// All fields have sensible defaults via [`TuningOpts::default`]; consumers
/// override via the chainable builder methods on [`TunnelOptsBuilder`].
pub struct TuningOpts {
    /// IPR connect handshake timeout. Used for a pinned IPR, and as the final
    /// budget once auto-discovery rotation is exhausted.
    pub connect_timeout: Duration,
    /// Per-exit handshake budget during auto-discovery rotation. Bounds one
    /// candidate before rotating to the next. Only consulted on the
    /// auto-discovery path (no pinned IPR). Kept well above a single mixnet
    /// round trip, which is second-scale, so a healthy exit is not abandoned.
    pub ipr_attempt_timeout: Duration,
    /// Maximum IPR candidates to try during auto-discovery rotation before
    /// giving up. Caps total establishment cost. Auto-discovery path only.
    pub ipr_max_attempts: usize,
    /// DNS query timeout (per attempt, primary or fallback).
    pub dns_timeout: Duration,
    /// TCP keepalive interval; smoltcp probes the peer at this cadence.
    pub tcp_keepalive_interval: Duration,
    /// Per-TCP-stream RX/TX buffer size in bytes. Trades memory for throughput.
    /// Capped to `u16::MAX` (65535) to fit the TCP window field width.
    pub tcp_buffer_size: usize,
    /// Maximum HTTP redirect chain depth before `mixFetch` gives up.
    pub max_redirects: u8,
}

impl Default for TuningOpts {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(60),
            ipr_attempt_timeout: Duration::from_secs(6),
            ipr_max_attempts: 5,
            // Sized to cover a cold TLS handshake to the resolver over the mixnet
            // (~7 s measured). A rate-limited resolver answers 429 immediately, so
            // this budget only applies to a resolver that does not respond at all.
            dns_timeout: Duration::from_secs(8),
            tcp_keepalive_interval: Duration::from_secs(10),
            tcp_buffer_size: 65535,
            max_redirects: 5,
        }
    }
}

impl TunnelOpts {
    /// Start a chainable builder.
    pub fn builder() -> TunnelOptsBuilder {
        TunnelOptsBuilder::default()
    }
}

/// Chainable builder for [`TunnelOpts`]. Setters for tuning fields delegate
/// into the nested [`TuningOpts`] at `build()` time, so callers see a flat API.
#[derive(Default)]
pub struct TunnelOptsBuilder {
    ipr_address: Option<Recipient>,
    preferred_gateway: Option<String>,
    client_id: Option<String>,
    force_tls: Option<bool>,
    disable_poisson_traffic: Option<bool>,
    disable_cover_traffic: Option<bool>,
    surbs: Option<ipr::SurbsConfig>,
    doh_endpoints: Option<Vec<url::Url>>,
    storage_passphrase: Option<String>,
    connect_timeout: Option<Duration>,
    ipr_attempt_timeout: Option<Duration>,
    ipr_max_attempts: Option<usize>,
    dns_timeout: Option<Duration>,
    tcp_keepalive_interval: Option<Duration>,
    tcp_buffer_size: Option<usize>,
    max_redirects: Option<u8>,
}

impl TunnelOptsBuilder {
    pub fn ipr_address(mut self, v: Recipient) -> Self {
        self.ipr_address = Some(v);
        self
    }
    pub fn preferred_gateway(mut self, v: impl Into<String>) -> Self {
        self.preferred_gateway = Some(v.into());
        self
    }
    pub fn client_id(mut self, v: impl Into<String>) -> Self {
        self.client_id = Some(v.into());
        self
    }
    pub fn force_tls(mut self, v: bool) -> Self {
        self.force_tls = Some(v);
        self
    }
    pub fn disable_poisson_traffic(mut self, v: bool) -> Self {
        self.disable_poisson_traffic = Some(v);
        self
    }
    pub fn disable_cover_traffic(mut self, v: bool) -> Self {
        self.disable_cover_traffic = Some(v);
        self
    }
    pub fn surbs(mut self, v: ipr::SurbsConfig) -> Self {
        self.surbs = Some(v);
        self
    }
    pub fn doh_endpoints(mut self, v: Vec<url::Url>) -> Self {
        self.doh_endpoints = Some(v);
        self
    }
    pub fn storage_passphrase(mut self, v: impl Into<String>) -> Self {
        self.storage_passphrase = Some(v.into());
        self
    }
    pub fn connect_timeout(mut self, v: Duration) -> Self {
        self.connect_timeout = Some(v);
        self
    }
    pub fn ipr_attempt_timeout(mut self, v: Duration) -> Self {
        self.ipr_attempt_timeout = Some(v);
        self
    }
    pub fn ipr_max_attempts(mut self, v: usize) -> Self {
        self.ipr_max_attempts = Some(v);
        self
    }
    pub fn dns_timeout(mut self, v: Duration) -> Self {
        self.dns_timeout = Some(v);
        self
    }
    pub fn tcp_keepalive_interval(mut self, v: Duration) -> Self {
        self.tcp_keepalive_interval = Some(v);
        self
    }
    pub fn tcp_buffer_size(mut self, v: usize) -> Self {
        self.tcp_buffer_size = Some(v);
        self
    }
    pub fn max_redirects(mut self, v: u8) -> Self {
        self.max_redirects = Some(v);
        self
    }

    pub fn build(self) -> TunnelOpts {
        let defaults = TuningOpts::default();
        TunnelOpts {
            ipr_address: self.ipr_address,
            preferred_gateway: self.preferred_gateway,
            client_id: self.client_id.unwrap_or_else(|| "smolmix-wasm".to_string()),
            force_tls: self.force_tls.unwrap_or(true),
            disable_poisson_traffic: self.disable_poisson_traffic.unwrap_or(false),
            disable_cover_traffic: self.disable_cover_traffic.unwrap_or(false),
            surbs: self.surbs.unwrap_or_default(),
            doh_endpoints: self.doh_endpoints,
            storage_passphrase: self.storage_passphrase,
            tuning: TuningOpts {
                connect_timeout: self.connect_timeout.unwrap_or(defaults.connect_timeout),
                ipr_attempt_timeout: self
                    .ipr_attempt_timeout
                    .unwrap_or(defaults.ipr_attempt_timeout),
                ipr_max_attempts: self.ipr_max_attempts.unwrap_or(defaults.ipr_max_attempts),
                dns_timeout: self.dns_timeout.unwrap_or(defaults.dns_timeout),
                tcp_keepalive_interval: self
                    .tcp_keepalive_interval
                    .unwrap_or(defaults.tcp_keepalive_interval),
                tcp_buffer_size: self.tcp_buffer_size.unwrap_or(defaults.tcp_buffer_size),
                max_redirects: self.max_redirects.unwrap_or(defaults.max_redirects),
            },
        }
    }
}

/// The mixnet tunnel. Owns the smoltcp stack, base client, and connection pool.
pub struct WasmTunnel {
    stack: SmoltcpStack,
    notify: ReactorNotify,
    allocated_ips: IpPair,
    /// Resolved per-tunnel DoH endpoints, tried in order. Falls back to
    /// [`dns::default_doh_endpoints`] when the caller didn't override.
    doh_endpoints: Vec<url::Url>,
    /// All timeouts + buffer sizes + redirect limits; populated from
    /// [`TunnelOpts::tuning`] at construction.
    tuning: TuningOpts,
    /// Plain per-session DNS cache. No TTL respect (cache lives until tunnel
    /// shutdown). See [`dns::resolve`] for usage.
    dns_cache: Mutex<HashMap<String, IpAddr>>,
    /// Serialises DNS lookups so concurrent callers coalesce on the cache.
    dns_lock: futures::lock::Mutex<()>,
    /// One idle connection per (host, port).
    conn_pool: Mutex<HashMap<(String, u16), PooledConn>>,
    /// Per-origin locks to avoid stampeding parallel TCP+TLS handshakes.
    #[allow(clippy::type_complexity)]
    origin_locks: Mutex<HashMap<(String, u16), Arc<futures::lock::Mutex<()>>>>,
    /// `Mutex<Option<_>>` because `ShutdownTracker::shutdown(self).await`
    /// takes ownership, but `WasmTunnel` lives in a `OnceLock`.
    base_tracker: Mutex<Option<ShutdownTracker>>,
    /// Child of `base_tracker`; bridge + reactor spawn through it.
    smolmix_tracker: Mutex<Option<ShutdownTracker>>,
    state: state::State,
}

/// Handles the Nym base client hands back after `start_base()`.
struct ClientHandles {
    client_input: Arc<ClientInput>,
    reconstructed_receiver: ipr::ReconstructedReceiver,
    shutdown_handle: ShutdownTracker,
    /// Lifted out so `ipr::discover_ipr` reuses the same URLs the base client did.
    nym_api_urls: Vec<url::Url>,
}

/// smoltcp handles returned by `init_network_stack` (reactor + bridge already spawned).
struct NetworkStack {
    stack: SmoltcpStack,
    notify: ReactorNotify,
}

impl WasmTunnel {
    /// Connect to the mixnet and establish an IPR tunnel.
    pub async fn new(opts: TunnelOpts) -> Result<Self, FetchError> {
        nym_wasm_utils::console_log!("[smolmix] starting tunnel...");

        let ClientHandles {
            client_input,
            mut reconstructed_receiver,
            shutdown_handle,
            nym_api_urls,
        } = Self::start_nym_client(&opts).await?;

        // Cascade points at smolmix_tracker so state.fail() only kills
        // smolmix tasks; shutdown() handles the base client.
        let smolmix_tracker = shutdown_handle.child_tracker();
        let state = state::State::new(smolmix_tracker.clone_shutdown_token());

        let (ipr_address, allocated_ips, negotiated_mtu, stream_id) = match opts.ipr_address {
            Some(addr) => {
                // Pinned IPR: single attempt, full connect_timeout, cross-version
                // fallback. A single fixed exit has nothing to rotate to, so
                // fast-fail rotation is skipped here by design.
                //
                // Best-effort: read the node's version from the directory to pick
                // the protocol version. Not found ⇒ None ⇒ connect defaults to v9.
                let version = match ipr::lookup_node_version(&nym_api_urls, &addr).await {
                    Ok(version) => Some(version),
                    Err(e) => {
                        crate::util::debug_log!(
                            "[smolmix] IPR version lookup failed ({e}); defaulting to v9"
                        );
                        None
                    }
                };
                let stream_id: u64 = rand::random();
                let (ips, mtu) = Self::ipr_handshake(
                    &client_input,
                    &mut reconstructed_receiver,
                    &addr,
                    stream_id,
                    opts.surbs,
                    opts.tuning.connect_timeout,
                    version.as_ref(),
                )
                .await?;
                (addr, ips, mtu, stream_id)
            }
            None => {
                nym_wasm_utils::console_log!("[smolmix] no IPR specified, auto-discovering...");
                let candidates = ipr::discover_ipr(&nym_api_urls).await?;
                Self::connect_rotating(
                    &client_input,
                    &mut reconstructed_receiver,
                    &candidates,
                    opts.surbs,
                    opts.tuning.ipr_attempt_timeout,
                    opts.tuning.ipr_max_attempts,
                )
                .await?
            }
        };

        let NetworkStack { stack, notify } = Self::init_network_stack(
            allocated_ips,
            negotiated_mtu,
            client_input.clone(),
            reconstructed_receiver,
            ipr_address,
            stream_id,
            &smolmix_tracker,
            &state,
            opts.surbs.data,
        )?;

        state.set(state::TunnelState::Ready);
        nym_wasm_utils::console_log!("[smolmix] tunnel ready");

        Ok(Self {
            stack,
            notify,
            allocated_ips,
            doh_endpoints: opts.doh_endpoints.unwrap_or_else(default_doh_endpoints),
            tuning: opts.tuning,
            dns_cache: Mutex::new(HashMap::new()),
            dns_lock: futures::lock::Mutex::new(()),
            conn_pool: Mutex::new(HashMap::new()),
            origin_locks: Mutex::new(HashMap::new()),
            base_tracker: Mutex::new(Some(shutdown_handle)),
            smolmix_tracker: Mutex::new(Some(smolmix_tracker)),
            state,
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

        let client_store =
            ClientStorage::new_async(&opts.client_id, opts.storage_passphrase.clone())
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
            .map_err(|e| FetchError::Tunnel(format!("gateway-storage error: {e}")))?
            .active_gateway_id_bs58
            .is_some();

        if !has_gateway {
            let user_agent = nym_bin_common::bin_info!().into();
            add_gateway(
                opts.preferred_gateway.clone(),
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
            BaseClientBuilder::<QueryReqwestRpcNyxdClient, _>::new(config.clone(), storage, None);

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
            shutdown_handle: started_client.shutdown_handle,
            nym_api_urls: config.client.nym_api_urls.clone(),
        })
    }

    /// Auto-discovery establishment: try candidates in preference order with a
    /// short per-exit budget, rotating to a different exit on timeout or error.
    /// The per-exit `attempt_timeout` bounds the whole handshake for one exit
    /// (Open plus connect, including any internal version fallback), so a black-
    /// holing exit costs one budget rather than the full `connect_timeout`.
    ///
    /// A fresh stream id per attempt keeps a late response from a rotated-away
    /// exit from being mistaken for the current attempt's reply. On exhaustion of
    /// the candidates (capped at `max_attempts`) it fails fast rather than falling
    /// back to a slow full-timeout attempt.
    async fn connect_rotating(
        client_input: &Arc<ClientInput>,
        receiver: &mut ipr::ReconstructedReceiver,
        candidates: &[(Recipient, semver::Version)],
        surbs: ipr::SurbsConfig,
        attempt_timeout: Duration,
        max_attempts: usize,
    ) -> Result<(Recipient, IpPair, Option<u16>, u64), FetchError> {
        let mut last_err: Option<FetchError> = None;
        for (addr, version) in candidates.iter().take(max_attempts) {
            let stream_id: u64 = rand::random();
            let started = wasmtimer::std::Instant::now();
            nym_wasm_utils::console_log!("[smolmix] connecting to IPR {addr}...");
            // No outer per-exit timeout. `open_and_connect` is already bounded per
            // protocol version, and wrapping the whole thing strangles the
            // v10-then-v9 fallback: many nodes advertise v10 in the directory but
            // run v9, so v10 gets no reply and the connect must fall through to v9
            // to succeed. `attempt_timeout` is therefore the per-version budget;
            // one exit may spend up to two of them (v10 probe, then v9 connect)
            // before rotating.
            match ipr::open_and_connect(
                client_input,
                receiver,
                addr,
                stream_id,
                surbs,
                attempt_timeout,
                Some(version),
            )
            .await
            {
                Ok((ips, mtu)) => {
                    nym_wasm_utils::console_log!(
                        "[smolmix] IPR connected: {addr} (in {:?})",
                        started.elapsed()
                    );
                    return Ok((*addr, ips, mtu, stream_id));
                }
                Err(e) => {
                    nym_wasm_utils::console_log!(
                        "[smolmix] IPR {addr} failed after {:?}: {e}; rotating",
                        started.elapsed()
                    );
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| FetchError::Tunnel("no IPR candidates to try".into())))
    }

    /// Open the LP stream + run the IPR connect handshake. Returns the IPs the
    /// IPR allocated and the MTU it reported (`None` against a pre-v10 IPR).
    #[allow(clippy::too_many_arguments)]
    async fn ipr_handshake(
        client_input: &Arc<ClientInput>,
        receiver: &mut ipr::ReconstructedReceiver,
        ipr_address: &Recipient,
        stream_id: u64,
        surbs: ipr::SurbsConfig,
        connect_timeout: Duration,
        node_version: Option<&semver::Version>,
    ) -> Result<(IpPair, Option<u16>), FetchError> {
        nym_wasm_utils::console_log!("[smolmix] connecting to IPR...");
        let (allocated_ips, negotiated_mtu) = ipr::open_and_connect(
            client_input,
            receiver,
            ipr_address,
            stream_id,
            surbs,
            connect_timeout,
            node_version,
        )
        .await?;
        nym_wasm_utils::console_log!("[smolmix] IPR connected");
        crate::util::debug_log!(
            "[smolmix] allocated IPv4: {}, IPv6: {}, MTU: {:?}",
            allocated_ips.ipv4,
            allocated_ips.ipv6,
            negotiated_mtu,
        );
        Ok((allocated_ips, negotiated_mtu))
    }

    /// Build the smoltcp interface, spawn the reactor + bridge, and return
    /// the shared handles the tunnel keeps to drive the stack.
    #[allow(clippy::too_many_arguments)]
    fn init_network_stack(
        allocated_ips: IpPair,
        negotiated_mtu: Option<u16>,
        client_input: Arc<ClientInput>,
        reconstructed_receiver: ipr::ReconstructedReceiver,
        ipr_address: Recipient,
        stream_id: u64,
        tracker: &ShutdownTracker,
        state: &state::State,
        data_surbs: u32,
    ) -> Result<NetworkStack, FetchError> {
        let mut device = WasmDevice::new(negotiated_mtu);
        let iface_config = Config::new(HardwareAddress::Ip);
        let mut iface = smoltcp::iface::Interface::new(iface_config, &mut device, smoltcp_now());

        // smoltcp's address + route tables are heapless vecs with capacity
        // IFACE_MAX_ADDR_COUNT / IFACE_MAX_ROUTE_COUNT (default 8 each). We add 2
        // of each on a fresh interface, so a push failure means the table is full.
        let mut addr_full = false;
        iface.update_ip_addrs(|addrs| {
            if addrs
                .push(IpCidr::new(IpAddress::from(allocated_ips.ipv4), 32))
                .is_err()
                || addrs
                    .push(IpCidr::new(IpAddress::from(allocated_ips.ipv6), 128))
                    .is_err()
            {
                addr_full = true;
            }
        });
        if addr_full {
            return Err(FetchError::Tunnel("smoltcp address table full".into()));
        }
        iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
            .map_err(|_| FetchError::Tunnel("smoltcp route table full".into()))?;
        iface
            .routes_mut()
            .add_default_ipv6_route(Ipv6Address::UNSPECIFIED)
            .map_err(|_| FetchError::Tunnel("smoltcp route table full".into()))?;

        let stack = SmoltcpStack::new(iface, device);
        let notify = Arc::new(Notify::new());

        reactor::start_reactor(stack.clone(), notify.clone(), tracker, state.clone());
        bridge::start_bridge(
            stack.clone(),
            client_input,
            reconstructed_receiver,
            ipr_address,
            stream_id,
            notify.clone(),
            tracker,
            state.clone(),
            data_surbs,
        );

        Ok(NetworkStack { stack, notify })
    }

    /// Open a TCP connection through the tunnel (SYN -> established).
    pub async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<WasmTcpStream> {
        stream::tcp_connect(
            self.stack.clone(),
            self.notify.clone(),
            addr,
            self.tcp_keepalive_interval(),
            self.tcp_buffer_size(),
        )
        .await
    }

    /// Create a UDP socket bound to an ephemeral port.
    pub async fn udp_socket(&self) -> io::Result<WasmUdpSocket> {
        stream::create_udp_socket(self.stack.clone(), self.notify.clone())
    }

    /// Gracefully disconnect from the Nym mixnet.
    ///
    /// Signals the bridge and reactor to stop, then drops the base-client
    /// handles so the Nym client stops consuming cover/Poisson traffic and
    /// closes its gateway WebSocket. `WasmTunnel` itself lives in a
    /// `OnceLock` for the lifetime of the worker, so dropping `self` is not
    /// an option; we drop the inner handles instead.
    pub async fn shutdown(&self) {
        use state::TunnelState;
        if matches!(
            self.state.get(),
            TunnelState::ShuttingDown | TunnelState::Shutdown
        ) {
            return;
        }
        self.state.set(TunnelState::ShuttingDown);

        // Cancel + wait, child first. The base token cancels the whole
        // subtree, but each level's TaskTracker only waits on its own
        // tasks, so both need an explicit `.shutdown().await`.
        // Take the trackers out of their Mutexes first so the sync guards drop
        // before the async `.shutdown().await` (clippy::await_holding_lock).
        let smolmix_tracker = self
            .smolmix_tracker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let base_tracker = self
            .base_tracker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(tracker) = smolmix_tracker {
            tracker.shutdown().await;
        }
        if let Some(tracker) = base_tracker {
            tracker.shutdown().await;
        }

        // Don't overwrite a Failed state that was set during teardown.
        if !matches!(self.state.get(), TunnelState::Failed { .. }) {
            self.state.set(TunnelState::Shutdown);
        }
        nym_wasm_utils::console_log!("[smolmix] tunnel shut down");
    }

    /// The IP addresses allocated to this tunnel by the IPR.
    pub fn allocated_ips(&self) -> IpPair {
        self.allocated_ips
    }

    /// Panic-aware via `State::get`'s short-circuit.
    pub(crate) fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub(crate) fn tunnel_state(&self) -> state::TunnelState {
        self.state.get()
    }

    /// DNS resolution cache, checked by `dns::resolve` before querying.
    pub(crate) fn dns_cache(&self) -> &Mutex<HashMap<String, IpAddr>> {
        &self.dns_cache
    }

    /// Async lock that serialises DNS lookups for request coalescing.
    pub(crate) fn dns_lock(&self) -> &futures::lock::Mutex<()> {
        &self.dns_lock
    }

    /// DoH resolver endpoints used by `dns::resolve`, tried in order.
    pub(crate) fn doh_endpoints(&self) -> &[url::Url] {
        &self.doh_endpoints
    }
    /// Per-query DNS timeout (used in `dns::resolve_with`).
    pub(crate) fn dns_timeout(&self) -> Duration {
        self.tuning.dns_timeout
    }
    /// TCP keepalive interval applied to every new `WasmTcpStream`.
    pub(crate) fn tcp_keepalive_interval(&self) -> Duration {
        self.tuning.tcp_keepalive_interval
    }
    /// TCP RX/TX buffer size in bytes applied to every new `WasmTcpStream`.
    pub(crate) fn tcp_buffer_size(&self) -> usize {
        self.tuning.tcp_buffer_size
    }
    /// Maximum HTTP redirect chain depth before `mixFetch` gives up.
    pub(crate) fn max_redirects(&self) -> u8 {
        self.tuning.max_redirects
    }

    /// Get (or create) the per-origin lock for serialising concurrent requests.
    pub(crate) fn origin_lock(&self, host: &str, port: u16) -> Arc<futures::lock::Mutex<()>> {
        self.origin_locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry((host.to_string(), port))
            .or_insert_with(|| Arc::new(futures::lock::Mutex::new(())))
            .clone()
    }

    /// Take an idle connection from the pool (if one exists for this origin).
    pub(crate) fn take_pooled(&self, host: &str, port: u16) -> Option<PooledConn> {
        self.conn_pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(host.to_string(), port))
    }

    /// Return a reusable connection to the pool for later use.
    pub(crate) fn return_to_pool(&self, host: String, port: u16, conn: PooledConn) {
        self.conn_pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((host, port), conn);
    }
}
