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

use crate::set::split_into_sets;
use nymsphinx_types::SphinxPacket;
use packet_sizes::PacketSize;
use topology::NymTopology;

pub mod fragment;
pub mod packet_sizes;
pub mod reconstruction;
pub mod set;

/// The idea behind the process of chunking is to incur as little data overhead as possible due
/// to very computationally costly sphinx encapsulation procedure.
///
/// To achieve this, the underlying message is split into so-called "sets", which are further
/// subdivided into the base unit of "fragment" that is directly encapsulated by a Sphinx packet.
/// This allows to encapsulate messages of arbitrary length.
///
/// Each message, regardless of its size, consists of at least a single `Set` that has at least
/// a single `Fragment`.
///
/// Each `Fragment` can have variable, yet fully deterministic, length,
/// that depends on its position in the set as well as total number of sets. This is further
/// explained in `fragment.rs` file.  
///
/// Similarly, each `Set` can have a variable number of `Fragment`s inside. However, that
/// value is more restrictive: if it's the last set into which the message was split
/// (or implicitly the only one), it has no lower bound on the number of `Fragment`s.
/// (Apart from the restriction of containing at least a single one). If the set is located
/// somewhere in the middle, *it must be* full. Finally, regardless of its position, it must also be
/// true that it contains no more than `u8::max_value()`, i.e. 255 `Fragment`s.
/// Again, the reasoning for this is further explained in `set.rs` file. However, you might
/// also want to look at `fragment.rs` to understand the full context behind that design choice.
///
/// Both of those concepts as well as their structures, i.e. `Set` and `Fragment`
/// are further explained in the respective files.

#[derive(PartialEq, Debug)]
pub enum ChunkingError {
    InvalidPayloadLengthError,
    TooBigMessageToSplit,
    MalformedHeaderError,
    NoValidProvidersError,
    NoValidRoutesAvailableError,
    InvalidTopologyError,
    TooShortFragmentData,
    MalformedFragmentData,
    UnexpectedFragmentCount,
}

pub struct MessageChunker {
    packet_size: PacketSize,
    reply_surbs: bool,
    surb_acks: bool,
}

impl MessageChunker {
    pub fn new() -> Self {
        Default::default()
    }

    fn available_plaintext_size(&self) -> usize {
        let mut available_size = self.packet_size.plaintext_size();
        if self.surb_acks {
            available_size -= PacketSize::ACKPacket.size()
        }
        if self.reply_surbs {
            // TODO
        }
        available_size
    }

    pub fn with_reply_surbs(mut self, reply_surbs: bool) -> Self {
        self.reply_surbs = reply_surbs;
        self
    }

    pub fn with_surb_acks(mut self, surb_acks: bool) -> Self {
        self.surb_acks = surb_acks;
        self
    }

    pub fn with_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.packet_size = packet_size;
        self
    }

    pub fn finalize() -> Vec<Vec<u8>> {
        todo!()
    }

    pub fn attach_surb_acks() {}

    pub fn attach_reply_surbs() {}

    // while we could have gotten around by not passing topology in the previous implementation,
    // it's really difficult to not do it here, as we need to construct the SURB-ACK and later the
    // reply-SURB. If we let it for the callee, it would have introduced a lot of extra complexity
    pub fn split_message<T: NymTopology>(
        message: &[u8],
        topology: &T, // TODO: see what happens if we change `&T` to just `T`
    ) -> Vec<SphinxPacket> {
        todo!()
    }
}

impl Default for MessageChunker {
    fn default() -> Self {
        MessageChunker {
            packet_size: Default::default(),
            reply_surbs: false,
            surb_acks: true,
        }
    }
}

/// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
/// directly. After receiving they can be combined using `reconstruction::MessageReconstructor`
/// to obtain the original message back.
/// `available_packet_space` defines how much space is available in each packet for the plaintext
/// message. It will depend on, among other things, whether a SURB needs to be put there

// pub fn split_and_prepare_payloads(message: &[u8], available_packet_space: usize) -> Vec<Vec<u8>> {
pub fn split_and_prepare_payloads(message: &[u8]) -> Vec<Vec<u8>> {
    // let fragmented_messages = split_into_sets(message, available_packet_space);
    let fragmented_messages = split_into_sets(message);
    fragmented_messages
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|fragment| fragment.into_bytes())
        .collect()
}
