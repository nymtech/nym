// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::{error, trace, warn};
use nym_sphinx::addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nym_sphinx::params::PacketSize;

pub trait GatewayPacketRouter {
    type Error: std::error::Error;

    fn route_received(&self, unwrapped_packets: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        let mut received_messages = Vec::new();
        let mut received_acks = Vec::new();

        // remember: gateway removes final layer of sphinx encryption and from the unwrapped
        // data he takes the SURB-ACK and first hop address.
        // currently SURB-ACKs are attached in EVERY packet, even cover, so this is always true
        let sphinx_ack_overhead = PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;
        let outfox_ack_overhead =
            PacketSize::OutfoxAckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;

        for received_packet in unwrapped_packets {
            // note: if we ever fail to route regular outfox, it might be because I've removed a match on
            // `size == PacketSize::OutfoxRegularPacket.size() - outfox_ack_overhead` since it seemed
            // redundant given we have `size == PacketSize::OutfoxRegularPacket.plaintext_size() - outfox_ack_overhead`
            // and all the headers should have already be stripped at this point
            match received_packet.len() {
                n if n == PacketSize::AckPacket.plaintext_size() => {
                    trace!("received sphinx ack");
                    received_acks.push(received_packet);
                }

                n if n <= PacketSize::OutfoxAckPacket.plaintext_size() => {
                    // we don't know the real size of the payload, it could be anything <= 48 bytes
                    trace!("received outfox ack");
                    received_acks.push(received_packet);
                }

                n if n == PacketSize::RegularPacket.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received regular sphinx packet");
                    received_messages.push(received_packet);
                }

                n if n
                    == PacketSize::OutfoxRegularPacket
                        .plaintext_size()
                        .saturating_sub(outfox_ack_overhead) =>
                {
                    trace!("received regular outfox packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket8.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended8 packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket16.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended16 packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket32.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended32 packet");
                    received_messages.push(received_packet);
                }

                n => {
                    // this can happen if other clients are not padding their messages
                    warn!("Received message of unexpected size. Probably from an outdated client... len: {n}");
                    received_messages.push(received_packet);
                }
            }
        }

        if !received_messages.is_empty() {
            trace!("routing {} received packets", received_messages.len());
            if let Err(err) = self.route_mixnet_messages(received_messages) {
                error!("failed to route received messages: {err}");
                return Err(err);
            }
        }

        if !received_acks.is_empty() {
            trace!("routing {} received acks", received_acks.len());
            if let Err(err) = self.route_acks(received_acks) {
                error!("failed to route received acks: {err}");
                return Err(err);
            }
        }

        Ok(())
    }

    fn route_mixnet_messages(&self, received_messages: Vec<Vec<u8>>) -> Result<(), Self::Error>;

    fn route_acks(&self, received_acks: Vec<Vec<u8>>) -> Result<(), Self::Error>;
}
