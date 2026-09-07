// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ClientCoreError;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_lp_gateway_client::LpGatewayClient;
use std::net::SocketAddr;
use std::sync::{mpsc, mpsc::TrySendError};
use tracing::info;
use tracing::log::warn;

/// All of the LP data plane's socket I/O, and nothing else.
///
/// Keeping the writes here rather than in the handler is what lets the handler's release-time drain
/// stay non-blocking: it ticks every millisecond and hands packets over a channel instead of
/// waiting on the network.
pub(crate) struct LpDataListener {
    /// Owns the data socket, and knows how to send an already-encrypted packet somewhere.
    gateway_client: LpGatewayClient,

    /// Channel to send incoming data to the processing pipeline
    inbound_input_tx: mpsc::SyncSender<EncryptedLpPacket>,

    // This has to be a tokio channel, to be async and bounded
    /// Channel to receive outgoing data from the processing pipeline
    outbound_output_rx: tokio::sync::mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataListener {
    pub fn new(
        gateway_client: LpGatewayClient,
        inbound_input_tx: mpsc::SyncSender<EncryptedLpPacket>,
        outbound_output_rx: tokio::sync::mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        Self {
            gateway_client,
            inbound_input_tx,
            outbound_output_rx,
            shutdown,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientCoreError> {
        info!("Started the LP data listener");

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data listener: received shutdown signal");
                    break;
                }

                result = self.outbound_output_rx.recv() => {
                    match result {
                        Some((packet, dst_addr)) => {
                            if let Err(e) = self.gateway_client.send(&packet, dst_addr).await {
                                warn!("LP data packet error to {dst_addr}: {e}");
                            }
                        }
                        None => {
                            warn!("LP outgoing packet channel closed");
                            break;
                        }
                    }
                }

                result = self.gateway_client.recv() => {
                    match result {
                        Ok((packet, src_addr)) => {
                            info!("received a packet from {src_addr} on the LP data socket");
                            if let Err(e) = self.inbound_input_tx.try_send(packet) {
                                match e {
                                    TrySendError::Full(_) => {
                                        warn!("LP data listener: packet sending buffer is full, the client might be overloaded");
                                    },
                                    TrySendError::Disconnected(_) => {
                                        warn!("LP data listener: incoming packet channel is closed");
                                        break;
                                    },
                                }
                            }
                        }
                        Err(e) => {
                            warn!("LP data socket recv error: {e}");
                        }
                    }
                }
            }
        }

        info!("LP data listener shutdown complete");
        Ok(())
    }
}
