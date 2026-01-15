// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// LP (Lewes Protocol) Metrics Documentation
//
// This module implements comprehensive metrics collection for LP operations using nym-metrics macros.
// All metrics are automatically prefixed with the package name (nym_gateway) when registered.
//
// ## Connection Metrics (via NetworkStats in nym-node-metrics)
// - active_lp_connections: Gauge tracking current active LP connections (incremented on accept, decremented on close)
//
// ## Handler Metrics (in handler.rs)
// - lp_connections_total: Counter for total LP connections handled
// - lp_client_hello_failed: Counter for ClientHello failures (timestamp validation, protocol errors)
// - lp_handshakes_success: Counter for successful handshake completions
// - lp_handshakes_failed: Counter for failed handshakes
// - lp_handshake_duration_seconds: Histogram of handshake durations (buckets: 10ms to 10s)
// - lp_timestamp_validation_accepted: Counter for timestamp validations that passed
// - lp_timestamp_validation_rejected: Counter for timestamp validations that failed
// - lp_errors_handshake: Counter for handshake errors
// - lp_errors_send_response: Counter for errors sending registration responses
// - lp_errors_timestamp_too_old: Counter for ClientHello timestamps that are too old
// - lp_errors_timestamp_too_far_future: Counter for ClientHello timestamps that are too far in the future
//
// ## Registration Metrics (in registration.rs)
// - lp_registration_attempts_total: Counter for all registration attempts
// - lp_registration_success_total: Counter for successful registrations (any mode)
// - lp_registration_failed_total: Counter for failed registrations (any mode)
// - lp_registration_failed_timestamp: Counter for registrations rejected due to invalid timestamp
// - lp_registration_duration_seconds: Histogram of registration durations (buckets: 100ms to 30s)
//
// ## Mode-Specific Registration Metrics (in registration.rs)
// - lp_registration_dvpn_attempts: Counter for dVPN mode registration attempts
// - lp_registration_dvpn_success: Counter for successful dVPN registrations
// - lp_registration_dvpn_failed: Counter for failed dVPN registrations
// - lp_registration_mixnet_attempts: Counter for Mixnet mode registration attempts
// - lp_registration_mixnet_success: Counter for successful Mixnet registrations
// - lp_registration_mixnet_failed: Counter for failed Mixnet registrations
//
// ## Credential Verification Metrics (in registration.rs)
// - lp_credential_verification_attempts: Counter for credential verification attempts
// - lp_credential_verification_success: Counter for successful credential verifications
// - lp_credential_verification_failed: Counter for failed credential verifications
// - lp_bandwidth_allocated_bytes_total: Counter for total bandwidth allocated (in bytes)
//
// ## Error Categorization Metrics
// - lp_errors_wg_peer_registration: Counter for WireGuard peer registration failures
//
// ## Connection Lifecycle Metrics (in handler.rs)
// - lp_connection_duration_seconds: Histogram of connection duration from start to end (buckets: 1s to 24h)
// - lp_connection_bytes_received_total: Counter for total bytes received including protocol framing
// - lp_connection_bytes_sent_total: Counter for total bytes sent including protocol framing
// - lp_connections_completed_gracefully: Counter for connections that completed successfully
// - lp_connections_completed_with_error: Counter for connections that terminated with an error
//
// ## State Cleanup Metrics (in cleanup task)
// - lp_states_cleanup_handshake_removed: Counter for stale handshakes removed by cleanup task
// - lp_states_cleanup_session_removed: Counter for stale sessions removed by cleanup task
// - lp_states_cleanup_demoted_removed: Counter for demoted (read-only) sessions removed by cleanup task
//
// ## Subsession/Rekeying Metrics (in handler.rs)
// - lp_subsession_kk2_sent: Counter for SubsessionKK2 responses sent (indicates client initiated rekeying)
// - lp_subsession_complete: Counter for successful subsession promotions
// - lp_subsession_receiver_index_collision: Counter for subsession receiver_index collisions
//
// ## Usage Example
// To view metrics, the nym-metrics registry automatically collects all metrics.
// They can be exported via Prometheus format using the metrics endpoint.

