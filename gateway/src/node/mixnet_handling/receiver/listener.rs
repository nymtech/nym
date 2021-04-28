// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use log::*;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
}

// TODO: this file is nearly identical to the one in mixnode
impl Listener {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Listener { address }
    }

    pub(crate) async fn run(&mut self, connection_handler: ConnectionHandler) {
        info!("Starting mixnet listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address, err);
                process::exit(1);
            }
        };

        loop {
            match tcp_listener.accept().await {
                Ok((socket, remote_addr)) => {
                    let handler = connection_handler.clone_without_cache();
                    tokio::spawn(handler.handle_connection(socket, remote_addr));
                }
                Err(e) => warn!("failed to get client: {:?}", e),
            }
        }
    }

    pub(crate) fn start(mut self, connection_handler: ConnectionHandler) -> JoinHandle<()> {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
