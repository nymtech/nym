// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::listener::connection_handler::ConnectionHandler;
use log::error;
use std::net::SocketAddr;
use std::process;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::ShutdownListener;

pub(crate) mod connection_handler;

pub(crate) struct Listener {
    address: SocketAddr,
    shutdown: ShutdownListener,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr, shutdown: ShutdownListener) -> Self {
        Listener { address, shutdown }
    }

    async fn run(&mut self, connection_handler: ConnectionHandler) {
        log::trace!("Starting Listener");
        let listener = match TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address, err);
                process::exit(1);
            }
        };

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                connection = listener.accept() => {
                    match connection {
                        Ok((socket, remote_addr)) => {
                            let handler = connection_handler.clone();
                            tokio::spawn(handler.handle_connection(socket, remote_addr));
                        }
                        Err(err) => warn!("Failed to accept incoming connection - {:?}", err),
                    }
                },
                _ = self.shutdown.recv() => {
                    log::trace!("Listener: Received shutdown");
                }
            };
        }
        log::trace!("Listener: Exiting");
    }

    pub(crate) fn start(mut self, connection_handler: ConnectionHandler) -> JoinHandle<()> {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
