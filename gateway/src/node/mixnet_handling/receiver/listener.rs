// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::Storage;
use nym_task::TaskClient;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;
use tracing::*;

pub(crate) struct Listener {
    address: SocketAddr,
    shutdown: TaskClient,
}

// TODO: this file is nearly identical to the one in mixnode
impl Listener {
    pub(crate) fn new(address: SocketAddr, shutdown: TaskClient) -> Self {
        Listener { address, shutdown }
    }

    pub(crate) async fn run<St>(&mut self, connection_handler: ConnectionHandler<St>)
    where
        St: Storage + Clone + 'static,
    {
        info!("Starting mixnet listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    log::trace!("mixnet_handling::Listener: Received shutdown");
                }
                connection = tcp_listener.accept() => {
                    match connection {
                        Ok((socket, remote_addr)) => {
                            let handler = connection_handler.clone();
                            tokio::spawn(handler.handle_connection(socket, remote_addr, self.shutdown.clone().named(format!("MixnetConnectionHandler_{remote_addr}"))));
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }
            }
        }
    }

    pub(crate) fn start<St>(mut self, connection_handler: ConnectionHandler<St>) -> JoinHandle<()>
    where
        St: Storage + Clone + 'static,
    {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
