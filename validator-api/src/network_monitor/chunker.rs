// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::{
    acknowledgements::AckKey, addressing::clients::Recipient, preparer::MessagePreparer,
};
use rand_07::rngs::OsRng;
use std::time::Duration;
use topology::NymTopology;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

pub(crate) struct Chunker {
    rng: OsRng,
    message_preparer: MessagePreparer<OsRng>,
}

impl Chunker {
    pub(crate) fn new(tested_mix_me: Recipient) -> Self {
        Chunker {
            rng: OsRng,
            message_preparer: MessagePreparer::new(
                OsRng,
                tested_mix_me,
                DEFAULT_AVERAGE_PACKET_DELAY,
                DEFAULT_AVERAGE_ACK_DELAY,
            ),
        }
    }

    pub(crate) async fn prepare_packets_from(
        &mut self,
        message: Vec<u8>,
        topology: &NymTopology,
        packet_sender: Recipient,
    ) -> Vec<MixPacket> {
        // I really dislike how we have to overwrite the parameter of the `MessagePreparer` on each run
        // but without some significant API changes in the `MessagePreparer` this was the easiest
        // way to being able to have variable sender address.
        self.message_preparer.set_sender_address(packet_sender);
        self.prepare_packets(message, topology, packet_sender).await
    }

    async fn prepare_packets(
        &mut self,
        message: Vec<u8>,
        topology: &NymTopology,
        packet_sender: Recipient,
    ) -> Vec<MixPacket> {
        let ack_key: AckKey = AckKey::new(&mut self.rng);

        let (split_message, _reply_keys) = self
            .message_preparer
            .prepare_and_split_message(message, false, topology)
            .expect("failed to split the message");

        let mut mix_packets = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // don't bother with acks etc. for time being
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(message_chunk, topology, &ack_key, &packet_sender)
                .await
                .unwrap();

            mix_packets.push(prepared_fragment.mix_packet);
        }
        mix_packets
    }
}
