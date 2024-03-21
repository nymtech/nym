// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use crate::node::storage::Storage;
use log::*;
use nym_mixnet_client::forwarder::MixForwardingSender;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
    shared_state: CommonHandlerState,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr, shared_state: CommonHandlerState) -> Self {
        Listener {
            address,
            shared_state,
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    pub(crate) async fn run<St>(
        &mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: St,
        active_clients_store: ActiveClientsStore,
        mut shutdown: nym_task::TaskClient,
    ) where
        St: Storage + Clone + 'static,
    {
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
                            trace!("received a socket connection from {remote_addr}");
                            // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                            // clients or spawned tokio tasks -> perhaps a worker system?
                            let handle = FreshHandler::new(
                                OsRng,
                                socket,
                                outbound_mix_sender.clone(),
                                storage.clone(),
                                active_clients_store.clone(),
                                self.shared_state.clone(),
                            );
                            let shutdown = shutdown.clone().named(format!("ClientConnectionHandler_{remote_addr}"));
                            tokio::spawn(async move { handle.start_handling(shutdown).await });
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }

            }
        }
    }

    pub(crate) fn start<St>(
        mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: St,
        active_clients_store: ActiveClientsStore,
        shutdown: nym_task::TaskClient,
    ) -> JoinHandle<()>
    where
        St: Storage + Clone + 'static,
    {
        tokio::spawn(async move {
            self.run(outbound_mix_sender, storage, active_clients_store, shutdown)
                .await
        })
    }
}
