// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nym_mixnet_client::forwarder::{
    mix_forwarding_channels, MixForwardingReceiver, MixForwardingSender, PacketToForward,
};
use nym_mixnet_client::SendWithoutResponse;
use nym_node_metrics::NymNodeMetrics;
use nym_nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue};
use nym_sphinx_forwarding::packet::MixPacket;
use std::io;
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

pub struct PacketForwarder<C> {
    delay_queue: NonExhaustiveDelayQueue<MixPacket>,
    mixnet_client: C,

    metrics: NymNodeMetrics,

    packet_sender: MixForwardingSender,
    packet_receiver: MixForwardingReceiver,
    shutdown: nym_task::TaskClient,
}

impl<C> PacketForwarder<C> {
    pub fn new(client: C, metrics: NymNodeMetrics, shutdown: nym_task::TaskClient) -> Self {
        let (packet_sender, packet_receiver) = mix_forwarding_channels();

        PacketForwarder {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client: client,
            metrics,
            packet_sender,
            packet_receiver,
            shutdown,
        }
    }

    pub fn sender(&self) -> MixForwardingSender {
        self.packet_sender.clone()
    }

    fn forward_packet(&mut self, packet: MixPacket)
    where
        C: SendWithoutResponse,
    {
        let next_hop = packet.next_hop();
        let packet_type = packet.packet_type();
        let packet = packet.into_packet();

        if let Err(err) = self
            .mixnet_client
            .send_without_response(next_hop, packet, packet_type)
        {
            if err.kind() == io::ErrorKind::WouldBlock {
                // we only know for sure if we dropped a packet if our sending queue was full
                // in any other case the connection might still be re-established (or created for the first time)
                // and the packet might get sent, but we won't know about it
                self.metrics
                    .mixnet
                    .egress_dropped_forward_packet(next_hop.into())
            } else if err.kind() == io::ErrorKind::NotConnected {
                // let's give the benefit of the doubt and assume we manage to establish connection
                self.metrics
                    .mixnet
                    .egress_sent_forward_packet(next_hop.into())
            }
        } else {
            self.metrics
                .mixnet
                .egress_sent_forward_packet(next_hop.into())
        }
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    fn handle_done_delaying(&mut self, packet: Expired<MixPacket>)
    where
        C: SendWithoutResponse,
    {
        let delayed_packet = packet.into_inner();
        self.forward_packet(delayed_packet)
    }

    fn handle_new_packet(&mut self, new_packet: PacketToForward)
    where
        C: SendWithoutResponse,
    {
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

    pub async fn run(&mut self)
    where
        C: SendWithoutResponse,
    {
        let mut processed = 0;
        trace!("starting PacketForwarder");
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
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
                    if processed % 1000 == 0 {
                        let queue_len = self.packet_sender.len();
                        match queue_len {
                            n if n > 200 => error!("there are currently {n} mix packets waiting to get forwarded!"),
                            n if n > 50 => warn!("there are currently {n} mix packets waiting to get forwarded"),
                            n => trace!("there are currently {n} mix packets waiting to get forwarded"),
                        }
                    }

                    processed += 1;
                }
            }
        }
        trace!("PacketForwarder: Exiting");
    }
}
