// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::LpConfig;
use crate::error::NymNodeError;
use crate::node::lp::control::handler::LpConnectionHandler;
use crate::node::lp::state::SharedLpControlState;
use nym_task::ShutdownTracker;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info, trace, warn};

/// LP listener that accepts TCP connections on port 41264
pub struct LpControlListener {
    /// Address to bind to
    bind_address: SocketAddr,

    /// Shared state for connection handlers
    handler_state: SharedLpControlState,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpControlListener {
    pub fn new(
        bind_address: SocketAddr,
        handler_state: SharedLpControlState,
        shutdown: ShutdownTracker,
    ) -> Self {
        Self {
            bind_address,
            handler_state,
            shutdown,
        }
    }

    fn lp_config(&self) -> LpConfig {
        self.handler_state.shared.lp_config
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
        self.handler_state
            .shared
            .metrics
            .network
            .new_lp_connection();

        // Spawn handler task
        let handler = LpConnectionHandler::new(stream, remote_addr, self.handler_state.clone());

        let metrics = self.handler_state.shared.metrics.clone();
        self.shutdown.try_spawn_named_with_shutdown(
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

    fn active_lp_connections(&self) -> usize {
        self.handler_state
            .shared
            .metrics
            .network
            .active_lp_connections_count()
    }
}
