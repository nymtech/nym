// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::SharedData;
use nym_task::TaskClient;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

pub(crate) struct Listener {
    bind_address: SocketAddr,
    shutdown: TaskClient,
    shared_data: SharedData,
}

impl Listener {
    pub(crate) fn new(bind_address: SocketAddr, shared_data: SharedData) -> Self {
        Listener {
            bind_address,
            shutdown: shared_data.task_client.fork("socket-listener"),
            shared_data,
        }
    }

    pub(crate) async fn run(&mut self) {
        info!("attempting to run mixnet listener on {}", self.bind_address);

        let tcp_listener = match tokio::net::TcpListener::bind(self.bind_address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {}: {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.bind_address);

                // that's a bit gnarly, but we need to make sure we trigger shutdown
                let mut shutdown_bomb = self.shutdown.fork("shutdown-bomb");
                shutdown_bomb.rearm();
                drop(shutdown_bomb);
                return;
            }
        };

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    trace!("mixnet listener: received shutdown");
                }
                connection = tcp_listener.accept() => {
                    self.shared_data.try_handle_connection(connection);
                }
            }
        }
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
