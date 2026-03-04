// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::error::NymNodeError;
use crate::node::lp::control::client_handler::LpClientConnectionHandler;
use crate::node::lp::control::node_handler::{
    InitialLpNodeConnectionHandler, LpNodeConnectionHandler,
};
use crate::node::lp::directory::{LpNodeDetails, LpNodes};
use crate::node::lp::state::{SharedLpClientControlState, SharedLpNodeControlState};
use nym_task::ShutdownTracker;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info, trace, warn};

/// LP listener that accepts TCP connections on port 41264
pub struct LpControlListener {
    /// Address to bind to
    bind_address: SocketAddr,

    /// Shared state for clients connection handlers
    clients_handler_state: SharedLpClientControlState,

    /// Shared state for nodes connection handlers
    nodes_handler_state: SharedLpNodeControlState,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpControlListener {
    pub fn new(
        bind_address: SocketAddr,
        handler_state: SharedLpClientControlState,
        shutdown: ShutdownTracker,
    ) -> Self {
        todo!()
        // Self {
        //     bind_address,
        //     handler_state,
        //     shutdown,
        // }
    }

    fn lp_config(&self) -> LpConfig {
        self.clients_handler_state.shared.lp_config
    }

    pub async fn run(&mut self) -> Result<(), NymNodeError> {
        let bind_address = self.bind_address;
        info!("Starting LP control listener on {bind_address}");

        let listener = TcpListener::bind(bind_address).await.map_err(|source| {
            error!("Failed to bind LP listener to {bind_address}: {source}",);
            NymNodeError::LpBindFailure {
                address: bind_address,
                source,
            }
        })?;

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
                        Ok((stream, addr)) => self.handle_connection(stream, addr),
                        Err(e) => warn!("Failed to accept LP connection: {e}")
                    }
                }
            }
        }

        info!("LP listener shutdown complete");
        Ok(())
    }

    fn handle_node_connection(
        &self,
        stream: tokio::net::TcpStream,
        remote_addr: SocketAddr,
        initiator_details: LpNodeDetails,
    ) {
        debug!("Accepting LP node connection from {remote_addr}");

        // Spawn handler task
        let mut handler = InitialLpNodeConnectionHandler::new(
            stream,
            remote_addr,
            initiator_details,
            self.nodes_handler_state.clone(),
        );

        self.shutdown.try_spawn_named_with_shutdown(
            async move {
                let metrics = handler.metrics().clone();

                // Increment connection counter
                metrics.network.new_ingress_lp_node_connection();

                let result = handler.handle().await;

                // Decrement connection counter
                metrics.network.closed_ingress_lp_node_connection();

                // Handler emits lifecycle metrics internally on success
                // For errors, we need to emit them here since handler is consumed
                if let Err(e) = result {
                    warn!("LP node handler error for {remote_addr}: {e}");
                    // Note: metrics are emitted in handle() for graceful path
                    // On error path, handle() returns early without emitting
                    // So we track errors here
                }
            },
            &format!("LP_NODE::{remote_addr}"),
        );
    }

    fn handle_client_connection(&self, stream: tokio::net::TcpStream, remote_addr: SocketAddr) {
        // Check connection limit (only for clients, nodes must always be allowed regardless of the limit)
        let active_connections = self.active_client_connections();
        let max_connections = self.lp_config().debug.max_connections;
        if active_connections >= max_connections {
            warn!(
                "LP connection limit exceeded ({active_connections}/{max_connections}), rejecting connection from {remote_addr}"
            );
            return;
        }

        debug!(
            "Accepting LP client connection from {remote_addr} ({active_connections} active connections)"
        );

        // Spawn handler task
        let mut handler =
            LpClientConnectionHandler::new(stream, remote_addr, self.clients_handler_state.clone());

        self.shutdown.try_spawn_named_with_shutdown(
            async move {
                // Increment connection counter
                handler.metrics().network.new_ingress_lp_client_connection();

                let result = handler.handle().await;
                // Decrement connection counter
                handler
                    .metrics()
                    .network
                    .closed_ingress_lp_client_connection();

                // Handler emits lifecycle metrics internally on success
                // For errors, we need to emit them here since handler is consumed
                if let Err(e) = result {
                    warn!("LP client handler error for {remote_addr}: {e}");
                    // Note: metrics are emitted in handle() for graceful path
                    // On error path, handle() returns early without emitting
                    // So we track errors here
                }
            },
            &format!("LP_CLIENT::{remote_addr}"),
        );
    }

    fn handle_connection(&self, stream: tokio::net::TcpStream, remote_addr: SocketAddr) {
        if let Some(initiator_details) = self
            .nodes_handler_state
            .nodes
            .get_node_details(remote_addr.ip())
        {
            self.handle_node_connection(stream, remote_addr, initiator_details);
        } else {
            self.handle_client_connection(stream, remote_addr);
        }
    }

    fn active_client_connections(&self) -> usize {
        self.clients_handler_state
            .shared
            .metrics
            .network
            .active_lp_client_connections_count()
    }
}
