// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::routing_filter::RoutingFilter;
use futures::StreamExt;
use nym_mixnet_client::SendWithoutResponse;
use nym_mixnet_client::forwarder::{
    MixForwardingReceiver, MixForwardingSender, PacketToForward, mix_forwarding_channels,
};
use nym_mixnet_client::metrics::{MixnetMetric, Traced, observe_drain_batch_size};
use nym_node_metrics::NymNodeMetrics;
use nym_nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::io;
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

/// Max ingress packets handled per `select!` wakeup before yielding back to the biased select
/// (so the delay queue + shutdown get serviced). Per-packet work in `handle_new_packet` is sub-µs
/// to low-µs, so 256 bounds the worst-case stall before those branches are re-checked to <~1ms.
const MAX_DRAIN_BATCH: usize = 256;

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
        F: RoutingFilter,
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
        F: RoutingFilter,
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
        if let Some(instant) = delay_target {
            // check if the delay has already expired, if so, don't bother putting it through
            // the delay queue only to retrieve it immediately. Just forward it.
            if instant.checked_duration_since(Instant::now()).is_none() {
                // the target elapsed before we could even queue it: upstream overhead already
                // ate the whole intended delay, so the overrun is now - target
                let overrun = Instant::now().saturating_duration_since(instant);
                traced.record_value(MixnetMetric::DelayQueueOverrun, overrun.as_secs_f64());
                self.forward_packet(traced)
            } else {
                self.delay_queue.insert_at(traced, instant);
            }
        } else {
            self.forward_packet(traced)
        }
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

    pub async fn run(&mut self, shutdown_token: ShutdownToken)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let mut processed: u64 = 0;
        let mut last_logged: u64 = 0;
        trace!("starting PacketForwarder");
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("PacketForwarder: Received shutdown");
                    break;
                }
                delayed = self.delay_queue.next() => {
                    // SAFETY: `stream` implementation of `NonExhaustiveDelayQueue` never returns `None`
                    #[allow(clippy::unwrap_used)]
                    self.handle_done_delaying(delayed.unwrap());
                }
                new_packet = self.packet_receiver.next() => {
                    // this one is impossible to ever panic - the struct itself contains a sender
                    // and hence it can't happen that ALL senders are dropped
                    #[allow(clippy::unwrap_used)]
                    self.handle_new_packet(new_packet.unwrap());

                    // drain whatever else is already queued (bounded by MAX_DRAIN_BATCH) so the
                    // per-wakeup select!/waker/coop overhead is amortised across the burst rather
                    // than paid per packet. `.next().await` above still parks us when idle;
                    // `try_recv()` only consumes already-queued packets and never blocks.
                    let mut batch_size = 1usize;
                    while batch_size < MAX_DRAIN_BATCH {
                        // Err = empty (or closed, unreachable since we hold a sender) -> stop
                        // draining and fall back to the idle `.next().await`
                        let Ok(packet) = self.packet_receiver.try_recv() else {
                            break;
                        };
                        self.handle_new_packet(packet);
                        batch_size += 1;
                    }
                    observe_drain_batch_size(batch_size);
                    processed += batch_size as u64;

                    let channel_len = self.packet_sender.len();
                    let delay_queue_len = self.delay_queue.len();
                    // log roughly every 1000 packets; `processed` now advances in batches, so use
                    // a crossing test rather than an exact modulo (which a batch could step over)
                    if processed - last_logged >= 1000 {
                        last_logged = processed;
                        match channel_len {
                            n if n > 1000 => error!(
                                event = "forwarder.queue_overload",
                                channel_depth = n,
                                delay_queue_depth = delay_queue_len,
                                packets_processed = processed,
                                last_drain_batch = batch_size,
                                "there are currently {n} mix packets waiting to get forwarded - the node seems to be significantly overloaded!"
                            ),
                            n if n > 500 => warn!(
                                event = "forwarder.queue_high",
                                channel_depth = n,
                                delay_queue_depth = delay_queue_len,
                                packets_processed = processed,
                                last_drain_batch = batch_size,
                                "there are currently {n} mix packets waiting to get forwarded - is the node overloaded?"
                            ),
                            n => trace!(
                                channel_depth = n,
                                delay_queue_depth = delay_queue_len,
                                packets_processed = processed,
                                last_drain_batch = batch_size,
                                "forwarder queue status"
                            ),
                        }
                    }
                    self.update_channel_size_metric(channel_len);
                }
            }

            // update the metrics on either new packet being inserted or packet being removed
            self.update_queue_len_metric();
        }
        trace!("PacketForwarder: Exiting");
    }
}
