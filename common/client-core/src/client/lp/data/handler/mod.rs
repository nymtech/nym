// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::lp::LpDataHandlerError;
use crate::client::lp::data::PACKET_BUFFER_SIZE;
use crate::client::lp::data::shared::SharedLpDataState;
use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_lp_data::common::traits::TransportUnwrap;
use nym_lp_data::packet::{EncryptedLpPacket, MalformedLpPacketError};
use nym_lp_data::{AddressedTimedData, TimedData};
use std::sync::{Arc, mpsc};
use std::time::Instant;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::interval;
use tracing::*;

pub mod error;
pub mod messages;
pub mod pipeline;
mod processing;

const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// Bounded queue depth in front of each worker; keeps memory bounded under
/// bursty load and provides drop-based backpressure.
const WORKER_QUEUE_DEPTH: usize = 128;

type WorkerOutput = Result<Option<Vec<u8>>, MalformedLpPacketError>;

/// LP Data Handler for UDP data plane, acts as a pipeline driver and buffer
/// for delaying packets. Heavy per-packet processing is fanned out across a
/// pool of worker threads spawned on the shared blocking pool tracked by the
/// surrounding [`nym_task::ShutdownTracker`].
pub struct LpDataHandler {
    /// Shared state
    shared_state: Arc<SharedLpDataState>,

    // Outbound pipeline
    /// Channel to receive data for the outbound pipeline
    outbound_input_rx: InputMessageReceiver,
    /// Buffer for outbound packet
    outbound_pkt_buffer: Vec<AddressedTimedData<EncryptedLpPacket>>,
    /// Channel to send outgoing data from the outbound pipeline
    outbound_output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,

    // Inbound pipeline
    /// Channel to receive incoming data for the inbound pipeline
    inbound_input_rx: mpsc::Receiver<EncryptedLpPacket>,
    /// Per-worker job queues (round-robin dispatch).
    worker_input_txs: Vec<mpsc::SyncSender<TimedData<EncryptedLpPacket>>>,
    /// Aggregated processed packets returned by the workers. (Inbound data)
    worker_output_rx: mpsc::Receiver<WorkerOutput>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    pub(crate) fn new(
        shared_state: Arc<SharedLpDataState>,
        outbound_input_rx: InputMessageReceiver,
        outbound_output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        inbound_input_rx: mpsc::Receiver<EncryptedLpPacket>,
        // SW TODO : inbound output (worker_output_rx)
        shutdown_tracker: &nym_task::ShutdownTracker,
    ) -> Result<Self, LpDataHandlerError> {
        let (worker_output_tx, worker_output_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);

        // Allow at least one worker, even if the config says 0
        let worker_count = 4; // SW Put that in the config

        // Create workers. They will stop naturally when worker_output_rx is dropped.
        // The mode is decided once here; each closure picks the right pipeline type so
        // the worker loop monomorphizes against a single concrete pipeline.
        let worker_input_txs = (0..worker_count)
            .map(|_| {
                let (worker_input_tx, _worker_input_rx) = mpsc::sync_channel(WORKER_QUEUE_DEPTH);
                let _worker_state = shared_state.clone();
                let _worker_output = worker_output_tx.clone();

                shutdown_tracker.spawn_blocking(move || {
                    // Instantiat pipeline
                    todo!()
                    //Self::run_worker(pipeline, worker_input_rx, worker_output);
                });

                worker_input_tx
            })
            .collect();

        Ok(Self {
            shared_state,
            outbound_input_rx,
            outbound_pkt_buffer: Vec::new(),
            outbound_output_tx,
            inbound_input_rx,
            worker_input_txs,
            worker_output_rx,
            shutdown: shutdown_tracker.clone_shutdown_token(),
        })
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
                            Ok(_packets) => {
                                // Dispatch to application
                                todo!()
                            },
                            Err(e) => {
                                warn!("LP data worker: error processing packet : {e}");
                            },
                        }

                    }
                    // Dispatch incoming packets to workers.
                    while let Ok(input) = self.inbound_input_rx.try_recv() {
                        next_worker = self.dispatch_to_workers(
                            TimedData::new(std_timestamp, input),
                            next_worker,
                        );
                    }

                    // Run outbound pipeline
                    while let Ok(_input) = self.outbound_input_rx.try_recv() {
                       // Run outbound pipeline and stack result in outbound_pkt_buffer
                        todo!()
                    }

                    // Hand over packets whose release time has come. Deliberately a channel and a
                    // non-blocking send: the socket write belongs off this loop, which must not wait on the network.
                    for pkt in self.outbound_pkt_buffer.extract_if(.., |p| p.data.timestamp <= std_timestamp) {
                        if let Err(e) = self.outbound_output_tx.try_send((pkt.data.data, pkt.dst)) {
                            match e {
                                TrySendError::Full(_) =>  {
                                    warn!("LP data handler: packet sending buffer is full, the client might be overloaded");
                                },
                                TrySendError::Closed(_) => {
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
    fn dispatch_to_workers(&self, mut job: TimedData<EncryptedLpPacket>, start: usize) -> usize {
        let n = self.worker_input_txs.len();
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
        start
    }

    fn run_worker<P>(
        mut pipeline: P,
        input_rx: mpsc::Receiver<TimedData<EncryptedLpPacket>>,
        output_tx: mpsc::SyncSender<WorkerOutput>,
    ) where
        P: ClientUnwrappingPipeline<EncryptedLpPacket, ()> // SW fill in message kind
            + TransportUnwrap<EncryptedLpPacket, Error = MalformedLpPacketError>, // This is needed to specify the error type
    {
        while let Ok(input) = input_rx.recv() {
            // Blocking is fine, we don't want to unclog ourself and process a new packet that will be dropped anyway
            if let Err(e) = output_tx.send(pipeline.unwrap(input.data, input.timestamp)) {
                trace!(
                    "Failed to send processing data back to handler : {e}. We are probably shutting down"
                );
                return;
            }
        }
    }
}
