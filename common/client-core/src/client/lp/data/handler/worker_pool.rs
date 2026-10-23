// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A pool of blocking workers, fed round-robin.
//!
//! Both directions of the data plane want the same thing: hand a job to one of N threads, collect
//! whatever they finish. Only the job and the result differ, so they are the type parameters.

use std::sync::mpsc;

use nym_task::ShutdownTracker;
use tracing::{error, trace, warn};

use crate::client::lp::data::PACKET_BUFFER_SIZE;

/// Bounded queue depth in front of each worker; keeps memory bounded under
/// bursty load and provides drop-based backpressure.
const WORKER_QUEUE_DEPTH: usize = 128;

/// One job's worth of work, which a [`WorkerPool`] runs on a thread of its own.
///
/// Implementors say only what one job becomes; the receiving, the handing back and the shutting
/// down are the pool's.
///
/// [`Clone`] is how the pool populates itself: it is handed one of these and clones it per thread,
/// so what a clone shares is what the pool shares.
///
/// [`LpOutboundPipeline`]: super::pipeline::LpOutboundPipeline
pub(crate) trait Worker: Clone + Send + 'static {
    /// What the pool dispatches.
    type I: Send + 'static;

    /// What comes back, in whatever order the workers finish.
    type O: Send + 'static;

    /// One job, start to finish.
    fn handle(&mut self, job: Self::I) -> Self::O;
}

/// `N` blocking workers turning `I`nputs into `O`utputs.
pub(crate) struct WorkerPool<I, O> {
    /// Per-worker job queues.
    txs: Vec<mpsc::SyncSender<I>>,

    /// What every worker hands back, in whatever order they finish.
    output_rx: mpsc::Receiver<O>,

    /// Where the next round-robin dispatch starts looking.
    next: usize,

    /// Names this pool in its logs.
    label: &'static str,
}

impl<I, O> WorkerPool<I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// Spawn `worker_count` threads, each with a clone of `worker`.
    ///
    /// They stop on their own once the pool is dropped, which takes the job queues with it.
    pub(crate) fn spawn<W>(
        label: &'static str,
        worker_count: usize,
        worker: W,
        shutdown_tracker: &ShutdownTracker,
    ) -> Self
    where
        W: Worker<I = I, O = O>,
    {
        let (output_tx, output_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);

        let txs = (0..worker_count)
            .map(|_| {
                let (input_tx, input_rx) = mpsc::sync_channel(WORKER_QUEUE_DEPTH);
                let mut worker = worker.clone();
                let output_tx = output_tx.clone();

                shutdown_tracker.spawn_blocking(move || {
                    while let Ok(job) = input_rx.recv() {
                        // Blocking is fine, we don't want to unclog ourself and take on a new job
                        // whose result would be dropped anyway
                        if output_tx.send(worker.handle(job)).is_err() {
                            trace!(
                                "LP {label} worker: could not hand a result back to the handler. We are probably shutting down"
                            );
                            return;
                        }
                    }
                });

                input_tx
            })
            .collect();

        WorkerPool {
            txs,
            output_rx,
            next: 0,
            label,
        }
    }

    /// How many workers this pool runs.
    pub(crate) fn len(&self) -> usize {
        self.txs.len()
    }

    /// Give a job to the next worker in turn.
    ///
    /// If that one is full, fall through to the next; if all are saturated, drop the job
    /// (UDP-style) rather than stalling the caller, which is a loop that must keep ticking.
    pub(crate) fn dispatch(&mut self, mut job: I) {
        let worker_count = self.txs.len();

        for offset in 0..worker_count {
            let idx = (self.next + offset) % worker_count;

            // SAFETY: idx is next+offset modulo the length
            #[expect(clippy::indexing_slicing)]
            match self.txs[idx].try_send(job) {
                Ok(()) => {
                    self.next = (idx + 1) % worker_count;
                    return;
                }
                Err(mpsc::TrySendError::Full(returned)) => {
                    job = returned;
                }
                Err(mpsc::TrySendError::Disconnected(returned)) => {
                    error!(
                        "LP {} worker {idx}: disconnected, which should not happen outside of shutdown",
                        self.label
                    );
                    job = returned;
                }
            }
        }

        warn!(
            "LP data handler: all {} workers saturated, dropping a job",
            self.label
        );
    }

    /// Everything the workers have finished since the last look, taken without blocking.
    pub(crate) fn drain(&self) -> mpsc::TryIter<'_, O> {
        self.output_rx.try_iter()
    }
}
