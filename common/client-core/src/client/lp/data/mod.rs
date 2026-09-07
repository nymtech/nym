// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Parking the branch
#![allow(clippy::todo)]
#![allow(dead_code)]
#![allow(clippy::incompatible_msrv)]

use std::sync::{Arc, mpsc};

use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::lp::data::handler::LpDataHandler;
use crate::client::lp::data::listener::LpDataListener;
use crate::client::lp::data::shared::SharedLpDataState;
use crate::error::ClientCoreError;

use nym_task::ShutdownTracker;
use tracing::error;

/// Maximum UDP packet size we'll accept
/// Sphinx packets are typically ~2KB, LP overhead is ~50 bytes, so 4KB is plenty
const MAX_UDP_PACKET_SIZE: usize = 4096;

pub(crate) const PACKET_BUFFER_SIZE: usize = 100;

pub mod handler;
mod listener;
pub mod shared;

pub struct LpDataSetup {
    listener: LpDataListener,

    handler: LpDataHandler,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpDataSetup {
    pub(crate) fn new(
        shared_state: SharedLpDataState,
        outbound_input_rx: InputMessageReceiver,
        shutdown: ShutdownTracker,
    ) -> Result<Self, ClientCoreError> {
        let (inbound_input_tx, inbound_input_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);
        let (outbound_output_tx, outbound_output_rx) =
            tokio::sync::mpsc::channel(PACKET_BUFFER_SIZE);

        let shared_state = Arc::new(shared_state);

        let listener = LpDataListener::new(
            shared_state.clone(),
            inbound_input_tx,
            outbound_output_rx,
            shutdown.clone_shutdown_token(),
        );

        let handler = LpDataHandler::new(
            shared_state,
            outbound_input_rx,
            outbound_output_tx,
            inbound_input_rx,
            &shutdown,
        )?;

        Ok(LpDataSetup {
            listener,
            handler,
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
