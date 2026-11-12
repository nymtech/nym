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
//! another, so a dispatcher puts them on one queue that every worker pulls from, each holding a
//! clone of the pipeline. What the clones share is the reassembler, behind its own lock, so
//! whichever worker happens to receive a message's last fragment is the one that completes it.

use std::time::Instant;

use crossbeam_channel::TrySendError;
use nym_lp_data::clients::traits::ClientUnwrappingPipeline;
use nym_task::ShutdownTracker;
use tracing::{debug, warn};

use crate::lp::handler::pipeline::SpInboundPipeline;
use crate::lp::ServiceProviderInputReceiver;

/// How many packets may be queued per worker.
///
/// Bounded like every other queue on this path, and for the same reason: a pool this far behind
/// will not catch up by being handed more to hold.
const WORKER_QUEUE_DEPTH: usize = 32;

pub(crate) struct SpInbound {
    /// Sphinx packets the gateway has decided are for this provider.
    from_gateway: ServiceProviderInputReceiver,

    /// The queue every worker pulls its next packet from, so a packet goes to whichever is free.
    jobs_tx: crossbeam_channel::Sender<Vec<u8>>,

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
        let worker_count = worker_count.max(1);
        let (jobs_tx, jobs) = crossbeam_channel::bounded(WORKER_QUEUE_DEPTH * worker_count);

        let workers = (0..worker_count)
            .map(|_| SpInboundWorker {
                pipeline: pipeline.clone(),
                jobs: jobs.clone(),
                to_provider: to_provider.clone(),
            })
            .collect();

        SpInbound {
            from_gateway,
            jobs_tx,
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
            jobs_tx,
            workers,
        } = self;

        for worker in workers {
            shutdown_tracker.spawn_blocking(move || worker.run());
        }

        shutdown_tracker.spawn_blocking(move || Self::dispatch(from_gateway, jobs_tx));
    }

    /// Queue packets for the pool until the gateway or the pool goes away.
    ///
    /// Ends when the gateway drops its sender, which is how it says it is shutting down - there is
    /// no token to watch, because a blocking receive is what this thread does. Dropping the queue's
    /// sender on the way out is what stops the workers.
    ///
    /// A packet that does not fit is dropped, exactly as one arriving at a full channel from the
    /// gateway is.
    fn dispatch(
        from_gateway: ServiceProviderInputReceiver,
        jobs_tx: crossbeam_channel::Sender<Vec<u8>>,
    ) {
        while let Ok(packet) = from_gateway.recv() {
            match jobs_tx.try_send(packet) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    warn!("LP provider inbound: every worker is saturated, dropping a packet");
                }
                Err(TrySendError::Disconnected(_)) => {
                    debug!("LP provider inbound: every worker has stopped");
                    return;
                }
            }
        }

        debug!("LP provider inbound: the gateway closed the channel");
    }
}

/// One peeling thread.
struct SpInboundWorker {
    /// Where a sphinx layer addressed to this provider is peeled, and its fragments reassembled.
    ///
    /// A clone per worker; the reassembler inside is the shared part.
    pipeline: SpInboundPipeline,

    /// The pool's shared queue.
    jobs: crossbeam_channel::Receiver<Vec<u8>>,

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
