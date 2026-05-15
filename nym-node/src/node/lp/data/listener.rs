// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::lp::data::MAX_UDP_PACKET_SIZE;
use crate::node::lp::state::SharedLpDataState;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_metrics::inc;
use std::net::SocketAddr;
use std::sync::mpsc;
use tokio::net::UdpSocket;
use tracing::log::warn;
use tracing::{error, info};

/// LP UDP listener that accepts TCP connections on port 51264 (by default)
pub struct LpDataListener {
    /// Shared data state
    state: SharedLpDataState,

    /// Channel to send incoming data to the processing pipeline
    input_tx: mpsc::SyncSender<EncryptedLpPacket>,

    /// Channel to receive outgoing data from the processling pipeline
    output_rx: tokio::sync::mpsc::Receiver<(Vec<u8>, SocketAddr)>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataListener {
    pub fn new(
        state: SharedLpDataState,
        input_tx: mpsc::SyncSender<EncryptedLpPacket>,
        output_rx: tokio::sync::mpsc::Receiver<(Vec<u8>, SocketAddr)>,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        Self {
            state,
            input_tx,
            output_rx,
            shutdown,
        }
    }

    pub async fn run(&mut self) -> Result<(), NymNodeError> {
        let bind_address = self.state.lp_config.data_bind_address;
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

                result = self.output_rx.recv() => {
                    match result {
                        Some((payload, dst_addr)) => {
                            println!("payload : {payload:?}");
                            if let Err(e) = socket.send_to(&payload, dst_addr).await {
                                warn!("LP data packet error to {dst_addr}: {e}");
                                inc!("lp_data_packet_egress_errors");
                            }
                        }
                        None => {
                            warn!("LP outgoing packet channel closed");
                            break;
                        }
                    }
                }

                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, src_addr)) => {
                            info!("received {len} bytes from {src_addr} on the LP Data endpoint");

                            if let Ok(encrypted_packet) = EncryptedLpPacket::decode(&buf[..len]) {
                                if let Err(e) = self.input_tx.send(encrypted_packet) {
                                    warn!("LP incoming packet channel closed : {e}");
                                    break;
                                }
                            } else {
                                warn!("Error reading LP packet from wire");
                                inc!("lp_data_ingress_processing_errors");
                            }
                        }
                        Err(e) => {
                            warn!("LP data socket recv error: {e}");
                            inc!("lp_data_ingress_errors");
                        }
                    }
                }
            }
        }

        info!("LP data handler shutdown complete");
        Ok(())
    }
}
