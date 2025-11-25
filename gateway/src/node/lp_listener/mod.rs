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
// ## Usage Example
// To view metrics, the nym-metrics registry automatically collects all metrics.
// They can be exported via Prometheus format using the metrics endpoint.

use crate::error::GatewayError;
use crate::node::ActiveClientsStore;
use dashmap::DashMap;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_storage::GatewayStorage;
use nym_lp::state_machine::LpStateMachine;
use nym_lp::LpSession;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownTracker;
use nym_wireguard::{PeerControlRequest, WireguardGatewayData};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::*;

mod handler;
mod handshake;
mod messages;
mod registration;

/// Configuration for LP listener
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpConfig {
    /// Enable/disable LP listener
    pub enabled: bool,

    /// Bind address for control port
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Control port (default: 41264)
    #[serde(default = "default_control_port")]
    pub control_port: u16,

    /// Data port (default: 51264)
    #[serde(default = "default_data_port")]
    pub data_port: u16,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum acceptable age of ClientHello timestamp in seconds (default: 30)
    ///
    /// ClientHello messages with timestamps older than this will be rejected
    /// to prevent replay attacks. Value should be:
    /// - Large enough to account for clock skew and network latency
    /// - Small enough to limit replay attack window
    ///
    /// Recommended: 30-60 seconds
    #[serde(default = "default_timestamp_tolerance_secs")]
    pub timestamp_tolerance_secs: u64,

    /// Use mock ecash manager for testing (default: false)
    ///
    /// When enabled, the LP listener will use a mock ecash verifier that
    /// accepts any credential without blockchain verification. This is
    /// useful for testing the LP protocol implementation without requiring
    /// a full blockchain/contract setup.
    ///
    /// WARNING: Only use this for local testing! Never enable in production.
    #[serde(default = "default_use_mock_ecash")]
    pub use_mock_ecash: bool,
}

impl Default for LpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: default_bind_address(),
            control_port: default_control_port(),
            data_port: default_data_port(),
            max_connections: default_max_connections(),
            timestamp_tolerance_secs: default_timestamp_tolerance_secs(),
            use_mock_ecash: default_use_mock_ecash(),
        }
    }
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_control_port() -> u16 {
    41264
}

fn default_data_port() -> u16 {
    51264
}

fn default_max_connections() -> usize {
    10000
}

fn default_timestamp_tolerance_secs() -> u64 {
    30 // 30 seconds - balances security vs clock skew tolerance
}

fn default_use_mock_ecash() -> bool {
    false // Always default to real ecash for security
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

    /// In-progress handshakes keyed by session_id
    ///
    /// Session ID is deterministically computed from both parties' X25519 keys immediately
    /// after ClientHello. Used during handshake phase. After handshake completes,
    /// state moves to session_states map.
    pub handshake_states: Arc<DashMap<u32, LpStateMachine>>,

    /// Established sessions keyed by session_id
    ///
    /// Used after handshake completes (session_id is deterministically computed from
    /// both parties' X25519 keys). Enables stateless transport - each packet lookup
    /// by session_id, decrypt/process, respond.
    pub session_states: Arc<DashMap<u32, LpSession>>,
}

/// LP listener that accepts TCP connections on port 41264
pub struct LpListener {
    /// Address to bind the LP control port (41264)
    control_address: SocketAddr,

    /// Port for data plane (51264) - reserved for future use
    data_port: u16,

    /// Shared state for connection handlers
    handler_state: LpHandlerState,

    /// Maximum concurrent connections
    max_connections: usize,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpListener {
    pub fn new(
        bind_address: SocketAddr,
        data_port: u16,
        handler_state: LpHandlerState,
        max_connections: usize,
        shutdown: ShutdownTracker,
    ) -> Self {
        Self {
            control_address: bind_address,
            data_port,
            handler_state,
            max_connections,
            shutdown,
        }
    }

    pub async fn run(&mut self) -> Result<(), GatewayError> {
        let listener = TcpListener::bind(self.control_address).await.map_err(|e| {
            error!(
                "Failed to bind LP listener to {}: {}",
                self.control_address, e
            );
            GatewayError::ListenerBindFailure {
                address: self.control_address.to_string(),
                source: Box::new(e),
            }
        })?;

        info!(
            "LP listener started on {} (data port reserved: {})",
            self.control_address, self.data_port
        );

        let shutdown_token = self.shutdown.clone_shutdown_token();

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
        if active_connections >= self.max_connections {
            warn!(
                "LP connection limit exceeded ({}/{}), rejecting connection from {}",
                active_connections, self.max_connections, remote_addr
            );
            return;
        }

        debug!(
            "Accepting LP connection from {} ({} active connections)",
            remote_addr, active_connections
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
                    warn!("LP handler error for {}: {}", remote_addr, e);
                    // Note: metrics are emitted in handle() for graceful path
                    // On error path, handle() returns early without emitting
                    // So we track errors here
                }

                // Decrement connection counter on exit
                metrics.network.lp_connection_closed();
            },
            &format!("LP::{}", remote_addr),
        );
    }

    fn active_lp_connections(&self) -> usize {
        self.handler_state
            .metrics
            .network
            .active_lp_connections_count()
    }
}
