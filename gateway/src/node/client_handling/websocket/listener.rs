// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::clients_handler::ClientsHandlerRequestSender;
use crate::node::client_handling::websocket::connection_handler::Handle;
use crypto::asymmetric::identity;
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
    local_identity: Arc<identity::KeyPair>,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr, local_identity: Arc<identity::KeyPair>) -> Self {
        Listener {
            address,
            local_identity,
        }
    }

    pub(crate) async fn run(
        &mut self,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: MixForwardingSender,
    ) {
        info!("Starting websocket listener at {}", self.address);
        let mut tcp_listener = tokio::net::TcpListener::bind(self.address)
            .await
            .expect("Failed to start websocket listener");

        loop {
            match tcp_listener.accept().await {
                Ok((socket, remote_addr)) => {
                    trace!("received a socket connection from {}", remote_addr);
                    // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                    // clients or spawned tokio tasks -> perhaps a worker system?
                    let mut handle = Handle::new(
                        socket,
                        clients_handler_sender.clone(),
                        outbound_mix_sender.clone(),
                        Arc::clone(&self.local_identity),
                    );
                    tokio::spawn(async move { handle.start_handling().await });
                }
                Err(e) => warn!("failed to get client: {:?}", e),
            }
        }
    }

    pub(crate) fn start(
        mut self,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: MixForwardingSender,
    ) -> JoinHandle<()> {
        tokio::spawn(async move { self.run(clients_handler_sender, outbound_mix_sender).await })
    }
}
