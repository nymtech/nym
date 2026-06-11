// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_mixnet_client::SendWithoutResponse;
use nym_mixnet_client::forwarder::{MixForwardingSender, mix_forwarding_channels};
use nym_mixnet_client::metrics::Traced;
use nym_node_metrics::NymNodeMetrics;
use nym_sphinx_forwarding::packet::MixPacket;
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, trace};

mod delay;
mod router;

pub use delay::DelayForwarder;
pub use router::PacketRouter;

/// Max packets handled per `select!` wakeup, per drainable branch, before yielding back to the
/// biased select so shutdown and the other branches stay responsive. Per-packet work is sub-µs to
/// low-µs, so 256 bounds the worst-case stall to <~1ms.
const MAX_DRAIN_BATCH: usize = 256;

/// Capacity of the router -> delay-forwarder handoff channel. It only holds packets in transit
/// (dispatched by the router, not yet inserted into the delay queue), which the delay task drains
/// promptly, so steady-state occupancy is a handful; this bound exists purely to cap memory if the
/// delay task is momentarily descheduled. On overflow the router drops (counted as an egress drop)
/// rather than block, keeping intake decoupled from release.
const DELAYED_HANDOFF_CAPACITY: usize = 2048;

/// A packet handed from the [`PacketRouter`] to the [`DelayForwarder`] to be held until its
/// (Poisson) release instant.
struct DelayedPacket {
    packet: Traced<MixPacket>,
    release_at: Instant,
}

/// Forward a (released or zero-delay) packet to the next hop via the mixnet client, recording the
/// egress send/drop metric. Shared by both tasks: the router forwards zero/already-elapsed-delay
/// packets directly, the delay task forwards on release.
fn forward_packet<C: SendWithoutResponse>(
    mixnet_client: &C,
    metrics: &NymNodeMetrics,
    packet: Traced<MixPacket>,
) {
    let next_hop = packet.inner.next_hop_address();

    if let Err(err) = mixnet_client.send_without_response(packet) {
        if err.kind() == io::ErrorKind::WouldBlock {
            // we only know for sure if we dropped a packet if our sending queue was full
            trace!(
                event = "packet.dropped.buffer_full",
                next_hop = %next_hop,
                "dropping packet: egress connection buffer full (WouldBlock)"
            );
            metrics.mixnet.egress_dropped_forward_packet(next_hop)
        } else if err.kind() == io::ErrorKind::NotConnected {
            debug!(
                next_hop = %next_hop,
                "packet queued for not-yet-connected peer"
            );
            metrics.mixnet.egress_sent_forward_packet(next_hop)
        }
    } else {
        metrics.mixnet.egress_sent_forward_packet(next_hop)
    }
}

/// The node's forward-hop egress engine - the last in-node stage of the mixnet pipeline, split
/// across two concurrently-scheduled tasks so neither blocks the other:
///
/// 1. [`PacketRouter`] (`router`) - the intake task. It is the sole consumer of the
///    ingress-to-forwarder channel: every forward-hop packet (plus acks) arrives here. Per packet
///    it applies the routing filter, then either forwards it immediately (zero/already-elapsed
///    delay) or hands it to the delay task over a bounded channel. Its work is sub-µs, so new
///    packets are never stuck behind delayed-release processing.
/// 2. [`DelayForwarder`] (`delay`) - the dedicated [`NonExhaustiveDelayQueue`] task. It owns the
///    delay queue exclusively (the queue can't be shared by reference), receiving insertions from
///    the router and forwarding each packet to the next hop once its release instant passes.
///
/// Splitting intake from release decouples the `ForwarderQueue` latency (ingress wait) from the
/// `DelayQueue`/`DelayQueueOverrun` latency (release timing): a burst of simultaneous releases no
/// longer delays new-packet intake, and vice versa. Both tasks share the mixnet client (`C`) and
/// stamp the `mixnet_packet_*` latency-trace stages they own.
///
/// This is just the builder: it wires the channels and hands out the ingress [`sender`](Self::sender),
/// then [`into_tasks`](Self::into_tasks) yields the two runnables to spawn.
///
/// [`NonExhaustiveDelayQueue`]: nym_nonexhaustive_delayqueue::NonExhaustiveDelayQueue
pub struct PacketForwarder<C, F> {
    router: PacketRouter<C, F>,
    delay_forwarder: DelayForwarder<C>,
}

impl<C, F> PacketForwarder<C, F> {
    pub fn new(client: C, routing_filter: F, metrics: NymNodeMetrics) -> Self {
        let (packet_sender, packet_receiver) = mix_forwarding_channels();
        let (delayed_sender, delayed_receiver) = mpsc::channel(DELAYED_HANDOFF_CAPACITY);
        let mixnet_client = Arc::new(client);

        let router = PacketRouter::new(
            Arc::clone(&mixnet_client),
            routing_filter,
            metrics.clone(),
            packet_sender,
            packet_receiver,
            delayed_sender,
        );
        let delay_forwarder = DelayForwarder::new(mixnet_client, metrics, delayed_receiver);

        PacketForwarder {
            router,
            delay_forwarder,
        }
    }

    pub fn sender(&self) -> MixForwardingSender {
        self.router.sender()
    }

    /// Consume the builder, yielding the two tasks to spawn: (router, delay-forwarder).
    pub fn into_tasks(self) -> (PacketRouter<C, F>, DelayForwarder<C>) {
        (self.router, self.delay_forwarder)
    }
}
