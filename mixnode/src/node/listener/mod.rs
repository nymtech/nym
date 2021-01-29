// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::listener::connection_handler::ConnectionHandler;
use log::*;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub(crate) mod connection_handler;

pub(crate) struct Listener {
    address: SocketAddr,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Listener { address }
    }

    async fn run(&mut self, connection_handler: ConnectionHandler) {
        let mut listener = TcpListener::bind(self.address)
            .await
            .expect("Failed to create TCP listener");
        loop {
            match listener.accept().await {
                Ok((socket, remote_addr)) => {
                    let handler = connection_handler.clone_without_cache();
                    tokio::spawn(handler.handle_connection(socket, remote_addr));
                }
                Err(err) => warn!("Failed to accept incoming connection - {:?}", err),
            }
        }
    }

    pub(crate) fn start(mut self, connection_handler: ConnectionHandler) -> JoinHandle<()> {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
