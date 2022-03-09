// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::node_statistics::UpdateSender;
use futures::channel::mpsc;
use futures::StreamExt;
use nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue, TimerError};
use nymsphinx::forwarding::packet::MixPacket;
use std::io;
use tokio::time::Instant;

// Delay + MixPacket vs Instant + MixPacket

// rather than using Duration directly, we use an Instant, this way we minimise skew due to
// time packet spent waiting in the queue to get delayed
pub(crate) type PacketDelayForwardSender = mpsc::UnboundedSender<(MixPacket, Option<Instant>)>;
type PacketDelayForwardReceiver = mpsc::UnboundedReceiver<(MixPacket, Option<Instant>)>;

/// Entity responsible for delaying received sphinx packet and forwarding it to next node.
pub(crate) struct DelayForwarder<C>
where
    C: mixnet_client::SendWithoutResponse,
{
    delay_queue: NonExhaustiveDelayQueue<MixPacket>,
    mixnet_client: C,
    packet_sender: PacketDelayForwardSender,
    packet_receiver: PacketDelayForwardReceiver,
    node_stats_update_sender: UpdateSender,
}

impl<C> DelayForwarder<C>
where
    C: mixnet_client::SendWithoutResponse,
{
    pub(crate) fn new(client: C, node_stats_update_sender: UpdateSender) -> DelayForwarder<C> {
        let (packet_sender, packet_receiver) = mpsc::unbounded();

        DelayForwarder::<C> {
            delay_queue: NonExhaustiveDelayQueue::new(),
            mixnet_client: client,
            packet_sender,
            packet_receiver,
            node_stats_update_sender,
        }
    }

    pub(crate) fn sender(&self) -> PacketDelayForwardSender {
        self.packet_sender.clone()
    }

    fn forward_packet(&mut self, packet: MixPacket) {
        let next_hop = packet.next_hop();
        let packet_mode = packet.packet_mode();
        let sphinx_packet = packet.into_sphinx_packet();

        if let Err(err) =
            self.mixnet_client
                .send_without_response(next_hop, sphinx_packet, packet_mode)
        {
            if err.kind() == io::ErrorKind::WouldBlock {
                // we only know for sure if we dropped a packet if our sending queue was full
                // in any other case the connection might still be re-established (or created for the first time)
                // and the packet might get sent, but we won't know about it
                self.node_stats_update_sender
                    .report_dropped(next_hop.to_string())
            } else if err.kind() == io::ErrorKind::NotConnected {
                // let's give the benefit of the doubt and assume we manage to establish connection
                self.node_stats_update_sender
                    .report_sent(next_hop.to_string());
            }
        } else {
            self.node_stats_update_sender
                .report_sent(next_hop.to_string());
        }
    }

    /// Upon packet being finished getting delayed, forward it to the mixnet.
    fn handle_done_delaying(&mut self, packet: Option<Result<Expired<MixPacket>, TimerError>>) {
        // those are critical errors that I don't think can be recovered from.
        let delayed = packet.expect("the queue has unexpectedly terminated!");
        let delayed_packet = delayed
            .expect("Encountered timer issue within the runtime!")
            .into_inner();

        self.forward_packet(delayed_packet)
    }

    fn handle_new_packet(&mut self, new_packet: (MixPacket, Option<Instant>)) {
        // in case of a zero delay packet, don't bother putting it in the delay queue,
        // just forward it immediately
        if let Some(instant) = new_packet.1 {
            // check if the delay has already expired, if so, don't bother putting it through
            // the delay queue only to retrieve it immediately. Just forward it.
            if instant.checked_duration_since(Instant::now()).is_none() {
                self.forward_packet(new_packet.0)
            } else {
                self.delay_queue.insert_at(new_packet.0, instant);
            }
        } else {
            self.forward_packet(new_packet.0)
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                delayed = self.delay_queue.next() => {
                    self.handle_done_delaying(delayed);
                }
                new_packet = self.packet_receiver.next() => {
                    // this one is impossible to ever panic - the object itself contains a sender
                    // and hence it can't happen that ALL senders are dropped
                    self.handle_new_packet(new_packet.unwrap())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
    use nymsphinx_params::packet_sizes::PacketSize;
    use nymsphinx_params::PacketMode;
    use nymsphinx_types::builder::SphinxPacketBuilder;
    use nymsphinx_types::{
        crypto, Delay as SphinxDelay, Destination, DestinationAddressBytes, Node, NodeAddressBytes,
        SphinxPacket, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, NODE_ADDRESS_LENGTH,
    };

    #[derive(Default)]
    struct TestClient {
        pub packets_sent: Arc<Mutex<Vec<(NymNodeRoutingAddress, SphinxPacket, PacketMode)>>>,
    }

    impl mixnet_client::SendWithoutResponse for TestClient {
        fn send_without_response(
            &mut self,
            address: NymNodeRoutingAddress,
            packet: SphinxPacket,
            packet_mode: PacketMode,
        ) -> io::Result<()> {
            self.packets_sent
                .lock()
                .unwrap()
                .push((address, packet, packet_mode));
            Ok(())
        }
    }

    fn make_valid_sphinx_packet(size: PacketSize) -> SphinxPacket {
        let (_, node1_pk) = crypto::keygen();
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
            node1_pk,
        );
        let (_, node2_pk) = crypto::keygen();
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
            node2_pk,
        );
        let (_, node3_pk) = crypto::keygen();
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            node3_pk,
        );

        let route = [node1, node2, node3];
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );
        let delays = vec![
            SphinxDelay::new_from_nanos(42),
            SphinxDelay::new_from_nanos(42),
            SphinxDelay::new_from_nanos(42),
        ];
        SphinxPacketBuilder::new()
            .with_payload_size(size.payload_size())
            .build_packet(b"foomp".to_vec(), &route, &destination, &delays)
            .unwrap()
    }

    #[tokio::test]
    async fn packets_received_are_forwarded() {
        // Wire up the DelayForwarder
        let (stats_sender, _stats_receiver) = mpsc::unbounded();
        let node_stats_update_sender = UpdateSender::new(stats_sender);
        let client = TestClient::default();
        let client_packets_sent = client.packets_sent.clone();
        let mut delay_forwarder = DelayForwarder::new(client, node_stats_update_sender);
        let packet_sender = delay_forwarder.sender();

        // Spawn the worker, listening on packet_sender channel
        tokio::spawn(async move { delay_forwarder.run().await });

        // Send a `MixPacket` down the channel without any delay attached.
        let next_hop =
            NymNodeRoutingAddress::from(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 42));
        let mix_packet = MixPacket::new(
            next_hop,
            make_valid_sphinx_packet(PacketSize::default()),
            PacketMode::default(),
        );
        let forward_instant = None;
        packet_sender
            .unbounded_send((mix_packet, forward_instant))
            .unwrap();

        // Give the the worker a chance to act
        tokio::time::sleep(Duration::from_millis(10)).await;

        // The client should have forwarded the packet straight away
        assert_eq!(
            client_packets_sent
                .lock()
                .unwrap()
                .iter()
                .map(|(a, _, _)| *a)
                .collect::<Vec<_>>(),
            vec![next_hop]
        );
    }
}
