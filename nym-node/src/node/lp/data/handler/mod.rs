// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP Data Handler - UDP listener for LP data plane (port 51264)
//!
//! This module handles the data plane for LP clients that have completed registration
//! via the control plane (TCP:41264). LP-wrapped Sphinx packets arrive here, get
//! decrypted, and are forwarded into the mixnet.
//!
//! # Packet Flow
//!
//! ```text
//! LP Client → UDP:51264 → LP Data Handler → Mixnet Entry
//!           LP(Sphinx)      decrypt LP      forward Sphinx
//! ```
//!

use crate::node::lp::data::handler::pipeline::MixnodeDataPipeline;
use crate::node::lp::data::shared::SharedLpDataState;
use nym_lp_data::AddressedTimedData;
use nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_metrics::inc;
use rand::rngs::OsRng;
use std::sync::{Arc, mpsc};
use std::time::Instant;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::interval;
use tracing::*;

pub mod error;
pub(crate) mod messages;
mod pipeline;
mod processing;

const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// LP Data Handler for UDP data plane, act as a pipeline driver and buffer for delaying packet
pub struct LpDataHandler {
    /// Shared state
    shared_state: Arc<SharedLpDataState>,

    /// Channel to receive incoming data
    input_rx: mpsc::Receiver<EncryptedLpPacket>,

    /// Channel to send outgoing data
    output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,

    pipeline: MixnodeDataPipeline<OsRng>,
    outgoing_pkt_buffer: Vec<AddressedTimedData<Instant, EncryptedLpPacket, SocketAddr>>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    /// Create a new LP data handler
    pub(crate) fn new(
        state: SharedLpDataState,
        input_rx: mpsc::Receiver<EncryptedLpPacket>,
        output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        shutdown: nym_task::ShutdownToken,
    ) -> Self {
        let shared_state = Arc::new(state);
        Self {
            shared_state: shared_state.clone(),
            input_rx,
            output_tx,
            pipeline: MixnodeDataPipeline::new(shared_state, OsRng),
            outgoing_pkt_buffer: Vec::new(),
            shutdown,
        }
    }

    pub async fn run(&mut self) {
        info!("Starting LP data handler");
        let mut ticking_interval = interval(PIPELINE_TICKING_DURATION);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data handler: received shutdown signal");
                    break;
                }

                timestamp = ticking_interval.tick() => {
                    let std_timestamp = timestamp.into();
                    while let Ok(input) = self.input_rx.try_recv() {
                        match self.pipeline.process(input, std_timestamp) { // SW need to spawn that in a new thread later
                            Ok(packets) => self.outgoing_pkt_buffer.extend(packets),
                            Err(e) => {
                                warn!("LP data handler: Error processing packet : {e}");
                                inc!("lp_data_packet_errors");
                                self.shared_state.malformed_packet();
                            }

                        }

                    }
                    for pkt in self.outgoing_pkt_buffer.extract_if(.., |p| p.data.timestamp <= std_timestamp) {
                        if let Err(e) = self.output_tx.try_send((pkt.data.data, pkt.dst)) {
                            match e {
                                TrySendError::Full(_) =>  {
                                    warn!("LP data handler: packet sending buffer is full");
                                },
                                TrySendError::Closed(_) => {
                                    info!("LP data handler: outgoing channel is closed");
                                    break;
                                },
                            }
                        }
                    }
                }
            }
        }

        info!("LP data handler shutdown complete");
    }
}
