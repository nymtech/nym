// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::SharedData;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use tracing::{Instrument, debug, error, info, instrument, trace};

pub(crate) struct Listener {
    bind_address: SocketAddr,
    shared_data: SharedData,
}

impl Listener {
    pub(crate) fn new(bind_address: SocketAddr, shared_data: SharedData) -> Self {
        Listener {
            bind_address,
            shared_data,
        }
    }
    #[instrument(skip_all, level = "debug")]
    pub(crate) async fn run(&mut self, shutdown: ShutdownToken) {
        info!("attempting to run mixnet listener on {}", self.bind_address);

        let tcp_listener = match tokio::net::TcpListener::bind(self.bind_address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!(
                    "Failed to bind to {}: {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?",
                    self.bind_address
                );
                shutdown.cancel();
                return;
            }
        };

        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => {
                    trace!("mixnet listener: received shutdown");
                    break
                }
                connection = tcp_listener.accept().in_current_span() => {
                    self.shared_data.try_handle_connection(connection);
                }
            }
        }
        debug!("mixnet socket listener: Exiting");
    }
}
