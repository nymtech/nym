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

use crate::node::lp::data::PACKET_BUFFER_SIZE;
use crate::node::lp::data::handler::pipeline::MixnodeDataPipeline;
use crate::node::lp::data::shared::SharedLpDataState;
use nym_lp_data::AddressedTimedData;
use nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline;
use nym_lp_data::packet::{EncryptedLpPacket, MalformedLpPacketError};
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

/// Bounded queue depth in front of each worker; keeps memory bounded under
/// bursty load and provides drop-based backpressure.
const WORKER_QUEUE_DEPTH: usize = 128;

type WorkerOutput =
    Result<Vec<AddressedTimedData<Instant, EncryptedLpPacket, SocketAddr>>, MalformedLpPacketError>;

/// A single packet processing job dispatched to a worker thread.
struct WorkerInput {
    packet: EncryptedLpPacket,
    timestamp: Instant,
}

/// LP Data Handler for UDP data plane, acts as a pipeline driver and buffer
/// for delaying packets. Heavy per-packet processing is fanned out across a
/// pool of worker threads spawned on the shared blocking pool tracked by the
/// surrounding [`nym_task::ShutdownTracker`].
pub struct LpDataHandler {
    /// Shared state
    shared_state: Arc<SharedLpDataState>,

    /// Channel to receive incoming data
    input_rx: mpsc::Receiver<EncryptedLpPacket>,

    /// Channel to send outgoing data
    output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,

    /// Per-worker job queues (round-robin dispatch).
    worker_input_txs: Vec<mpsc::SyncSender<WorkerInput>>,

    /// Aggregated processed packets returned by the workers.
    worker_output_rx: mpsc::Receiver<WorkerOutput>,

    outgoing_pkt_buffer: Vec<AddressedTimedData<Instant, EncryptedLpPacket, SocketAddr>>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    pub(crate) fn new(
        shared_state: Arc<SharedLpDataState>,
        input_rx: mpsc::Receiver<EncryptedLpPacket>,
        output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        shutdown_tracker: &nym_task::ShutdownTracker,
    ) -> Self {
        let (worker_output_tx, worker_output_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);

        // Allow at least one worker, even if the config says 0
        let worker_count = shared_state.lp_config.debug.data_worker_count.max(1);

        // Create workers. They will stop naturally when worker_output_rx is dropped
        let worker_input_txs = (0..worker_count)
            .map(|_| {
                let (worker_input_tx, worker_input_rx) = mpsc::sync_channel(WORKER_QUEUE_DEPTH);
                let worker_state = shared_state.clone();
                let worker_output = worker_output_tx.clone();
                shutdown_tracker.spawn_blocking(move || {
                    Self::run_worker(worker_state, worker_input_rx, worker_output)
                });

                worker_input_tx
            })
            .collect();

        Self {
            shared_state,
            input_rx,
            output_tx,
            worker_input_txs,
            worker_output_rx,
            outgoing_pkt_buffer: Vec::new(),
            shutdown: shutdown_tracker.clone_shutdown_token(),
        }
    }

    pub async fn run(&mut self) {
        info!(
            workers = self.worker_input_txs.len(),
            "Starting LP data handler"
        );
        let mut ticking_interval = interval(PIPELINE_TICKING_DURATION);
        let mut next_worker = 0;

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data handler: received shutdown signal");
                    break;
                }

                timestamp = ticking_interval.tick() => {
                    let std_timestamp: Instant = timestamp.into();

                    // Drain processed packets returned by workers.
                    while let Ok(processing_result) = self.worker_output_rx.try_recv() {
                        match processing_result {
                            Ok(packets) => {
                                self.outgoing_pkt_buffer.extend(packets);
                            },
                            Err(e) => {
                                warn!("LP data worker: error processing packet : {e}");
                                inc!("lp_data_packet_errors");
                            },
                        }

                    }
                    // Dispatch incoming packets to workers.
                    while let Ok(input) = self.input_rx.try_recv() {
                        next_worker = self.dispatch_to_workers(
                            input,
                            std_timestamp,
                            next_worker,
                        );
                    }

                    // Send packets that needs sending
                    for pkt in self.outgoing_pkt_buffer.extract_if(.., |p| p.data.timestamp <= std_timestamp) {
                        if let Err(e) = self.output_tx.try_send((pkt.data.data, pkt.dst)) {
                            match e {
                                TrySendError::Full(_) =>  {
                                    warn!("LP data handler: packet sending buffer is full, the node might be overloaded");
                                    self.shared_state.egress_overloaded_packet_dropped();
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

        // Workers will stop because we are dropping the receiving channel
        info!("LP data handler shutdown complete");
    }

    /// Round-robin dispatch a job across worker queues. If the chosen worker is
    /// full, fall through to the next one; if all are saturated, drop the packet
    /// (UDP-style) and bump a metric. Returns the worker index to start from on
    /// the next dispatch.
    fn dispatch_to_workers(
        &self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
        start: usize,
    ) -> usize {
        let n = self.worker_input_txs.len();
        let mut job = WorkerInput { packet, timestamp };
        for offset in 0..n {
            let idx = (start + offset) % n;
            match self.worker_input_txs[idx].try_send(job) {
                Ok(()) => return (idx + 1) % n,
                Err(mpsc::TrySendError::Full(returned)) => {
                    job = returned;
                }
                Err(mpsc::TrySendError::Disconnected(returned)) => {
                    error!(
                        "LP data worker {idx} disconnected; this shouldn't happen outside of shut down"
                    );
                    job = returned;
                }
            }
        }

        warn!("LP data handler: all workers saturated, dropping packet");
        self.shared_state.worker_pool_overloaded_packet_dropped();
        start
    }

    fn run_worker(
        state: Arc<SharedLpDataState>,
        input_rx: mpsc::Receiver<WorkerInput>,
        output_rx: mpsc::SyncSender<WorkerOutput>,
    ) {
        let mut pipeline = MixnodeDataPipeline::new(state.clone(), OsRng);
        while let Ok(input) = input_rx.recv() {
            // Blocking is fine, we don't want to unclog ourself and process a new packet that will be dropped anyway
            if let Err(e) = output_rx.send(pipeline.process(input.packet, input.timestamp)) {
                trace!(
                    "Failed to send processing data back to handler : {e}. We are probably shutting down"
                );
                return;
            }
        }
    }
}
