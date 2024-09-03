// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use nym_gateway_storage::Storage;
use nym_mixnet_client::forwarder::MixForwardingSender;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;
use tracing::*;

pub(crate) struct Listener<S> {
    address: SocketAddr,
    shared_state: CommonHandlerState<S>,
}

impl<S> Listener<S>
where
    S: Storage + Send + Sync + Clone + 'static,
{
    pub(crate) fn new(address: SocketAddr, shared_state: CommonHandlerState<S>) -> Self {
        Listener {
            address,
            shared_state,
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    pub(crate) async fn run(
        &mut self,
        outbound_mix_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        mut shutdown: nym_task::TaskClient,
    ) {
        info!("Starting websocket listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind the websocket to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("client_handling::Listener: received shutdown");
                }
                connection = tcp_listener.accept() => {
                    match connection {
                        Ok((socket, remote_addr)) => {
                            let shutdown = shutdown.clone().named(format!("ClientConnectionHandler_{remote_addr}"));
                            trace!("received a socket connection from {remote_addr}");
                            // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                            // clients or spawned tokio tasks -> perhaps a worker system?
                            let handle = FreshHandler::new(
                                OsRng,
                                socket,
                                outbound_mix_sender.clone(),
                                active_clients_store.clone(),
                                self.shared_state.clone(),
                                remote_addr,
                                shutdown,
                            );
                            tokio::spawn(handle.start_handling());
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }

            }
        }
    }

    pub(crate) fn start(
        mut self,
        outbound_mix_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: nym_task::TaskClient,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run(outbound_mix_sender, active_clients_store, shutdown)
                .await
        })
    }
}