use crate::error::GatewayError;
use crate::node::ActiveClientsStore;
use dashmap::DashMap;
use nym_config::serde_helpers::de_maybe_port;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_storage::GatewayStorage;
use nym_lp::state_machine::LpStateMachine;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownTracker;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Semaphore};
use tracing::*;

pub use nym_mixnet_client::forwarder::{
    mix_forwarding_channels, MixForwardingReceiver, MixForwardingSender,
};
pub use nym_wireguard::{PeerControlRequest, WireguardGatewayData};

mod data_handler;
pub mod handler;
mod messages;
mod registration;

/// Configuration for LP listener
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpConfig {
    /// Bind address for the TCP LP control traffic.
    /// default: `[::]:41264`
    pub control_bind_address: SocketAddr,

    /// Bind address for the UDP LP data traffic.
    /// default: `[::]:51264`
    pub data_bind_address: SocketAddr,

    /// Custom announced port for listening for the TCP LP control traffic.
    /// If unspecified, the value from the `control_bind_address` will be used instead
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_control_port: Option<u16>,

    /// Custom announced port for listening for the UDP LP data traffic.
    /// If unspecified, the value from the `data_bind_address` will be used instead    
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_data_port: Option<u16>,

    /// Auxiliary configuration
    #[serde(default)]
    pub debug: LpDebug,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpDebug {
    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Maximum acceptable age of ClientHello timestamp in seconds (default: 30)
    ///
    /// ClientHello messages with timestamps older than this will be rejected
    /// to prevent replay attacks. Value should be:
    /// - Large enough to account for clock skew and network latency
    /// - Small enough to limit replay attack window
    ///
    /// Recommended: 30-60 seconds
    #[serde(with = "humantime_serde")]
    pub timestamp_tolerance: Duration,

    /// Use mock ecash manager for testing (default: false)
    ///
    /// When enabled, the LP listener will use a mock ecash verifier that
    /// accepts any credential without blockchain verification. This is
    /// useful for testing the LP protocol implementation without requiring
    /// a full blockchain/contract setup.
    ///
    /// WARNING: Only use this for local testing! Never enable in production.
    pub use_mock_ecash: bool,

    /// Maximum age of in-progress handshakes before cleanup (default: 90s)
    ///
    /// Handshakes should complete quickly (3-5 packets). This TTL accounts for:
    /// - Network latency and retransmits
    /// - Slow clients
    /// - Clock skew tolerance
    ///
    /// Stale handshakes are removed by the cleanup task to prevent memory leaks.
    #[serde(with = "humantime_serde")]
    pub handshake_ttl: Duration,

    /// Maximum age of established sessions before cleanup (default: 24h)
    ///
    /// Sessions can be long-lived for dVPN tunnels. This TTL should be set
    /// high enough to accommodate expected usage patterns:
    /// - dVPN sessions: hours to days
    /// - Registration: minutes
    ///
    /// Sessions with no activity for this duration are removed by the cleanup task.
    #[serde(with = "humantime_serde")]
    pub session_ttl: Duration,

    /// Maximum age of demoted (read-only) sessions before cleanup (default: 60s)
    ///
    /// After subsession promotion, old sessions enter ReadOnlyTransport state.
    /// They only need to stay alive briefly to drain in-flight packets.
    /// This shorter TTL prevents memory buildup from frequent rekeying.
    #[serde(with = "humantime_serde")]
    pub demoted_session_ttl: Duration,

    /// How often to run the state cleanup task (default: 5 minutes)
    ///
    /// The cleanup task scans for and removes stale handshakes and sessions.
    /// Lower values = more frequent cleanup but higher overhead.
    /// Higher values = less overhead but slower memory reclamation.
    #[serde(with = "humantime_serde")]
    pub state_cleanup_interval: Duration,

    /// Maximum concurrent forward connections (default: 1000)
    ///
    /// Limits simultaneous outbound connections when forwarding LP packets to other gateways
    /// during telescope setup. This prevents file descriptor exhaustion under high load.
    ///
    /// When at capacity, new forward requests return an error, signaling the client
    /// to choose a different gateway.
    pub max_concurrent_forwards: usize,
}

impl LpConfig {
    pub const DEFAULT_CONTROL_PORT: u16 = 41264;
    pub const DEFAULT_DATA_PORT: u16 = 51264;

    pub fn announced_control_port(&self) -> u16 {
        self.announce_control_port
            .unwrap_or(self.control_bind_address.port())
    }

    pub fn announced_data_port(&self) -> u16 {
        self.announce_data_port
            .unwrap_or(self.data_bind_address.port())
    }
}

impl Default for LpConfig {
    fn default() -> Self {
        LpConfig {
            control_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_CONTROL_PORT,
            ),
            data_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_DATA_PORT,
            ),
            announce_control_port: None,
            announce_data_port: None,
            debug: Default::default(),
        }
    }
}

