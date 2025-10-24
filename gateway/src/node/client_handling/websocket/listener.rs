// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use nym_task::ShutdownTracker;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::{io, process};
use tokio::net::TcpStream;
use tracing::*;

pub struct Listener {
    address: SocketAddr,
    maximum_open_connections: usize,
    shared_state: CommonHandlerState,
    shutdown: ShutdownTracker,
}

impl Listener {
    pub(crate) fn new(
        address: SocketAddr,
        maximum_open_connections: usize,
        shared_state: CommonHandlerState,
        shutdown: ShutdownTracker,
    ) -> Self {
        Listener {
            address,
            maximum_open_connections,
            shared_state,
            shutdown,
        }
    }

    fn active_connections(&self) -> usize {
        self.shared_state
            .metrics
            .network
            .active_ingress_websocket_connections_count()
    }

    fn prepare_connection_handler(
        &self,
        socket: TcpStream,
        remote_address: SocketAddr,
    ) -> FreshHandler<OsRng, TcpStream> {
        FreshHandler::new(
            OsRng,
            socket,
            self.shared_state.clone(),
            remote_address,
            self.shutdown.clone_shutdown_token(),
        )
    }

    #[instrument(skip_all)]
    fn try_handle_accepted_connection(&self, accepted: io::Result<(TcpStream, SocketAddr)>) {
        match accepted {
            Ok((socket, remote_address)) => {
                trace!("received a socket connection from {remote_address}");

                let active = self.active_connections();

                // 1. check if we're within the connection limit
                if active >= self.maximum_open_connections {
                    warn!(
                        "connection limit exceeded ({}). can't accept request from {remote_address}",
                        self.maximum_open_connections
                    );
                    return;
                }

                debug!("there are currently {active} connected clients on the gateway websocket");

                // 2. prepare shared data for the new connection handler
                let handle = self.prepare_connection_handler(socket, remote_address);

                // 3. increment the connection counter.
                // make sure to do it before spawning the task,
                // as another connection might get accepted before the task is scheduled
                // for execution
                self.shared_state
                    .metrics
                    .network
                    .new_ingress_websocket_client();

                // 4. spawn the task handling the client connection
                self.shutdown.try_spawn_named(
                    async move {
                        // TODO: refactor it similarly to the mixnet listener on the nym-node
                        let metrics_ref = handle.shared_state.metrics.clone();

                        // 4.1. handle all client requests until connection gets terminated
                        handle.start_handling().in_current_span().await;

                        // 4.2. decrement the connection counter
                        metrics_ref.network.disconnected_ingress_websocket_client();
                    },
                    &format!("Websocket::{remote_address}"),
                );
            }
            Err(err) => warn!("failed to accept client connection: {err}"),
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    #[instrument(skip_all)]
    pub async fn run(&mut self) {
        info!("Starting websocket listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind the websocket to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        let shutdown_token = self.shutdown.clone_shutdown_token();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("client_handling::Listener: received shutdown");
                    break
                }
                connection = tcp_listener.accept().in_current_span() => {
                    self.try_handle_accepted_connection(connection)
                }
            }
        }
    }
}
