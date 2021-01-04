// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::params::PacketMode;
use nymsphinx::{
    acknowledgements::AckKey, addressing::clients::Recipient, preparer::MessagePreparer,
};
use rand::rngs::OsRng;
use std::time::Duration;
use topology::NymTopology;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

pub(crate) struct Chunker {
    rng: OsRng,
    me: Recipient,
    message_preparer: MessagePreparer<OsRng>,
}

impl Chunker {
    pub(crate) fn new(me: Recipient) -> Self {
        Chunker {
            rng: OsRng,
            me,
            message_preparer: MessagePreparer::new(
                OsRng,
                me,
                DEFAULT_AVERAGE_PACKET_DELAY,
                DEFAULT_AVERAGE_ACK_DELAY,
                PacketMode::Mix,
                None,
            ),
        }
    }

    pub(crate) async fn prepare_messages(
        &mut self,
        message: Vec<u8>,
        topology: &NymTopology,
    ) -> Vec<MixPacket> {
        let ack_key: AckKey = AckKey::new(&mut self.rng);

        let (split_message, _reply_keys) = self
            .message_preparer
            .prepare_and_split_message(message, false, &topology)
            .expect("failed to split the message");

        let mut mix_packets = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // don't bother with acks etc. for time being
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(message_chunk, &topology, &ack_key, &self.me)
                .await
                .unwrap();

            mix_packets.push(prepared_fragment.mix_packet);
        }
        mix_packets
    }
}
