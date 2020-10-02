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

use nymsphinx::{
    acknowledgements::AckKey,
    addressing::{clients::Recipient, nodes::NymNodeRoutingAddress},
    preparer::MessagePreparer,
    SphinxPacket,
};
use rand::rngs::OsRng;
use std::time::Duration;
use topology::NymTopology;

const DEFAULT_RNG: OsRng = OsRng;
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

pub fn prepare_messages(
    message: String,
    me: Recipient,
    topology: &NymTopology,
) -> Vec<(NymNodeRoutingAddress, SphinxPacket)> {
    let message_bytes = message.into_bytes();

    let mut message_preparer = MessagePreparer::new(
        DEFAULT_RNG,
        me,
        DEFAULT_AVERAGE_PACKET_DELAY,
        DEFAULT_AVERAGE_ACK_DELAY,
    );

    let ack_key: AckKey = AckKey::new(&mut DEFAULT_RNG);

    let (split_message, _reply_keys) = message_preparer
        .prepare_and_split_message(message_bytes, false, &topology)
        .expect("failed to split the message");

    let mut socket_messages = Vec::with_capacity(split_message.len());
    for message_chunk in split_message {
        // don't bother with acks etc. for time being
        let prepared_fragment = message_preparer
            .prepare_chunk_for_sending(message_chunk, &topology, &ack_key, &me) //2 was  &self.ack_key
            .unwrap();

        socket_messages.push((
            prepared_fragment.first_hop_address,
            prepared_fragment.sphinx_packet,
        ));
    }
    socket_messages
}
