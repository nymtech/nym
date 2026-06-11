// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{DelayedPacket, MAX_DRAIN_BATCH, forward_packet};
use futures::StreamExt;
use nym_mixnet_client::SendWithoutResponse;
use nym_mixnet_client::metrics::{MixnetMetric, Traced, observe_delay_drain_batch_size};
use nym_node_metrics::NymNodeMetrics;
use nym_nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, trace};

/// The dedicated [`NonExhaustiveDelayQueue`] task of the [`PacketForwarder`](super::PacketForwarder).
/// Owns the queue exclusively: receives insertions from the [`PacketRouter`](super::PacketRouter)
/// and forwards each packet to the next hop once its release instant passes. Releases are biased
/// ahead of insertions so that, under load, packets whose mix delay has elapsed go out promptly
/// (keeping `DelayQueueOverrun` low) - a delayed insertion is harmless as long as it lands before
/// its (future) release instant, which the bounded drain guarantees.
pub struct DelayForwarder<C> {
    delay_queue: NonExhaustiveDelayQueue<Traced<MixPacket>>,
    mixnet_client: Arc<C>,
    metrics: NymNodeMetrics,

    delayed_receiver: mpsc::Receiver<DelayedPacket>,
}

impl<C> DelayForwarder<C> {
    pub(super) fn new(
        mixnet_client: Arc<C>,
        metrics: NymNodeMetrics,
        delayed_receiver: mpsc::Receiver<DelayedPacket>,
    ) -> Self {
        DelayForwarder {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client,
            metrics,
            delayed_receiver,
        }
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    fn handle_done_delaying(&mut self, packet: Expired<Traced<MixPacket>>)
    where
        C: SendWithoutResponse,
    {
        // how late beyond the target release the queue actually handed the packet back: the
        // delay-queue's own scheduling/retrieval overhead (timer granularity + task wakeup)
        let overrun = Instant::now().saturating_duration_since(packet.deadline());
        let mut delayed_packet = packet.into_inner();
        // close out the DelayQueue stage (the full wait: intended mix delay + overrun)
        delayed_packet.record(MixnetMetric::DelayQueue);
        delayed_packet.record_value(MixnetMetric::DelayQueueOverrun, overrun.as_secs_f64());
        forward_packet(&*self.mixnet_client, &self.metrics, delayed_packet);
    }

    /// Drain every packet whose release deadline has already passed, bounded by [`MAX_DRAIN_BATCH`]
    /// so a release burst can't monopolise the loop. `try_next_expired` never blocks, so this is a
    /// no-op (returns 0) when nothing is due. Returns how many packets were released.
    fn drain_expired(&mut self) -> usize
    where
        C: SendWithoutResponse,
    {
        let mut released = 0;
        while released < MAX_DRAIN_BATCH {
            let Some(expired) = self.delay_queue.try_next_expired() else {
                break;
            };
            self.handle_done_delaying(expired);
            released += 1;
        }
        released
    }

    /// Insert a packet handed over by the router, keyed on its absolute release instant.
    fn handle_insert(&mut self, delayed: DelayedPacket) {
        self.delay_queue
            .insert_at(delayed.packet, delayed.release_at);
    }

    /// Drain every packet currently waiting in the handoff channel into the delay queue, bounded by
    /// [`MAX_DRAIN_BATCH`] so a flood of insertions can't monopolise the loop. `try_recv` never
    /// blocks. Returns how many packets were inserted.
    fn drain_pending_inserts(&mut self) -> usize {
        let mut inserted = 0;
        while inserted < MAX_DRAIN_BATCH {
            // Err = channel empty (or closed, handled by the `recv` arm in `run`)
            let Ok(delayed) = self.delayed_receiver.try_recv() else {
                break;
            };
            self.handle_insert(delayed);
            inserted += 1;
        }
        inserted
    }

    fn update_queue_len_metric(&self) {
        self.metrics
            .process
            .update_forward_hop_packets_being_delayed(self.delay_queue.len());
    }

    pub async fn run(mut self, shutdown_token: ShutdownToken)
    where
        C: SendWithoutResponse,
    {
        trace!("starting DelayForwarder");
        loop {
            // releases serviced this wakeup; the select arm seeds it with the item it consumed
            let mut released = 0usize;
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("DelayForwarder: Received shutdown");
                    break;
                }
                // the work arms only WAKE us and consume their single ready item; the body below
                // then services BOTH branches so neither can starve the other under `biased`
                delayed = self.delay_queue.next() => {
                    // SAFETY: `stream` impl of `NonExhaustiveDelayQueue` never returns `None`
                    #[allow(clippy::unwrap_used)]
                    self.handle_done_delaying(delayed.unwrap());
                    released = 1;
                }
                inserted = self.delayed_receiver.recv() => {
                    let Some(first) = inserted else {
                        debug!("DelayForwarder: router dropped the handoff channel");
                        break;
                    };
                    self.handle_insert(first);
                }
            }

            // bring the queue current (inserts) first so a freshly-arrived-but-already-due packet
            // can release this same wakeup, then flush everything now due (releases)
            self.drain_pending_inserts();
            released += self.drain_expired();
            if released > 0 {
                observe_delay_drain_batch_size(released);
            }

            // update the metric on either an insertion or a release
            self.update_queue_len_metric();
        }
        trace!("DelayForwarder: Exiting");
    }
}
