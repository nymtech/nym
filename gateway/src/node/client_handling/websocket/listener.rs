// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use nym_task::TaskClient;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;
use tracing::*;

pub struct Listener {
    address: SocketAddr,
    shared_state: CommonHandlerState,
    shutdown: TaskClient,
}

impl Listener {
    pub(crate) fn new(
        address: SocketAddr,
        shared_state: CommonHandlerState,
        shutdown: TaskClient,
    ) -> Self {
        Listener {
            address,
            shared_state,
            shutdown,
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    pub(crate) async fn run(&mut self) {
        info!("Starting websocket listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind the websocket to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    trace!("client_handling::Listener: received shutdown");
                }
                connection = tcp_listener.accept() => {
                    match connection {
                        Ok((socket, remote_addr)) => {
                            let shutdown = self.shutdown.fork(format!("websocket_handler_{remote_addr}"));
                            trace!("received a socket connection from {remote_addr}");
                            // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                            // clients or spawned tokio tasks -> perhaps a worker system?
                            let handle = FreshHandler::new(
                                OsRng,
                                socket,
                                self.shared_state.clone(),
                                remote_addr,
                                shutdown,
                            );
                            tokio::spawn(async move {
                                // TODO: refactor it similarly to the mixnet listener on the nym-node
                                let metrics_ref = handle.shared_state.metrics.clone();
                                metrics_ref.network.new_ingress_websocket_client();
                                handle.start_handling().await;
                                metrics_ref.network.disconnected_ingress_websocket_client();
                            });
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }

            }
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
