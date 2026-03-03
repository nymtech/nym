// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::lp::data::MAX_UDP_PACKET_SIZE;
use crate::node::lp::data::handler::LpDataHandler;
use crate::node::lp::state::SharedLpDataState;
use nym_metrics::inc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::log::warn;
use tracing::{debug, error, info};

/// LP UDP listener that accepts TCP connections on port 51264 (by default)
pub struct LpDataListener {
    /// Address to bind to
    bind_address: SocketAddr,

    /// State used for handling received requests
    handler: LpDataHandler,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataListener {
    pub fn new(
        bind_address: SocketAddr,
        state: SharedLpDataState,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        Self {
            bind_address,
            handler: LpDataHandler::new(state),
            shutdown,
        }
    }

    pub async fn run(&self) -> Result<(), NymNodeError> {
        let bind_address = self.bind_address;
        info!("Starting LP data listener on {bind_address}");
        let socket = UdpSocket::bind(bind_address).await.map_err(|source| {
            error!("Failed to bind LP data socket to {bind_address}: {source}");
            NymNodeError::LpBindFailure {
                address: bind_address,
                source,
            }
        })?;

        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data listener: received shutdown signal");
                    break;
                }

                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, src_addr)) => {
                            // Process packet in place (no spawn - UDP is fast)
                            if let Err(e) = self.handler.handle_packet(&buf[..len], src_addr).await {
                                debug!("LP data packet error from {src_addr}: {e}");
                                inc!("lp_data_packet_errors");
                            }
                        }
                        Err(e) => {
                            warn!("LP data socket recv error: {e}");
                            inc!("lp_data_recv_errors");
                        }
                    }
                }
            }
        }

        info!("LP data handler shutdown complete");
        Ok(())
    }
}
