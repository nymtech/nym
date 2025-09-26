// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::routing_filter::RoutingFilter;
use futures::StreamExt;
use nym_mixnet_client::forwarder::{
    mix_forwarding_channels, MixForwardingReceiver, MixForwardingSender, PacketToForward,
};
use nym_mixnet_client::SendWithoutResponse;
use nym_node_metrics::NymNodeMetrics;
use nym_nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::io;
use tokio::time::Instant;
use tracing::{debug, error, instrument, trace, warn};

pub(crate) mod global;

pub struct PacketForwarder<C, F> {
    delay_queue: NonExhaustiveDelayQueue<MixPacket>,
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

    #[instrument(skip_all)]
    fn forward_packet(&mut self, packet: MixPacket)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let next_hop = packet.next_hop_address();

        if let Err(err) = self.mixnet_client.send_without_response(packet) {
            if err.kind() == io::ErrorKind::WouldBlock {
                // we only know for sure if we dropped a packet if our sending queue was full
                // in any other case the connection might still be re-established (or created for the first time)
                // and the packet might get sent, but we won't know about it
                self.metrics.mixnet.egress_dropped_forward_packet(next_hop)
            } else if err.kind() == io::ErrorKind::NotConnected {
                // let's give the benefit of the doubt and assume we manage to establish connection
                self.metrics.mixnet.egress_sent_forward_packet(next_hop)
            }
        } else {
            self.metrics.mixnet.egress_sent_forward_packet(next_hop)
        }
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    fn handle_done_delaying(&mut self, packet: Expired<MixPacket>)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let delayed_packet = packet.into_inner();
        self.forward_packet(delayed_packet);
    }

    #[instrument(skip_all)]
    fn handle_new_packet(&mut self, new_packet: PacketToForward)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let next_hop = new_packet.packet.next_hop();

        if !self.routing_filter.should_route(next_hop.as_ref().ip()) {
            debug!("dropping packet as the egress address does not belong to any known node");
            self.metrics
                .mixnet
                .egress_dropped_forward_packet(next_hop.into());
            return;
        }

        // in case of a zero delay packet, don't bother putting it in the delay queue,
        // just forward it immediately
        if let Some(instant) = new_packet.forward_delay_target {
            // check if the delay has already expired, if so, don't bother putting it through
            // the delay queue only to retrieve it immediately. Just forward it.
            if instant.checked_duration_since(Instant::now()).is_none() {
                self.forward_packet(new_packet.packet)
            } else {
                self.delay_queue.insert_at(new_packet.packet, instant);
            }
        } else {
            self.forward_packet(new_packet.packet)
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

    #[instrument(skip_all)]
    pub async fn run(&mut self, shutdown_token: ShutdownToken)
    where
        C: SendWithoutResponse,
        F: RoutingFilter,
    {
        let mut processed = 0;
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
                    let channel_len = self.packet_sender.len();
                    if processed % 1000 == 0 {
                        match channel_len {
                            n if n > 1000 => error!("there are currently {n} mix packets waiting to get forwarded - the node seems to be significantly overloaded!"),
                            n if n > 500 => warn!("there are currently {n} mix packets waiting to get forwarded - is the node overloaded?"),
                            n => trace!("there are currently {n} mix packets waiting to get forwarded"),
                        }
                    }
                    self.update_channel_size_metric(channel_len);
                    processed += 1;
                }
            }

            // update the metrics on either new packet being inserted or packet being removed
            self.update_queue_len_metric();
        }
        trace!("PacketForwarder: Exiting");
    }
}
