// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Packets off the wire, messages into the client.
//!
//! Everything the client's inbound direction owns. Its counterpart is
//! [`outbound`](super::outbound), and the two share nothing but the tick that drives them -
//! retiring either is deleting its file and its field.

use std::sync::mpsc;
use std::time::Instant;

use nym_lp_data::TimedData;
use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_sphinx::message::NymMessage;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::ShutdownTracker;
use tracing::{trace, warn};

use super::pipeline::LpInboundPipeline;
use super::worker_pool::{Worker, WorkerPool};
use crate::client::lp::LpDataHandlerError;
use crate::client::received_buffer::ReceivedMessagesBuffer;

/// A message, when the packet completed one.
type InboundOutput = Result<Option<Vec<u8>>, LpDataHandlerError>;

/// Takes arriving packets to finished messages and delivers them.
pub(crate) struct ClientInbound {
    /// Packets the listener has read off the socket.
    input_rx: mpsc::Receiver<EncryptedLpPacket>,

    /// Where a packet is decrypted, unframed, unwrapped and reassembled.
    pool: WorkerPool<TimedData<EncryptedLpPacket>, InboundOutput>,

    /// Where finished messages go, the same place the websocket path puts them.
    received_buffer: ReceivedMessagesBuffer<SphinxMessageReceiver>,
}

impl ClientInbound {
    pub(crate) fn new(
        pipeline: LpInboundPipeline,
        input_rx: mpsc::Receiver<EncryptedLpPacket>,
        received_buffer: ReceivedMessagesBuffer<SphinxMessageReceiver>,
        worker_count: usize,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self {
        // the pool clones it per worker, sharing the sessions and reconstructors
        let pool = WorkerPool::spawn("inbound", worker_count, pipeline, shutdown_tracker);

        ClientInbound {
            input_rx,
            pool,
            received_buffer,
        }
    }

    pub(crate) fn worker_count(&self) -> usize {
        self.pool.len()
    }

    /// One turn of this direction: deliver what is finished, then hand on what has arrived.
    pub(crate) async fn tick(&mut self, now: Instant) {
        self.deliver_finished().await;
        self.dispatch_arrived(now);
    }

    /// Deliver the messages the workers finished reassembling.
    async fn deliver_finished(&mut self) {
        let mut received = Vec::new();

        for processed in self.pool.drain() {
            match processed {
                Ok(Some(message)) => received.push(NymMessage::new_plain(message)),
                Ok(None) => trace!("LP data handler: packet completed no message"),
                Err(err) => warn!("LP inbound worker: error processing a packet: {err}"),
            }
        }

        if !received.is_empty() {
            self.received_buffer
                .handle_reconstructed_messages(received)
                .await;
        }
    }

    /// Hand arriving packets to a worker, which takes them the whole way.
    fn dispatch_arrived(&mut self, now: Instant) {
        while let Ok(packet) = self.input_rx.try_recv() {
            self.pool.dispatch(TimedData::new(now, packet));
        }
    }
}

/// One packet, the whole way: decrypt, unframe, unwrap the sphinx layer, reassemble.
impl Worker for LpInboundPipeline {
    type I = TimedData<EncryptedLpPacket>;
    type O = InboundOutput;

    fn handle(&mut self, job: Self::I) -> Self::O {
        self.unwrap(job.data, job.timestamp)
    }
}