impl LpDebug {
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;

    // 30 seconds - balances security vs clock skew tolerance
    pub const DEFAULT_TIMESTAMP_TOLERANCE: Duration = Duration::from_secs(30);

    // 90 seconds - handshakes should complete quickly
    pub const DEFAULT_HANDSHAKE_TTL: Duration = Duration::from_secs(90);

    // 24 hours - for long-lived dVPN sessions
    pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(86400);

    // 1 minute - enough to drain in-flight packets after subsession promotion
    pub const DEFAULT_DEMOTED_SESSION_TTL: Duration = Duration::from_secs(60);

    // 5 minutes - balances memory reclamation with task overhead
    pub const DEFAULT_STATE_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

    // Limits concurrent outbound connections to prevent fd exhaustion
    pub const DEFAULT_MAX_CONCURRENT_FORWARDS: usize = 1000;
}

impl Default for LpDebug {
    fn default() -> Self {
        LpDebug {
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            timestamp_tolerance: Self::DEFAULT_TIMESTAMP_TOLERANCE,
            use_mock_ecash: false,
            handshake_ttl: Self::DEFAULT_HANDSHAKE_TTL,
            session_ttl: Self::DEFAULT_SESSION_TTL,
            demoted_session_ttl: Self::DEFAULT_DEMOTED_SESSION_TTL,
            state_cleanup_interval: Self::DEFAULT_STATE_CLEANUP_INTERVAL,
            max_concurrent_forwards: Self::DEFAULT_MAX_CONCURRENT_FORWARDS,
        }
    }
}

/// Wrapper for state entries with timestamp tracking for cleanup
///
/// This wrapper adds `created_at` and `last_activity` timestamps to state entries,
/// enabling TTL-based cleanup of stale handshakes and sessions.
pub struct TimestampedState<T> {
    /// The actual state (LpStateMachine or LpSession)
    pub state: T,

    /// When this state was created (never changes)
    created_at: std::time::Instant,

    /// Last activity timestamp (unix seconds, atomically updated)
    ///
    /// For handshakes: never updated (use created_at for TTL)
    /// For sessions: updated on every packet received
    last_activity: std::sync::atomic::AtomicU64,
}

impl<T> TimestampedState<T> {
    /// Create a new timestamped state
    pub fn new(state: T) -> Self {
        let now_instant = std::time::Instant::now();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            state,
            created_at: now_instant,
            last_activity: std::sync::atomic::AtomicU64::new(now_unix),
        }
    }

    /// Update last_activity timestamp (cheap, lock-free operation)
    pub fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_activity
            .store(now, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get age since creation
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Get time since last activity
    pub fn since_activity(&self) -> std::time::Duration {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self
            .last_activity
            .load(std::sync::atomic::Ordering::Relaxed);
        Duration::from_secs(now.saturating_sub(last))
    }
}

/// Shared state for LP connection handlers
#[derive(Clone)]
pub struct LpHandlerState {
    /// Ecash verifier for bandwidth credentials
    pub ecash_verifier:
        Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,

