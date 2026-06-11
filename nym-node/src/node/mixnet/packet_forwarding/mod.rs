// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::routing_filter::RoutingFilter;
use futures::StreamExt;
use nym_mixnet_client::SendWithoutResponse;
use nym_mixnet_client::forwarder::{
    MixForwardingReceiver, MixForwardingSender, PacketToForward, mix_forwarding_channels,
};
use nym_mixnet_client::metrics::{
    MixnetMetric, Traced, observe_delay_drain_batch_size, observe_drain_batch_size,
};
use nym_node_metrics::NymNodeMetrics;
use nym_nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::io;
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

/// Max packets handled per `select!` wakeup, per drainable branch (ingress channel and expired
/// delay-queue items), before yielding back to the biased select so shutdown stays responsive.
/// Per-packet work is sub-µs to low-µs, so 256 bounds the worst-case stall to <~1ms.
const MAX_DRAIN_BATCH: usize = 256;

/// The node's single forward-hop egress engine - the last in-node stage of the mixnet pipeline.
///
/// **Where it sits.** Inbound packets are accepted by the mixnet listener and processed
/// per-connection by a `ConnectionHandler`: sphinx unwrap, replay check, and - for *forward* hops -
/// computation of the intended (Poisson) mix delay. The handler hands each one off as a
/// [`PacketToForward`] over the unbounded ingress-to-forwarder channel. This forwarder is the sole
/// consumer of that channel: every forward-hop packet in the node (plus acks) funnels through it.
/// Final-hop packets never reach here; they are delivered to local clients instead.
///
/// **What it does**, per packet:
/// 1. drops it if the [`RoutingFilter`] doesn't recognise the next hop;
/// 2. holds it in the delay queue until its target release instant (the mix delay), or forwards it
///    immediately when the delay is zero or has already elapsed;
/// 3. on release, forwards it to the next hop via the mixnet client (`C: SendWithoutResponse`),
///    which owns the per-connection egress TCP sockets.
///
/// **Design notes.** It runs as one dedicated task owning both the ingress channel and the delay
/// queue. Its [`run`](Self::run) loop wakes on either a new packet or a delay-queue release, then
/// services *both* branches every wakeup - draining queued ingress packets first (to bring the
/// delay queue current) and then releasing everything now due - so a burst on one branch can never
/// starve the other (the failure mode a biased "one branch per wakeup" select would have). Each
/// drain is bounded by [`MAX_DRAIN_BATCH`] so shutdown stays responsive. Along the way it stamps
/// the latency-trace stages it owns (`ForwarderQueue`, `DelayQueue`, `DelayQueueOverrun`), feeding
/// the `mixnet_packet_*` metrics family.
pub struct PacketForwarder<C, F> {
    delay_queue: NonExhaustiveDelayQueue<Traced<MixPacket>>,
    mixnet_client: C,

    metrics: NymNodeMetrics,
    routing_filter: F,

    packet_sender: MixForwardingSender,
    packet_receiver: MixForwardingReceiver,
}

impl<C, F> PacketForwarder<C, F> {
    pub fn new(client: C, routing_filter: F, metrics: NymNodeMetrics) -> Self {
        let (packet_sender, packet_receiver) = mix_forwarding_channels();

        PacketForwarder {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client: client,
            metrics,
            routing_filter,
            packet_sender,
            packet_receiver,
        }
    }

    pub fn sender(&self) -> MixForwardingSender {
        self.packet_sender.clone()
    }

    fn forward_packet(&mut self, packet: Traced<MixPacket>)
    where
        C: SendWithoutResponse,
    {
        let next_hop = packet.inner.next_hop_address();

        if let Err(err) = self.mixnet_client.send_without_response(packet) {
            if err.kind() == io::ErrorKind::WouldBlock {
                // we only know for sure if we dropped a packet if our sending queue was full
                trace!(
                    event = "packet.dropped.buffer_full",
                    next_hop = %next_hop,
                    "dropping packet: egress connection buffer full (WouldBlock)"
                );
                self.metrics.mixnet.egress_dropped_forward_packet(next_hop)
            } else if err.kind() == io::ErrorKind::NotConnected {
                debug!(
                    next_hop = %next_hop,
                    "packet queued for not-yet-connected peer"
                );
                self.metrics.mixnet.egress_sent_forward_packet(next_hop)
            }
        } else {
            self.metrics.mixnet.egress_sent_forward_packet(next_hop)
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
        self.forward_packet(delayed_packet);
    }

    /// Drain every packet whose release deadline has already passed (a burst of simultaneous
    /// releases), bounded by [`MAX_DRAIN_BATCH`]. `try_next_expired` never blocks, so this is a
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

    fn handle_new_packet(&mut self, mut new_packet: PacketToForward)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        // close out the ForwarderQueue stage (wait in the ingress -> forwarder channel)
        new_packet.trace.record(MixnetMetric::ForwarderQueue);

        let next_hop = new_packet.packet.next_hop();

        if !self
            .routing_filter
            .should_route(next_hop.as_ref().ip(), new_packet.network_monitor_packet)
        {
            warn!(
                event = "packet.dropped.routing_filter",
                next_hop = %next_hop,
                "dropping packet: egress address does not belong to any known node"
            );
            self.metrics
                .mixnet
                .egress_dropped_forward_packet(next_hop.into());
            return;
        }

        let delay_target = new_packet.forward_delay_target;
        let traced = Traced::new(new_packet.packet, new_packet.trace);

        // in case of a zero delay packet, don't bother putting it in the delay queue,
        // just forward it immediately
        let Some(instant) = delay_target else {
            self.forward_packet(traced);
            return;
        };

        // check if the delay has already expired, if so, don't bother putting it through the delay
        // queue only to retrieve it immediately. Just forward it.
        if instant.checked_duration_since(Instant::now()).is_none() {
            // the target elapsed before we could even queue it: upstream overhead already
            // ate the whole intended delay, so the overrun is now - target
            let overrun = Instant::now().saturating_duration_since(instant);
            traced.record_value(MixnetMetric::DelayQueueOverrun, overrun.as_secs_f64());
            self.forward_packet(traced);
        } else {
            self.delay_queue.insert_at(traced, instant);
        }
    }

