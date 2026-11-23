// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A pool of blocking workers, all pulling from one shared queue.
//!
//! Both directions of the data plane want the same thing: hand a job to whichever of N threads is
//! free, collect whatever they finish. Only the job and the result differ, so they are the type
//! parameters.

use std::sync::mpsc;

use crossbeam_channel::TrySendError;
use nym_task::ShutdownTracker;
use tracing::{error, trace, warn};

use crate::client::lp::data::PACKET_BUFFER_SIZE;

/// Queue depth per worker; the shared queue holds `worker_count * WORKER_QUEUE_DEPTH` jobs, which
/// keeps memory bounded under bursty load and provides drop-based backpressure.
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
    /// The queue every worker pulls its next job from, so a job goes to whichever is free.
    input_tx: crossbeam_channel::Sender<I>,

    /// What every worker hands back, in whatever order they finish.
    output_rx: mpsc::Receiver<O>,

    worker_count: usize,

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
    /// They stop on their own once the pool is dropped, which takes the job queue with it.
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
        let (input_tx, input_rx) =
            crossbeam_channel::bounded::<I>(WORKER_QUEUE_DEPTH * worker_count);

        for _ in 0..worker_count {
            let input_rx = input_rx.clone();
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
        }

        WorkerPool {
            input_tx,
            output_rx,
            worker_count,
            label,
        }
    }

    /// How many workers this pool runs.
    pub(crate) fn len(&self) -> usize {
        self.worker_count
    }

    /// Queue a job for whichever worker is free next.
    ///
    /// If the queue is full, every worker is saturated: drop the job (UDP-style) rather than
    /// stalling the caller, which is a loop that must keep ticking.
    pub(crate) fn dispatch(&self, job: I) {
        match self.input_tx.try_send(job) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                warn!(
                    "LP data handler: all {} workers saturated, dropping a job",
                    self.label
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                error!(
                    "LP {} workers: all gone, which should not happen outside of shutdown",
                    self.label
                );
            }
        }
    }

    /// Everything the workers have finished since the last look, taken without blocking.
    pub(crate) fn drain(&self) -> mpsc::TryIter<'_, O> {
        self.output_rx.try_iter()
    }
}
