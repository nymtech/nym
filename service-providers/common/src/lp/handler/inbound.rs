// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sphinx from the gateway, plaintext to the provider.
//!
//! Everything the provider's inbound direction owns. Its counterpart is [`outbound`](super::outbound),
//! and the two share nothing - retiring either is deleting its file and its field.
//!
//! Unlike its counterpart there is no tick here, because nothing on the way in is scheduled: a
//! packet is peeled when it arrives and either completes a message or does not. A tick would be
//! ceremony over a channel receive.
//!
//! There is a pool, though, which the counterpart also has no use for. Peeling is the expensive
//! half - an x25519 operation and an AEAD open per packet - and packets are independent of one
//! another, so a dispatcher hands them round-robin to workers that each hold a clone of the
//! pipeline. What the clones share is the reassembler, behind its own lock, so whichever worker
//! happens to receive a message's last fragment is the one that completes it.

use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Instant;

use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_task::ShutdownTracker;
use tracing::{debug, warn};

use crate::lp::handler::pipeline::SpInboundPipeline;
use crate::lp::ServiceProviderInputReceiver;

/// How many packets may be queued in front of each worker.
///
/// Bounded like every other queue on this path, and for the same reason: a worker this far behind
/// will not catch up by being handed more to hold.
const WORKER_QUEUE_DEPTH: usize = 32;

pub(crate) struct SpInbound {
    /// Sphinx packets the gateway has decided are for this provider.
    from_gateway: ServiceProviderInputReceiver,

    /// The way in to each worker, dispatched round-robin.
    worker_txs: Vec<SyncSender<Vec<u8>>>,

    /// The peeling threads, built but not yet spawned.
    workers: Vec<SpInboundWorker>,
}

impl SpInbound {
    pub(crate) fn new(
        pipeline: SpInboundPipeline,
        from_gateway: ServiceProviderInputReceiver,
        to_provider: tokio::sync::mpsc::Sender<Vec<u8>>,
        worker_count: usize,
    ) -> Self {
        // a pool of none would leave the gateway writing into a channel nobody reads
        let (worker_txs, workers) = (0..worker_count.max(1))
            .map(|_| {
                let (jobs_tx, jobs) = std::sync::mpsc::sync_channel(WORKER_QUEUE_DEPTH);

                let worker = SpInboundWorker {
                    pipeline: pipeline.clone(),
                    jobs,
                    to_provider: to_provider.clone(),
                };

                (jobs_tx, worker)
            })
            .unzip();

        SpInbound {
            from_gateway,
            worker_txs,
            workers,
        }
    }

    /// Spawn the pool and the dispatcher that feeds it.
    ///
    /// Blocking threads rather than async tasks: every one of them is doing sphinx work, which is
    /// synchronous and CPU-bound, and none belongs on the runtime's cooperative threads.
    pub(crate) fn start(self, shutdown_tracker: &ShutdownTracker) {
        let SpInbound {
            from_gateway,
            worker_txs,
            workers,
        } = self;

        for worker in workers {
            shutdown_tracker.spawn_blocking(move || worker.run());
        }

        shutdown_tracker.spawn_blocking(move || Self::dispatch(from_gateway, worker_txs));
    }

    /// Hand packets to workers until the gateway or the pool goes away.
    ///
    /// Ends when the gateway drops its sender, which is how it says it is shutting down - there is
    /// no token to watch, because a blocking receive is what this thread does.
    fn dispatch(from_gateway: ServiceProviderInputReceiver, worker_txs: Vec<SyncSender<Vec<u8>>>) {
        let mut next = 0;

        while let Ok(packet) = from_gateway.recv() {
            let Some(following) = Self::dispatch_one(&worker_txs, packet, next) else {
                debug!("LP provider inbound: every worker has stopped");
                return;
            };
            next = following;
        }

        debug!("LP provider inbound: the gateway closed the channel");
    }

    /// Round-robin from `start`, trying every worker before giving up on a packet.
    ///
    /// A packet that fits nowhere is dropped, exactly as one arriving at a full channel from the
    /// gateway is. `None` once every worker has gone, which happens at shutdown and means there is
    /// nothing left to dispatch to.
    fn dispatch_one(
        worker_txs: &[SyncSender<Vec<u8>>],
        mut packet: Vec<u8>,
        start: usize,
    ) -> Option<usize> {
        let count = worker_txs.len();
        let mut any_alive = false;

        for offset in 0..count {
            let idx = (start + offset) % count;
            match worker_txs[idx].try_send(packet) {
                Ok(()) => return Some((idx + 1) % count),
                Err(TrySendError::Full(returned)) => {
                    any_alive = true;
                    packet = returned;
                }
                Err(TrySendError::Disconnected(returned)) => packet = returned,
            }
        }

        if !any_alive {
            return None;
        }

        warn!("LP provider inbound: every worker is saturated, dropping a packet");
        Some(start)
    }
}

/// One peeling thread.
struct SpInboundWorker {
    /// Where a sphinx layer addressed to this provider is peeled, and its fragments reassembled.
    ///
    /// A clone per worker; the reassembler inside is the shared part.
    pipeline: SpInboundPipeline,

    /// Packets the dispatcher has given this worker.
    jobs: std::sync::mpsc::Receiver<Vec<u8>>,

    /// Where completed messages go, as plaintext.
    to_provider: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl SpInboundWorker {
    /// Peel what the dispatcher hands over until one end or the other goes away.
    fn run(mut self) {
        while let Ok(packet) = self.jobs.recv() {
            let unwrapped = match self.pipeline.unwrap(packet, Instant::now()) {
                Ok(unwrapped) => unwrapped,
                Err(err) => {
                    warn!("LP provider inbound: dropping a packet: {err}");
                    continue;
                }
            };

            // `None` is ordinary: cover traffic, or a fragment that did not complete its message
            let Some(plaintext) = unwrapped else {
                continue;
            };

            if self.to_provider.blocking_send(plaintext).is_err() {
                debug!("LP provider inbound: the provider stopped listening");
                return;
            }
        }
    }
}
