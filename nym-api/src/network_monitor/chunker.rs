// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_sphinx::{
    acknowledgements::AckKey, addressing::clients::Recipient, preparer::MessagePreparer,
};
use nym_topology::NymTopology;
use rand_07::rngs::OsRng;
use std::time::Duration;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub(crate) struct Chunker {
    rng: OsRng,
    packet_size: PacketSize,
    message_preparer: MessagePreparer<OsRng>,
}

impl Chunker {
    pub(crate) fn new(tested_mix_me: Recipient) -> Self {
        Chunker {
            rng: OsRng,
            // no point in using anything else for monitoring
            // unless we should make it variable so mixnodes wouldn't know if
            // non-default packet is for measurement or not
            packet_size: PacketSize::RegularPacket,
            message_preparer: MessagePreparer::new(
                OsRng,
                tested_mix_me,
                DEFAULT_AVERAGE_PACKET_DELAY,
                DEFAULT_AVERAGE_ACK_DELAY,
            ),
        }
    }

    pub(crate) fn prepare_packets_from(
        &mut self,
        message: Vec<u8>,
        topology: &NymTopology,
        packet_sender: Recipient,
        packet_type: PacketType,
    ) -> Vec<MixPacket> {
        // I really dislike how we have to overwrite the parameter of the `MessagePreparer` on each run
        // but without some significant API changes in the `MessagePreparer` this was the easiest
        // way to being able to have variable sender address.
        self.message_preparer.set_sender_address(packet_sender);
        self.prepare_packets(message, topology, packet_sender, packet_type)
    }

    fn prepare_packets(
        &mut self,
        message: Vec<u8>,
        topology: &NymTopology,
        packet_sender: Recipient,
        packet_type: PacketType,
    ) -> Vec<MixPacket> {
        let ack_key: AckKey = AckKey::new(&mut self.rng);

        let split_message = self
            .message_preparer
            .pad_and_split_message(NymMessage::new_plain(message), self.packet_size);

        let mut mix_packets = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // don't bother with acks etc. for time being
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(
                    message_chunk,
                    topology,
                    &ack_key,
                    &packet_sender,
                    packet_type,
                )
                .unwrap();

            mix_packets.push(prepared_fragment.mix_packet);
        }
        mix_packets
    }
}
