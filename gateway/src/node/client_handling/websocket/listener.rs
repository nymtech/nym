// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use crate::node::storage::PersistentStorage;
use crypto::asymmetric::identity;
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[cfg(feature = "coconut")]
use coconut_interface::VerificationKey;

#[cfg(not(feature = "coconut"))]
use crate::node::client_handling::websocket::connection_handler::eth_events::EthEvents;

pub(crate) struct Listener {
    address: SocketAddr,
    local_identity: Arc<identity::KeyPair>,

    #[cfg(feature = "coconut")]
    aggregated_verification_key: VerificationKey,

    #[cfg(not(feature = "coconut"))]
    eth_events: EthEvents,
}

impl Listener {
    pub(crate) fn new(
        address: SocketAddr,
        local_identity: Arc<identity::KeyPair>,
        #[cfg(feature = "coconut")] aggregated_verification_key: VerificationKey,
        #[cfg(not(feature = "coconut"))] eth_events: EthEvents,
    ) -> Self {
        Listener {
            address,
            local_identity,
            #[cfg(feature = "coconut")]
            aggregated_verification_key,
            #[cfg(not(feature = "coconut"))]
            eth_events,
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    pub(crate) async fn run(
        &mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: PersistentStorage,
        active_clients_store: ActiveClientsStore,
    ) {
        info!("Starting websocket listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind the websocket to {} - {}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address, err);
                process::exit(1);
            }
        };

        loop {
            match tcp_listener.accept().await {
                Ok((socket, remote_addr)) => {
                    trace!("received a socket connection from {}", remote_addr);
                    // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                    // clients or spawned tokio tasks -> perhaps a worker system?
                    let handle = FreshHandler::new(
                        OsRng,
                        socket,
                        outbound_mix_sender.clone(),
                        Arc::clone(&self.local_identity),
                        storage.clone(),
                        active_clients_store.clone(),
                        #[cfg(feature = "coconut")]
                        self.aggregated_verification_key.clone(),
                        #[cfg(not(feature = "coconut"))]
                        self.eth_events.clone(),
                    );
                    tokio::spawn(async move { handle.start_handling().await });
                }
                Err(e) => warn!("failed to get client: {:?}", e),
            }
        }
    }

    pub(crate) fn start(
        mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: PersistentStorage,
        active_clients_store: ActiveClientsStore,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run(outbound_mix_sender, storage, active_clients_store)
                .await
        })
    }
}
