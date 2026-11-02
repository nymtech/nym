// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::LpConfig;
use crate::error::NymNodeError;
use crate::node::lp::control::ingress::client_handler::LpClientConnectionHandler;
use crate::node::lp::control::ingress::node_handler::InitialLpIngressNodeConnectionHandler;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::state::{SharedGatewayLpControlState, SharedLpControlState};
use nym_metrics::inc;
use nym_task::ShutdownTracker;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info, trace, warn};

/// LP listener that accepts TCP connections on port 41264
pub struct LpControlListener {
    /// Address to bind to
    bind_address: SocketAddr,

    /// Shared state for every connection handler, client and node alike.
    shared_state: SharedLpControlState,

    /// What only a gateway has, held only by a node that forwards client traffic. Its absence is
    /// what refuses a client connection anywhere else.
    gateway_state: Option<SharedGatewayLpControlState>,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpControlListener {
    pub fn new(
        bind_address: SocketAddr,
        shared_state: SharedLpControlState,
        gateway_state: Option<SharedGatewayLpControlState>,
        shutdown: ShutdownTracker,
    ) -> Self {
        Self {
            bind_address,
            shared_state,
            gateway_state,
            shutdown,
        }
    }

    fn lp_config(&self) -> LpConfig {
        self.shared_state.lp_config
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
        let handler = InitialLpIngressNodeConnectionHandler::new(
            stream,
            remote_addr,
            initiator_details,
            self.shared_state.clone(),
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

    fn handle_client_connection(
        &self,
        stream: tokio::net::TcpStream,
        remote_addr: SocketAddr,
        gateway_state: &SharedGatewayLpControlState,
    ) {
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
        let mut handler = LpClientConnectionHandler::new(
            stream,
            remote_addr,
            self.shared_state.clone(),
            gateway_state.clone(),
        );

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
        // Disable Nagle's algorithm on the accepted socket so our responses are flushed
        // immediately rather than coalesced. This is the write side of every reply we send,
        // including handshake replies forwarded back to an entry gateway. Non-fatal: a valid
        // connection should still be served if the option can't be set.
        if let Err(e) = stream.set_nodelay(true) {
            warn!("failed to set TCP_NODELAY on accepted LP connection from {remote_addr}: {e}");
        }

        if let Some(initiator_details) = self.shared_state.nodes.get_node_details(remote_addr.ip())
        {
            self.handle_node_connection(stream, remote_addr, initiator_details);
        } else if let Some(gateway_state) = &self.gateway_state {
            self.handle_client_connection(stream, remote_addr, gateway_state);
        } else {
            // Nothing this node could do for them: it forwards no client traffic, so serving the
            // connection would mean completing a handshake and accepting a registration only to
            // drop everything that followed. Refusing here is what makes that visible to the
            // caller instead of looking like a dead network.
            //
            debug!(
                "refusing LP client connection from {remote_addr}: this node forwards no client traffic"
            );
            inc!("lp_client_connections_refused");
        }
    }

    fn active_client_connections(&self) -> usize {
        self.shared_state
            .metrics
            .network
            .active_lp_client_connections_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::lp::directory::LpNodes;
    use nym_lp::peer::mock_peers;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpStream;

    /// A node that forwards no client traffic has no client state to serve a connection with, so
    /// the caller is dropped at the door rather than taken through a handshake and a registration
    /// whose traffic would then go nowhere.
    #[tokio::test]
    async fn a_node_without_client_state_refuses_a_client_connection() -> anyhow::Result<()> {
        let (_, local) = mock_peers();
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let bind_address = listener.local_addr()?;

        let control = LpControlListener::new(
            bind_address,
            SharedLpControlState {
                local_lp_peer: local,
                // so no source address resolves to a node, and every caller is a client
                nodes: LpNodes::new_empty(),
                sessions: Default::default(),
                metrics: Default::default(),
                lp_config: Default::default(),
            },
            // the point of the test: a mixnode builds none
            None,
            ShutdownTracker::default(),
        );

        let mut client = TcpStream::connect(bind_address).await?;
        let (accepted, remote_addr) = listener.accept().await?;
        control.handle_connection(accepted, remote_addr);

        // nothing is spawned to serve it, so the client reads end-of-stream rather than a reply
        let mut buf = [0u8; 1];
        assert_eq!(client.read(&mut buf).await?, 0);

        Ok(())
    }
}
