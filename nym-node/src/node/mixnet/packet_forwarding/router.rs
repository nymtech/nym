// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{DelayedPacket, MAX_DRAIN_BATCH, forward_packet};
use crate::node::routing_filter::RoutingFilter;
use futures::StreamExt;
use nym_mixnet_client::SendWithoutResponse;
use nym_mixnet_client::forwarder::{MixForwardingReceiver, MixForwardingSender, PacketToForward};
use nym_mixnet_client::metrics::{MixnetMetric, Traced, observe_drain_batch_size};
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownToken;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

/// Intake task of the [`PacketForwarder`](super::PacketForwarder): drains the ingress channel,
/// applies the routing filter, and either forwards zero-delay packets immediately or hands delayed
/// ones to the [`DelayForwarder`](super::DelayForwarder). Per-packet work is sub-µs, so new packets
/// are never stuck behind delayed-release processing.
pub struct PacketRouter<C, F> {
    mixnet_client: Arc<C>,
    routing_filter: F,
    metrics: NymNodeMetrics,

    // a clone is kept here both to query the channel depth and to keep the ingress channel open
    // (so `packet_receiver.next()` never spuriously yields `None`)
    packet_sender: MixForwardingSender,
    packet_receiver: MixForwardingReceiver,

    delayed_sender: mpsc::Sender<DelayedPacket>,
}

impl<C, F> PacketRouter<C, F> {
    pub(super) fn new(
        mixnet_client: Arc<C>,
        routing_filter: F,
        metrics: NymNodeMetrics,
        packet_sender: MixForwardingSender,
        packet_receiver: MixForwardingReceiver,
        delayed_sender: mpsc::Sender<DelayedPacket>,
    ) -> Self {
        PacketRouter {
            mixnet_client,
            routing_filter,
            metrics,
            packet_sender,
            packet_receiver,
            delayed_sender,
        }
    }

    pub(super) fn sender(&self) -> MixForwardingSender {
        self.packet_sender.clone()
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

        // in case of a zero delay packet, don't bother handing it to the delay task,
        // just forward it immediately
        let Some(instant) = delay_target else {
            forward_packet(&*self.mixnet_client, &self.metrics, traced);
            return;
        };

        // check if the delay has already expired, if so, don't bother going through the delay
        // task only to retrieve it immediately. Just forward it.
        if instant.checked_duration_since(Instant::now()).is_none() {
            // the target elapsed before we could even queue it: upstream overhead already
            // ate the whole intended delay, so the overrun is now - target
            let overrun = Instant::now().saturating_duration_since(instant);
            traced.record_value(MixnetMetric::DelayQueueOverrun, overrun.as_secs_f64());
            forward_packet(&*self.mixnet_client, &self.metrics, traced);
            return;
        }

        // hand off to the dedicated delay task; the DelayQueue stage runs from here until release
        let dispatched = self.delayed_sender.try_send(DelayedPacket {
            packet: traced,
            release_at: instant,
        });
        if let Err(err) = dispatched {
            match err {
                TrySendError::Full(_) => {
                    warn!(
                        event = "packet.dropped.delay_handoff_full",
                        next_hop = %next_hop,
                        "dropping packet: delay-forwarder handoff channel is full"
                    );
                    self.metrics
                        .mixnet
                        .egress_dropped_forward_packet(next_hop.into());
                }
                TrySendError::Closed(_) => {
                    debug!("delay forwarder has gone away; dropping packet during shutdown");
                }
            }
        }
    }

    /// Handle the just-received `first` ingress packet, then drain any others already queued,
    /// bounded by [`MAX_DRAIN_BATCH`], so the per-wakeup `select!`/waker/coop overhead is amortised
    /// across the burst rather than paid per packet. `try_recv` never blocks - we fall back to the
    /// idle `.next().await` in `run` once the channel empties. Returns how many packets were handled.
    fn drain_ingress(&mut self, first: PacketToForward) -> usize
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        self.handle_new_packet(first);
        let mut batch_size = 1;
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

    fn update_channel_size_metric(&self, channel_size: usize) {
        self.metrics
            .process
            .update_packet_forwarder_queue_size(channel_size)
    }

    /// Log the ingress queue depth at a severity reflecting how overloaded it is. Called
    /// periodically (~every 1000 packets), not per packet.
    fn log_queue_status(
        &self,
        channel_depth: usize,
        packets_processed: u64,
        last_drain_batch: usize,
    ) {
        match channel_depth {
            n if n > 1000 => error!(
                event = "forwarder.queue_overload",
                channel_depth = n,
                packets_processed,
                last_drain_batch,
                "there are currently {n} mix packets waiting to get forwarded - the node seems to be significantly overloaded!"
            ),
            n if n > 500 => warn!(
                event = "forwarder.queue_high",
                channel_depth = n,
                packets_processed,
                last_drain_batch,
                "there are currently {n} mix packets waiting to get forwarded - is the node overloaded?"
            ),
            n => trace!(
                channel_depth = n,
                packets_processed, last_drain_batch, "forwarder queue status"
            ),
        }
    }

    pub async fn run(mut self, shutdown_token: ShutdownToken)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let mut processed: u64 = 0;
        let mut last_logged: u64 = 0;
        trace!("starting PacketRouter");
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("PacketRouter: Received shutdown");
                    break;
                }
                new_packet = self.packet_receiver.next() => {
                    // impossible to panic: the struct holds a sender, so not all senders can drop
                    #[allow(clippy::unwrap_used)]
                    let batch_size = self.drain_ingress(new_packet.unwrap());
                    observe_drain_batch_size(batch_size);
                    processed += batch_size as u64;

                    let channel_len = self.packet_sender.len();
                    // log roughly every 1000 packets; `processed` advances in batches, so use a
                    // crossing test rather than an exact modulo (which a batch could step over)
                    if processed - last_logged >= 1000 {
                        last_logged = processed;
                        self.log_queue_status(channel_len, processed, batch_size);
                    }
                    self.update_channel_size_metric(channel_len);
                }
            }
        }
        trace!("PacketRouter: Exiting");
    }
}
