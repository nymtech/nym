// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::SharedData;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace};

pub(crate) struct Listener {
    bind_address: SocketAddr,
    shutdown: ShutdownToken,
    shared_data: SharedData,
}

impl Listener {
    pub(crate) fn new(bind_address: SocketAddr, shared_data: SharedData) -> Self {
        Listener {
            bind_address,
            shutdown: shared_data.shutdown.clone_with_suffix("socket-listener"),
            shared_data,
        }
    }

    pub(crate) async fn run(&mut self) {
        info!("attempting to run mixnet listener on {}", self.bind_address);

        let tcp_listener = match tokio::net::TcpListener::bind(self.bind_address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {}: {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.bind_address);
                self.shutdown.cancel();
                return;
            }
        };

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    trace!("mixnet listener: received shutdown");
                    break
                }
                connection = tcp_listener.accept() => {
                    self.shared_data.try_handle_connection(connection);
                }
            }
        }
        debug!("mixnet socket listener: Exiting");
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
