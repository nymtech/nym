// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::GatewayError;
use crate::node::ActiveClientsStore;
use nym_credential_verification::ecash::EcashManager;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_storage::GatewayStorage;
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
}

impl Default for LpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: default_bind_address(),
            control_port: default_control_port(),
            data_port: default_data_port(),
            max_connections: default_max_connections(),
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

/// Shared state for LP connection handlers
#[derive(Clone)]
pub struct LpHandlerState {
    /// Ecash verifier for bandwidth credentials
    pub ecash_verifier: Arc<EcashManager>,

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
        let listener = TcpListener::bind(self.control_address)
            .await
            .map_err(|e| {
                error!("Failed to bind LP listener to {}: {}", self.control_address, e);
                GatewayError::ListenerBindFailure {
                    address: self.control_address.to_string(),
                    source: Box::new(e),
                }
            })?;

        info!("LP listener started on {} (data port reserved: {})",
              self.control_address, self.data_port);

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

        debug!("Accepting LP connection from {} ({} active connections)",
               remote_addr, active_connections);

        // Increment connection counter
        self.handler_state.metrics.network.new_lp_connection();

        // Spawn handler task
        let handler = handler::LpConnectionHandler::new(
            stream,
            remote_addr,
            self.handler_state.clone(),
        );

        let metrics = self.handler_state.metrics.clone();
        self.shutdown.try_spawn_named(
            async move {
                if let Err(e) = handler.handle().await {
                    warn!("LP handler error for {}: {}", remote_addr, e);
                }
                // Decrement connection counter on exit
                metrics.network.lp_connection_closed();
            },
            &format!("LP::{}", remote_addr),
        );
    }

    fn active_lp_connections(&self) -> usize {
        self.handler_state.metrics.network.active_lp_connections_count()
    }
}