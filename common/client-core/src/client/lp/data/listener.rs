// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::lp::data::MAX_UDP_PACKET_SIZE;
use crate::client::lp::data::shared::SharedLpDataState;
use crate::error::ClientCoreError;
use nym_lp_data::packet::EncryptedLpPacket;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc, mpsc::TrySendError};
use tokio::net::UdpSocket;
use tracing::log::warn;
use tracing::{error, info};

/// LP UDP listener that accepts TCP connections on port 51264 (by default)
pub(crate) struct LpDataListener {
    /// Shared state
    shared_state: Arc<SharedLpDataState>,

    /// Channel to send incoming data to the processing pipeline
    inbound_input_tx: mpsc::SyncSender<EncryptedLpPacket>,

    // This has to be a tokio channel, to be async and bounded
    /// Channel to receive outgoing data from the processling pipeline
    outbound_output_rx: tokio::sync::mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataListener {
    pub fn new(
        shared_state: Arc<SharedLpDataState>,
        inbound_input_tx: mpsc::SyncSender<EncryptedLpPacket>,
        outbound_output_rx: tokio::sync::mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        Self {
            shared_state,
            inbound_input_tx,
            outbound_output_rx,
            shutdown,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientCoreError> {
        let socket = UdpSocket::bind("[::]:0").await.map_err(|source| {
            error!("Failed to bind LP data socket: {source}");
            ClientCoreError::LpBindFailure { source }
        })?;
        info!("Started LP data socket on {}", socket.local_addr()?);

        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data listener: received shutdown signal");
                    break;
                }

                result = self.outbound_output_rx.recv() => {
                    match result {
                        Some((payload, dst_addr)) => {
                            if let Err(e) = socket.send_to(&payload.to_bytes(), dst_addr).await {
                                warn!("LP data packet error to {dst_addr}: {e}");
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
                            info!("received {len} bytes from {src_addr} on the LP Data socket");
                            if let Ok(encrypted_packet) = EncryptedLpPacket::decode(&buf[..len]) {
                                if let Err(e) = self.inbound_input_tx.try_send(encrypted_packet) {
                                    match e {
                                       TrySendError::Full(_) =>  {
                                            warn!("LP data listener: packet sending buffer is full, the client might be overloaded");
                                        },
                                        TrySendError::Disconnected(_) => {
                                            warn!("LP data listener: incoming packet channel is closed");
                                            break;
                                        },
                                    }
                                }
                            } else {
                                warn!("Error reading LP packet from wire");
                            }
                        }
                        Err(e) => {
                            warn!("LP data socket recv error: {e}");
                        }
                    }
                }
            }
        }

        info!("LP data handler shutdown complete");
        Ok(())
    }
}