    /// Drain every packet currently queued in the ingress channel, bounded by [`MAX_DRAIN_BATCH`]
    /// so the per-wakeup `select!`/waker/coop overhead is amortised across the burst rather than
    /// paid per packet. `try_recv` never blocks. Returns how many packets were handled.
    fn drain_ingress(&mut self) -> usize
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let mut batch_size = 0;
        while batch_size < MAX_DRAIN_BATCH {
            // Err = channel empty (or closed, which is unreachable since we hold a sender)
            let Ok(packet) = self.packet_receiver.try_recv() else {
                break;
            };
            self.handle_new_packet(packet);
            batch_size += 1;
        }
        batch_size
    }

    fn update_queue_len_metric(&self) {
        self.metrics
            .process
            .update_forward_hop_packets_being_delayed(self.delay_queue.len());
    }

    fn update_channel_size_metric(&self, channel_size: usize) {
        self.metrics
            .process
            .update_packet_forwarder_queue_size(channel_size)
    }

    /// Log the forwarder's queue depth at a severity reflecting how overloaded it is. Called
    /// periodically (~every 1000 packets), not per packet.
    fn log_queue_status(
        &self,
        channel_depth: usize,
        packets_processed: u64,
        last_drain_batch: usize,
    ) {
        let delay_queue_depth = self.delay_queue.len();
        match channel_depth {
            n if n > 1000 => error!(
                event = "forwarder.queue_overload",
                channel_depth = n,
                delay_queue_depth,
                packets_processed,
                last_drain_batch,
                "there are currently {n} mix packets waiting to get forwarded - the node seems to be significantly overloaded!"
            ),
            n if n > 500 => warn!(
                event = "forwarder.queue_high",
                channel_depth = n,
                delay_queue_depth,
                packets_processed,
                last_drain_batch,
                "there are currently {n} mix packets waiting to get forwarded - is the node overloaded?"
            ),
            n => trace!(
                channel_depth = n,
                delay_queue_depth, packets_processed, last_drain_batch, "forwarder queue status"
            ),
        }
    }

    pub async fn run(&mut self, shutdown_token: ShutdownToken)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let mut processed: u64 = 0;
        let mut last_logged: u64 = 0;
        trace!("starting PacketForwarder");
        loop {
            // packets handled this wakeup per branch; the select arm seeds the one it consumed
            let mut ingress = 0usize;
            let mut released = 0usize;
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("PacketForwarder: Received shutdown");
                    break;
                }
                new_packet = self.packet_receiver.next() => {
                    // impossible to panic: the struct holds a sender, so not all senders can drop
                    #[allow(clippy::unwrap_used)]
                    self.handle_new_packet(new_packet.unwrap());
                    ingress = 1;
                }
                delayed = self.delay_queue.next() => {
                    // SAFETY: `stream` implementation of `NonExhaustiveDelayQueue` never returns `None`
                    #[allow(clippy::unwrap_used)]
                    self.handle_done_delaying(delayed.unwrap());
                    released = 1;
                }
            }

            // service both branches every wakeup so neither can starve the other: drain queued
            // ingress packets first (routing + insert/forward, bringing the delay queue current),
            // then release everything now due
            ingress += self.drain_ingress();
            released += self.drain_expired();

            if ingress > 0 {
                observe_drain_batch_size(ingress);
                processed += ingress as u64;

                let channel_len = self.packet_sender.len();
                // log roughly every 1000 packets; `processed` advances in batches, so use a
                // crossing test rather than an exact modulo (which a batch could step over)
                if processed - last_logged >= 1000 {
                    last_logged = processed;
                    self.log_queue_status(channel_len, processed, ingress);
                }
                self.update_channel_size_metric(channel_len);
            }
            if released > 0 {
                observe_delay_drain_batch_size(released);
            }

            // update the metric on either a new packet being inserted or a packet being released
            self.update_queue_len_metric();
        }
        trace!("PacketForwarder: Exiting");
    }
}