    /// Storage backend for persistence
    pub storage: GatewayStorage,

    /// Gateway's identity keypair
    pub local_identity: Arc<ed25519::KeyPair>,

    /// Metrics collection
    pub metrics: NymNodeMetrics,

    /// Active clients tracking
    pub active_clients_store: ActiveClientsStore,

    /// WireGuard peer controller channel (for dVPN registrations)
    pub wg_peer_controller: Option<mpsc::Sender<PeerControlRequest>>,

    /// WireGuard gateway data (contains keypair and config)
    pub wireguard_data: Option<WireguardGatewayData>,

    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,

    /// Channel for forwarding Sphinx packets into the mixnet
    ///
    /// Used by the LP data handler (UDP:51264) to forward decrypted Sphinx packets
    /// from LP clients into the mixnet for routing.
    pub outbound_mix_sender: MixForwardingSender,

    /// In-progress handshakes keyed by session_id
    ///
    /// Session ID is deterministically computed from both parties' X25519 keys immediately
    /// after ClientHello. Used during handshake phase. After handshake completes,
    /// state moves to session_states map.
    ///
    /// Wrapped in TimestampedState for TTL-based cleanup of stale handshakes.
    pub handshake_states: Arc<DashMap<u32, TimestampedState<LpStateMachine>>>,

    /// Established sessions keyed by session_id
    ///
    /// Used after handshake completes (session_id is deterministically computed from
    /// both parties' X25519 keys). Enables stateless transport - each packet lookup
    /// by session_id, decrypt/process, respond.
    ///
    /// Wrapped in TimestampedState for TTL-based cleanup of inactive sessions.
    ///
    /// Sessions are stored as LpStateMachine (not LpSession) to enable
    /// subsession/rekeying support. The state machine handles subsession initiation
    /// (SubsessionKK1/KK2/Ready) during transport phase, allowing long-lived connections
    /// to rekey without re-authentication.
    pub session_states: Arc<DashMap<u32, TimestampedState<LpStateMachine>>>,

    /// Semaphore limiting concurrent forward connections
    ///
    /// Prevents file descriptor exhaustion when forwarding LP packets during
    /// telescope setup. When at capacity, forward requests return an error
    /// so clients can choose a different gateway.
    // Connection limiting (not pooling) chosen for forward requests.
    //
    // Why not connection pooling?
    // 1. Forwarding is one-time per telescope setup (handshake only), not ongoing traffic.
    //    Once telescope is established, data flows directly through the tunnel.
    // 2. Telescope targets are distributed across many different gateways - each client
    //    typically connects to a different exit gateway, so pooled connections would
    //    rarely be reused.
    // 3. Connections already go out of scope after each request-response. FD exhaustion
    //    only happens from concurrent spikes, not accumulation.
    // 4. A pool would accumulate one idle connection per unique destination, most of
    //    which would never be reused before TTL expiration.
    //
    // Why semaphore limiting is better:
    // 1. Directly caps concurrent forward connections regardless of destination.
    // 2. When at capacity, returns "busy" error - client can choose another gateway.
    //    This is better than silently queuing requests behind a pool.
    // 3. Simple implementation: no TTL management, stale connection handling, or cleanup.
    pub forward_semaphore: Arc<Semaphore>,
}

