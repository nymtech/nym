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
use nym_task::ShutdownTracker;
use tracing::{debug, trace, warn};

use super::pipeline::LpInboundPipeline;
use super::worker_pool::{Worker, WorkerPool};
use crate::client::lp::LpDataHandlerError;

/// A message, when the packet completed one.
type InboundOutput = Result<Option<Vec<u8>>, LpDataHandlerError>;

/// Takes arriving packets to finished messages and delivers them.
pub(crate) struct ClientInbound {
    /// Packets the listener has read off the socket.
    input_rx: mpsc::Receiver<EncryptedLpPacket>,

    /// Where a packet is decrypted, unframed, unwrapped and reassembled.
    pool: WorkerPool<TimedData<EncryptedLpPacket>, InboundOutput>,

    output_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl ClientInbound {
    pub(crate) fn new(
        pipeline: LpInboundPipeline,
        input_rx: mpsc::Receiver<EncryptedLpPacket>,
        output_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
        worker_count: usize,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self {
        // the pool clones it per worker, sharing the sessions and reconstructors
        let pool = WorkerPool::spawn("inbound", worker_count, pipeline, shutdown_tracker);

        ClientInbound {
            input_rx,
            pool,
            output_tx,
        }
    }

    pub(crate) fn worker_count(&self) -> usize {
        self.pool.len()
    }

    /// One turn of this direction: deliver what is finished, then hand on what has arrived.
    pub(crate) fn tick(&mut self, now: Instant) {
        self.deliver_finished();
        self.dispatch_arrived(now);
    }

    /// Deliver the messages the workers finished reassembling.
    fn deliver_finished(&mut self) {
        for processed in self.pool.drain() {
            match processed {
                // never waits: inbound and outbound share one loop, so a slow reader must not
                // stall sending
                Ok(Some(message)) => {
                    if let Err(err) = self.output_tx.try_send(message) {
                        debug!("dropping a received LP message: {err}");
                    }
                }
                Ok(None) => trace!("LP data handler: packet completed no message"),
                Err(err) => warn!("LP inbound worker: error processing a packet: {err}"),
            }
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
