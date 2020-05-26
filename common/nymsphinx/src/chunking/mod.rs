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

use crate::chunking::set::split_into_sets;
use crate::packets::PacketSize;

pub mod fragment;
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
}

impl Default for MessageChunker {
    fn default() -> Self {
        MessageChunker {
            packet_size: Default::default(),
            reply_surbs: false,
            surb_acks: false,
        }
    }
}

/// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
/// directly. After receiving they can be combined using `reconstruction::MessageReconstructor`
/// to obtain the original message back.
pub fn split_and_prepare_payloads(message: &[u8]) -> Vec<Vec<u8>> {
    let fragmented_messages = split_into_sets(message);
    fragmented_messages
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|fragment| fragment.into_bytes())
        .collect()
}