/// LP listener that accepts TCP connections on port 41264
pub struct LpListener {
    /// Shared state for connection handlers
    handler_state: LpHandlerState,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpListener {
    pub fn new(handler_state: LpHandlerState, shutdown: ShutdownTracker) -> Self {
        Self {
            handler_state,
            shutdown,
        }
    }

    fn lp_config(&self) -> LpConfig {
        self.handler_state.lp_config
    }

    pub async fn run(&mut self) -> Result<(), GatewayError> {
        let control_bind_address = self.lp_config().control_bind_address;
        let data_bind_address = self.lp_config().data_bind_address;
        let listener = TcpListener::bind(control_bind_address).await.map_err(|e| {
            error!("Failed to bind LP listener to {control_bind_address}: {e}",);
            GatewayError::ListenerBindFailure {
                address: control_bind_address.to_string(),
                source: Box::new(e),
            }
        })?;

        let shutdown_token = self.shutdown.clone_shutdown_token();

        // Spawn background task for state cleanup
        let _cleanup_handle = self.spawn_state_cleanup_task();

        // Spawn UDP data handler for LP data plane (port 51264)
        let _data_handler_handle = self.spawn_data_handler().await?;

        info!(
            "LP listener started on {control_bind_address} (data handler on: {data_bind_address})",
        );

        loop {
            tokio::select! {
                biased;

                _ = shutdown_token.cancelled() => {
                    trace!("LP listener: received shutdown signal");
                    break;
                }

                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            self.handle_connection(stream, addr);
                        }
                        Err(e) => {
                            warn!("Failed to accept LP connection: {}", e);
                        }
                    }
                }
            }
        }

        info!("LP listener shutdown complete");
        Ok(())
    }

    fn handle_connection(&self, stream: tokio::net::TcpStream, remote_addr: SocketAddr) {
        // Check connection limit
        let active_connections = self.active_lp_connections();
        let max_connections = self.lp_config().debug.max_connections;
        if active_connections >= max_connections {
            warn!(
                "LP connection limit exceeded ({active_connections}/{max_connections}), rejecting connection from {remote_addr}"
            );
            return;
        }

        debug!(
            "Accepting LP connection from {remote_addr} ({active_connections} active connections)"
        );

        // Increment connection counter
        self.handler_state.metrics.network.new_lp_connection();

        // Spawn handler task
        let handler =
            handler::LpConnectionHandler::new(stream, remote_addr, self.handler_state.clone());

        let metrics = self.handler_state.metrics.clone();
        self.shutdown.try_spawn_named(
            async move {
                let result = handler.handle().await;

                // Handler emits lifecycle metrics internally on success
                // For errors, we need to emit them here since handler is consumed
                if let Err(e) = result {
                    warn!("LP handler error for {remote_addr}: {e}");
                    // Note: metrics are emitted in handle() for graceful path
                    // On error path, handle() returns early without emitting
                    // So we track errors here
                }

                // Decrement connection counter on exit
                metrics.network.lp_connection_closed();
            },
            &format!("LP::{remote_addr}"),
        );
    }

    /// Spawn the UDP data handler for LP data plane
    ///
    /// The data handler listens on UDP port 51264 and processes LP-wrapped Sphinx packets
    /// from registered clients. It decrypts the LP layer and forwards the Sphinx packets
    /// into the mixnet.
    async fn spawn_data_handler(&self) -> Result<tokio::task::JoinHandle<()>, GatewayError> {
        // Create data handler
        let data_handler = data_handler::LpDataHandler::new(
            self.lp_config().data_bind_address,
            self.handler_state.clone(),
            self.shutdown.clone_shutdown_token(),
        )
        .await?;

        // Spawn data handler task
        let handle = self.shutdown.try_spawn_named(
            async move {
                if let Err(e) = data_handler.run().await {
                    error!("LP data handler error: {e}");
                }
            },
            "LP::DataHandler",
        );

        Ok(handle)
    }

    /// Spawn background task for cleaning up stale state entries
    ///
    /// This task runs periodically (every `state_cleanup_interval_secs`) to remove:
    /// - Handshake states older than `handshake_ttl_secs`
    /// - Session states with no activity for `session_ttl_secs`
    ///
    /// The task automatically stops when the shutdown signal is received.
    fn spawn_state_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let handshake_states = Arc::clone(&self.handler_state.handshake_states);
        let session_states = Arc::clone(&self.handler_state.session_states);
        let dbg_cfg = self.handler_state.lp_config.debug;

        let handshake_ttl = dbg_cfg.handshake_ttl;
        let session_ttl = dbg_cfg.session_ttl;
        let demoted_session_ttl = dbg_cfg.demoted_session_ttl;
        let interval = dbg_cfg.state_cleanup_interval;
        let shutdown = self.shutdown.clone_shutdown_token();
        let metrics = self.handler_state.metrics.clone();

        info!(
            "Starting LP state cleanup task (handshake_ttl={}s, session_ttl={}s, demoted_ttl={}s, interval={}s)",
            handshake_ttl.as_secs(), session_ttl.as_secs(), demoted_session_ttl.as_secs(), interval.as_secs()
        );

        self.shutdown.try_spawn_named(
            Self::cleanup_loop(
                handshake_states,
                session_states,
                handshake_ttl,
                session_ttl,
                demoted_session_ttl,
                interval,
                shutdown,
                metrics,
            ),
            "LP::StateCleanup",
        )
    }

    /// Background loop for cleaning up stale state entries
    ///
    /// Runs periodically to scan handshake_states and session_states maps,
    /// removing entries that have exceeded their TTL.
    ///
    /// Demoted sessions (ReadOnlyTransport) use shorter TTL since they
    /// only need to drain in-flight packets after subsession promotion.
    #[allow(clippy::too_many_arguments)]
    async fn cleanup_loop(
        handshake_states: Arc<DashMap<u32, TimestampedState<LpStateMachine>>>,
        session_states: Arc<DashMap<u32, TimestampedState<LpStateMachine>>>,
        handshake_ttl: Duration,
        session_ttl: Duration,
        demoted_session_ttl: Duration,
        interval: Duration,
        shutdown: nym_task::ShutdownToken,
        _metrics: NymNodeMetrics,
    ) {
        use nym_lp::state_machine::LpStateBare;
        use nym_metrics::inc_by;

        let mut cleanup_interval = tokio::time::interval(interval);

        loop {
            tokio::select! {
                biased;

                _ = shutdown.cancelled() => {
                    debug!("LP state cleanup task: received shutdown signal");
                    break;
                }

                _ = cleanup_interval.tick() => {
                    let start = std::time::Instant::now();
                    let mut hs_removed = 0u64;
                    let mut ss_removed = 0u64;
                    let mut demoted_removed = 0u64;

                    // Remove stale handshakes (based on age since creation)
                    handshake_states.retain(|_, timestamped| {
                        if timestamped.age() > handshake_ttl {
                            hs_removed += 1;
                            false
                        } else {
                            true
                        }
                    });

                    // Remove stale sessions (based on time since last activity)
                    // Use shorter TTL for demoted (ReadOnlyTransport) sessions
                    session_states.retain(|_, timestamped| {
                        let is_demoted = timestamped.state.bare_state() == LpStateBare::ReadOnlyTransport;
                        let ttl = if is_demoted {
                            demoted_session_ttl
                        } else {
                            session_ttl
                        };

                        if timestamped.since_activity() > ttl {
                            if is_demoted {
                                demoted_removed += 1;
                            } else {
                                ss_removed += 1;
                            }
                            false
                        } else {
                            true
                        }
                    });

                    if hs_removed > 0 || ss_removed > 0 || demoted_removed > 0 {
                        let duration = start.elapsed();
                        info!(
                            "LP state cleanup: removed {} handshakes, {} sessions, {} demoted (took {:.3}s)",
                            hs_removed,
                            ss_removed,
                            demoted_removed,
                            duration.as_secs_f64()
                        );

                        // Track metrics
                        if hs_removed > 0 {
                            inc_by!("lp_states_cleanup_handshake_removed", hs_removed as i64);
                        }
                        if ss_removed > 0 {
                            inc_by!("lp_states_cleanup_session_removed", ss_removed as i64);
                        }
                        if demoted_removed > 0 {
                            inc_by!("lp_states_cleanup_demoted_removed", demoted_removed as i64);
                        }
                    }
                }
            }
        }

        info!("LP state cleanup task shutdown complete");
    }

    fn active_lp_connections(&self) -> usize {
        self.handler_state
            .metrics
            .network
            .active_lp_connections_count()
    }
}
