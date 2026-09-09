// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Messages from the client, packets onto the wire.
//!
//! Everything the client's outbound direction owns. Its counterpart is
//! [`inbound`](super::inbound), and the two share nothing but the tick that drives them - retiring
//! either is deleting its file and its field.

use std::net::SocketAddr;
use std::sync::{Arc, mpsc};
use std::time::Instant;

use nym_lp_data::clients::traits::ClientWrappingPipeline;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_lp_data::{AddressedTimedData, PipelinePayload};
use nym_task::ShutdownTracker;
use rand::rngs::OsRng;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use super::pipeline::{LpOutboundOptions, LpOutboundPipeline};
use super::worker_pool::{Worker, WorkerPool};
use crate::client::inbound_messages::{InputMessage, InputMessageReceiver};
use crate::client::lp::LpDataHandlerError;
use crate::client::lp::data::shared::SharedLpDataState;

/// The packets one message became, each with the time it may go out.
type OutboundOutput = Result<Vec<AddressedTimedData<EncryptedLpPacket>>, LpDataHandlerError>;

/// What the wrapping pipeline is defined over: a payload, what the stages need to know about it,
/// and where it goes.
///
/// No timestamp: a sender knows its message, not the handler's tick, so the stamp is put on at this
/// end - see [`ClientOutbound::dispatch_waiting`].
pub(crate) type LpOutboundInput = (Vec<u8>, LpOutboundOptions, SocketAddr);

/// Sends jobs to the LP outbound direction.
pub(crate) type LpOutboundJobSender = mpsc::Sender<LpOutboundInput>;

/// Takes what the client wants to send to packets on the wire, released when they are due.
///
/// Two ways in, and only one of them is meant to last. [`LpOutboundJob`] is the pipeline's own
/// language; [`InputMessage`] is a dialect of it that carries no destination, adapted by
/// [`outbound_job`] against the gateway this client registered with. When everything submits the
/// triplet, that adapter and its channel are what goes.
pub(crate) struct ClientOutbound {
    /// Messages the client has handed to the LP path, needing a gateway chosen for them.
    input_rx: InputMessageReceiver,

    /// Jobs that already say where they are going.
    job_rx: mpsc::Receiver<LpOutboundInput>,

    /// Where a message is chunked, sphinx-wrapped, framed and encrypted.
    pool: WorkerPool<PipelinePayload<LpOutboundOptions>, OutboundOutput>,

    /// The sessions this client holds
    shared_state: Arc<SharedLpDataState>,

    /// Prepared packets waiting for their release time.
    packet_buffer: Vec<AddressedTimedData<EncryptedLpPacket>>,

    /// Where released packets go to be written to the socket.
    output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
}

impl ClientOutbound {
    pub(crate) fn new(
        pipeline: LpOutboundPipeline<OsRng>,
        input_rx: InputMessageReceiver,
        job_rx: mpsc::Receiver<LpOutboundInput>,
        output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        shared_state: Arc<SharedLpDataState>,
        worker_count: usize,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self {
        // the pool clones it per worker, each drawing its routes independently
        let pool = WorkerPool::spawn("outbound", worker_count, pipeline, shutdown_tracker);

        ClientOutbound {
            input_rx,
            job_rx,
            pool,
            shared_state,
            packet_buffer: Vec::new(),
            output_tx,
        }
    }

    pub(crate) fn worker_count(&self) -> usize {
        self.pool.len()
    }

    /// One turn of this direction: collect what is prepared, hand on what is waiting, release what
    /// is due.
    pub(crate) fn tick(&mut self, now: Instant) {
        self.buffer_prepared();
        self.dispatch_waiting(now);
        self.release_due_packets(now);
    }

    /// Buffer what the workers have prepared, against its release time.
    fn buffer_prepared(&mut self) {
        for prepared in self.pool.drain() {
            match prepared {
                Ok(packets) => self.packet_buffer.extend(packets),
                Err(err) => warn!("LP outbound worker: error preparing a message: {err}"),
            }
        }
    }

    /// Hand everything waiting on either input to a worker, stamped with this tick.
    ///
    /// The stamp is put on here because a sender knows its message but not the clock this loop runs
    /// on, and the workers need one: a job crosses a thread boundary, so it carries its own time.
    fn dispatch_waiting(&mut self, now: Instant) {
        while let Ok((payload, options, dst)) = self.job_rx.try_recv() {
            self.pool
                .dispatch(PipelinePayload::new(now, payload, options, dst));
        }

        // the dialect that names no gateway, so one is chosen for it
        if let Some(gateway) = self.shared_state.sessions.any_gateway() {
            while let Ok(message) = self.input_rx.try_recv() {
                let Some(job) = outbound_job(message, gateway, now) else {
                    continue;
                };
                self.pool.dispatch(job);
            }
        }
    }

    /// Hand over packets whose release time has come.
    ///
    /// Deliberately a channel and a non-blocking send: the socket write belongs off the tick loop,
    /// which runs every millisecond and must not wait on the network.
    fn release_due_packets(&mut self, now: Instant) {
        if self.packet_buffer.is_empty() {
            return;
        }

        // `Vec::extract_if` would say this better but is newer than our MSRV
        let (due, waiting) = self
            .packet_buffer
            .drain(..)
            .partition(|p| p.data.timestamp <= now);
        self.packet_buffer = waiting;

        for pkt in due {
            if let Err(err) = self.output_tx.try_send((pkt.data.data, pkt.dst)) {
                match err {
                    TrySendError::Full(_) => {
                        warn!(
                            "LP data handler: packet sending buffer is full, the client might be overloaded"
                        );
                    }
                    TrySendError::Closed(_) => break,
                }
            }
        }
    }
}

/// One message, the whole way: chunk, sphinx-wrap, frame, encrypt.
impl Worker for LpOutboundPipeline<OsRng> {
    type I = PipelinePayload<LpOutboundOptions>;
    type O = OutboundOutput;

    fn handle(&mut self, job: Self::I) -> Self::O {
        self.process(
            Some((job.data.data, job.options, job.dst)),
            job.data.timestamp,
        )
    }
}

/// What an outbound worker can do with an [`InputMessage`], if anything.
// SW temporary until we settle on how we should take in inputs
fn outbound_job(
    message: InputMessage,
    gateway: SocketAddr,
    timestamp: Instant,
) -> Option<PipelinePayload<LpOutboundOptions>> {
    match message {
        InputMessage::Regular {
            recipient, data, ..
        } => Some(PipelinePayload::new(
            timestamp,
            data,
            LpOutboundOptions { recipient },
            gateway,
        )),

        InputMessage::MessageWrapper { message, .. } => outbound_job(*message, gateway, timestamp),

        // `Anonymous` and `Reply` need a reply SURB, which this path has no room for. `Premade` is
        // already sphinx, built for a route this path does not take.
        _ => {
            warn!(
                "LP data handler: only regular messages can travel over LP, dropping one that is not"
            );
            None
        }
    }
}
