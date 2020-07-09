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

use nymsphinx_acknowledgements::AckAes128Key;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_chunking::MessageChunker;
use nymsphinx_types::{Delay, SphinxPacket};
use rand::{CryptoRng, Rng};
use std::sync::Arc;
use topology::NymTopology;

pub struct PreparedMessage {
    /// Indicates the total expected round-trip time, i.e. delay from the sending of this message
    /// until receiving the acknowledgement included inside of it.
    total_delay: Delay,

    /// Indicates address of the node to which the message should be sent.
    first_hop_address: NymNodeRoutingAddress,

    /// The actual 'chunk' of the message that is going to go through the mix network.
    sphinx_packet: SphinxPacket,
}

#[derive(Debug)]
pub enum PreparationError {}

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
pub struct MessagePreparer<R: CryptoRng + Rng> {
    chunker: MessageChunker<R>,
    ack_key: Arc<AckAes128Key>,
}

impl<R> MessagePreparer<R>
where
    R: CryptoRng + Rng,
{
    fn pad_message(&self, message: Vec<u8>) -> Vec<u8> {
        todo!()
    }

    fn shared_key() {}

    fn attach_reply_surb(&self, message: Vec<u8>) -> Vec<u8> {
        todo!()
    }

    fn split_message(&self, message: Vec<u8>) {
        todo!()
    }

    pub fn prepare_message(
        &self,
        message: Vec<u8>,
        recipient: &Recipient, // TODO: MUST INCLUDE THEIR ENCRYPTION KEY
        with_reply_surb: bool,
        topology: &NymTopology,
    ) -> Result<Vec<PreparedMessage>, PreparationError> {
        /*
        0. let message = (if surb { 1 || SURB } else { 0 } ) || message || padding
        1. split message leaving 32 bytes free in each fragment
        2. For each Fragment:
            - generate (x, g^x)
            - compute k = KDF(g^x * their encryption key)
            - compute v_b = AES(k, Fragment)
            - compute vk_b = g^x || v_b
            - compute sphinx = Sphinx(recipient, vk_b) // surb-acks go somewhere here
        3. return Vec<PreparedMessage>
        */

        todo!()
    }
}

/*
   And for completion reconstruction:
   1. receive unwrapped sphinx packet: g^x || v_b
   2. recompute k = KDF(g^x * our encryption key)
   3. original_fragment = AES(k, v_b)
   4. deal with fragment as before
   5. on full message reconstruction output (message, Option<reply_surb>)
*/
