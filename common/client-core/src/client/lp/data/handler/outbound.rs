// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Messages from the client, packets onto the wire.
//!
//! Everything the client's outbound direction owns. Its counterpart is
//! [`inbound`](super::inbound), and the two share nothing but the tick that drives them - retiring
//! either is deleting its file and its field.

use std::net::SocketAddr;
use std::time::Instant;

use nym_crypto::rng::OsRng;
use nym_lp_data::clients::traits::ClientWrappingPipeline;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_lp_data::{AddressedTimedData, PipelineInput, PipelinePayload};
use nym_task::ShutdownTracker;
use tokio::sync::mpsc::error::TrySendError;
use tracing::warn;

use super::pipeline::{LpOutboundOptions, LpOutboundPipeline};
use super::worker_pool::{Worker, WorkerPool};
use crate::client::lp::LpDataHandlerError;
use crate::client::lp::data::input::LpOutboundMessageReceiver;
use crate::client::lp::data::shared::SharedLpDataState;
use nym_sphinx::addressing::nodes::NodeIdentity;

/// The packets one message became, each with the time it may go out.
type OutboundOutput = Result<Vec<AddressedTimedData<EncryptedLpPacket>>, LpDataHandlerError>;

/// Takes what the client wants to send to packets on the wire, released when they are due.
pub(crate) struct ClientOutbound {
    /// Messages the client has handed to the LP path.
    input_rx: LpOutboundMessageReceiver,

    /// Where a message is chunked, sphinx-wrapped, framed and encrypted.
    pool: WorkerPool<PipelinePayload<LpOutboundOptions>, OutboundOutput>,

    /// The sessions this client holds
    shared_state: SharedLpDataState,

    /// The gateway a message leaves through when it names none.
    ///
    /// Named by identity rather than address so that a message arriving before there is a session
    /// still says which gateway it was for.
    main_gateway: NodeIdentity,

    /// Prepared packets waiting for their release time.
    packet_buffer: Vec<AddressedTimedData<EncryptedLpPacket>>,

    /// Where released packets go to be written to the socket.
    output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
}

impl ClientOutbound {
    pub(crate) fn new(
        pipeline: LpOutboundPipeline<OsRng>,
        input_rx: LpOutboundMessageReceiver,
        output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        shared_state: SharedLpDataState,
        main_gateway: NodeIdentity,
        worker_count: usize,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self {
        // the pool clones it per worker, each drawing its routes independently
        let pool = WorkerPool::spawn("outbound", worker_count, pipeline, shutdown_tracker);

        ClientOutbound {
            input_rx,
            pool,
            shared_state,
            main_gateway,
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

    /// Hand everything waiting to a worker, stamped with this tick.
    ///
    /// The stamp is put on here because a sender knows its message but not the clock this loop runs
    /// on, and the workers need one: a job crosses a thread boundary, so it carries its own time.
    fn dispatch_waiting(&mut self, now: Instant) {
        // drained either way: holding messages back for a gateway we cannot reach would fill the
        // channel and stall every sender behind it, silently
        while let Ok(message) = self.input_rx.try_recv() {
            let gateway = message.gateway.unwrap_or(self.main_gateway);

            // resolved to where the gateway answers - from here down the pipeline deals only in
            // addresses. Dropped rather than queued or dialled for: nothing here asks for a
            // session, so a message with no session to travel on has nothing to wait for
            let Some(address) = self.shared_state.sessions().data_address(gateway) else {
                warn!("LP outbound: no session with gateway {gateway}, dropping a message");
                continue;
            };

            self.pool.dispatch(PipelinePayload::new(
                now,
                message.data,
                message.options,
                address,
            ));
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
            Some(PipelineInput::new(job.data.data, job.options, job.dst)),
            job.data.timestamp,
        )
    }
}
