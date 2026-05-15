// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::mpsc;

use crate::error::NymNodeError;
use crate::node::lp::data::handler::LpDataHandler;
use crate::node::lp::state::SharedLpDataState;
use crate::{config::LpConfig, node::lp::data::listener::LpDataListener};

use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownTracker;
use tracing::error;

/// Maximum UDP packet size we'll accept
/// Sphinx packets are typically ~2KB, LP overhead is ~50 bytes, so 4KB is plenty
const MAX_UDP_PACKET_SIZE: usize = 4096;

const PACKET_BUFFER_SIZE: usize = 100;

pub mod handler;
mod listener;
//mod sender;

pub struct LpDataSetup {
    listener: LpDataListener,

    handler: LpDataHandler,

    //sender: LpDataSender,
    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpDataSetup {
    pub async fn new(
        lp_config: LpConfig,
        metrics: NymNodeMetrics,
        shutdown: ShutdownTracker,
    ) -> Result<Self, NymNodeError> {
        let data_state = SharedLpDataState { metrics, lp_config };

        let (input_tx, input_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);
        let (output_tx, output_rx) = tokio::sync::mpsc::channel(PACKET_BUFFER_SIZE);

        let listener = LpDataListener::new(
            data_state.clone(),
            input_tx,
            output_rx,
            shutdown.clone_shutdown_token(),
        );

        let handler = LpDataHandler::new(
            data_state.clone(),
            input_rx,
            output_tx,
            shutdown.clone_shutdown_token(),
        );
        //let sender = LpDataSender::new(data_state, output_rx, shutdown.clone_shutdown_token());

        Ok(LpDataSetup {
            listener,
            handler,
            //sender,
            shutdown,
        })
    }

    pub fn start_tasks(mut self) {
        // Spawn the UDP data handler for LP data plane
        // The data handler listens on UDP port 51264 and processes LP-wrapped Sphinx packets
        // from registered clients. It decrypts the LP layer and forwards the Sphinx packets
        let shutdown_token = self.shutdown.clone_shutdown_token();
        let mut listener = self.listener;
        self.shutdown.try_spawn_named(
            async move {
                if let Err(err) = listener.run().await {
                    shutdown_token.cancel();
                    error!("LP data listener error: {err}");
                }
            },
            "LP::LpDataListener",
        );

        self.shutdown
            .try_spawn_named(async move { self.handler.run().await }, "LP::LpDataHandler");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sphinx packets are typically around 2KB
    // 4KB should be plenty with room to spare
    const _: () = {
        assert!(MAX_UDP_PACKET_SIZE >= 2048 + 100);
    };
}
