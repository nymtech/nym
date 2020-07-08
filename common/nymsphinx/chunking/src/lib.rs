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

use crate::fragment::{
    linked_fragment_payload_max_len, unlinked_fragment_payload_max_len, Fragment,
    FragmentIdentifier,
};
use crate::set::split_into_sets;
use nymsphinx_acknowledgements::surb_ack::SURBAck;
use nymsphinx_acknowledgements::AckAes128Key;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::DEFAULT_NUM_MIX_HOPS;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{delays, Delay, Destination, SphinxPacket};
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time::Duration;
use topology::{NymTopology, NymTopologyError};

// Future consideration: currently in a lot of places, the payloads have randomised content
// which is not a perfect testing strategy as it might not detect some edge cases I never would
// have assumed could be possible. A better approach would be to research some Fuzz testing
// library like: https://github.com/rust-fuzz/afl.rs and use that instead for the inputs.

// perhaps it might be useful down the line for interaction testing between client,mixes,etc?

// TODO: this module has evolved significantly since the tests were first written
// they should definitely be revisited.
// For instance there are not tests for the cases when we are padding the message

pub mod fragment;
pub mod reconstruction;
pub mod set;

type DefaultRng = OsRng;
const DEFAULT_RNG: DefaultRng = OsRng;

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
    MalformedFragmentIdentifier,
}

// Note: `Rng` implies `RngCore`
#[derive(Debug, Clone)]
pub struct MessageChunker<R: CryptoRng + Rng> {
    rng: R,
    ack_recipient: Recipient,
    packet_size: PacketSize,
    reply_surbs: bool,
    should_pad: bool,
    average_packet_delay_duration: Duration,
    average_ack_delay_duration: Duration,
}

impl MessageChunker<DefaultRng> {
    pub fn new(
        ack_recipient: Recipient,
        should_pad: bool,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
    ) -> Self {
        Self::new_with_rng(
            DEFAULT_RNG,
            ack_recipient,
            should_pad,
            average_packet_delay_duration,
            average_ack_delay_duration,
        )
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        use nymsphinx_types::{DestinationAddressBytes, NodeAddressBytes};

        let empty_address = [0u8; 32];
        let empty_recipient = Recipient::new(
            DestinationAddressBytes::from_bytes(empty_address),
            NodeAddressBytes::from_bytes(empty_address),
        );
        Self::new(
            empty_recipient,
            false,
            Default::default(),
            Default::default(),
        )
    }
}

impl<R> MessageChunker<R>
where
    R: CryptoRng + Rng,
{
    pub fn new_with_rng(
        rng: R,
        ack_recipient: Recipient,
        should_pad: bool,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
    ) -> Self {
        MessageChunker {
            rng,
            ack_recipient,
            should_pad,
            packet_size: Default::default(),
            reply_surbs: false,
            average_packet_delay_duration,
            average_ack_delay_duration,
        }
    }

    pub fn available_plaintext_size(&self) -> usize {
        // we need to put first hop's destination alongside the actual ack
        // TODO: a possible optimization way down the line: currently we're always assuming that
        // the addresses will have `MAX_NODE_ADDRESS_UNPADDED_LEN`, i.e. be ipv6. In most cases
        // they're actually going to be ipv4 hence wasting few bytes every packet.
        // To fully utilise all available space, I guess first we'd need to generate routes for ACKs
        // and only then perform the chunking with `available_plaintext_size` being called per chunk.
        // However this will probably introduce bunch of complexity
        // for relatively not a lot of gain, so it shouldn't be done just yet.
        let available_size = self.packet_size.plaintext_size()
            - PacketSize::ACKPacket.size()
            - MAX_NODE_ADDRESS_UNPADDED_LEN;
        if self.reply_surbs {
            // TODO
            unimplemented!();
        }
        available_size
    }

    pub fn with_reply_surbs(mut self, reply_surbs: bool) -> Self {
        self.reply_surbs = reply_surbs;
        self
    }

    pub fn with_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.packet_size = packet_size;
        self
    }

    /// Tries to convert this `Fragment` into a `SphinxPacket` that can be sent through the Nym mix-network,
    /// such that it contains required SURB-ACK.
    /// This method can fail if the provided network topology is invalid.
    /// It returns total expected delay as well as the `SphinxPacket` to be sent through the network.
    pub fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckAes128Key,
        packet_recipient: &Recipient,
    ) -> Result<(Delay, (SocketAddr, SphinxPacket)), NymTopologyError> {
        let (ack_delay, surb_bytes) = self
            .generate_surb_ack(&fragment.fragment_identifier(), topology, ack_key)?
            .prepare_for_sending();

        // SURB_FIRST_HOP || SURB_ACK || CHUNK_DATA
        let packet_payload: Vec<_> = surb_bytes
            .into_iter()
            .chain(fragment.into_bytes().into_iter())
            .collect();

        let route = topology.random_route_to_gateway(
            &mut self.rng,
            DEFAULT_NUM_MIX_HOPS,
            &packet_recipient.gateway(),
        )?;
        let delays =
            delays::generate_from_average_duration(route.len(), self.average_packet_delay_duration);
        let destination = Destination::new(packet_recipient.destination(), Default::default());

        // once merged, that's an easy rng injection point for sphinx packets : )
        let packet = SphinxPacketBuilder::new()
            .with_payload_size(self.packet_size.payload_size())
            .build_packet(packet_payload, &route, &destination, &delays)
            .unwrap();

        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();

        Ok((
            delays.iter().sum::<Delay>() + ack_delay,
            (first_hop_address.into(), packet),
        ))
    }

    fn generate_surb_ack(
        &mut self,
        fragment_id: &FragmentIdentifier,
        topology: &NymTopology,
        ack_key: &AckAes128Key,
    ) -> Result<SURBAck, NymTopologyError> {
        SURBAck::construct(
            &mut self.rng,
            &self.ack_recipient,
            ack_key,
            fragment_id.to_bytes(),
            self.average_ack_delay_duration,
            topology,
        )
    }

    /// Returns number of fragments the message will be split to as well as number of available
    /// bytes in the final fragment
    pub fn number_of_required_fragments(
        message_len: usize,
        plaintext_per_fragment: usize,
    ) -> (usize, usize) {
        let max_unlinked = unlinked_fragment_payload_max_len(plaintext_per_fragment);
        let max_linked = linked_fragment_payload_max_len(plaintext_per_fragment);

        match set::total_number_of_sets(message_len, plaintext_per_fragment) {
            n if n == 1 => {
                // is if it's a single fragment message
                if message_len < max_unlinked {
                    return (1, max_unlinked - message_len);
                }

                // all fragments will be 'unlinked'
                let quot = message_len / max_unlinked;
                let rem = message_len % max_unlinked;

                if rem == 0 {
                    (quot, 0)
                } else {
                    (quot + 1, max_unlinked - rem)
                }
            }

            n => {
                // in first and last set there will be one 'linked' fragment
                // and two 'linked' fragment in every other set, meaning
                // there will be 2 * (n - 2) + 2 = 2n - 2 'linked' fragments total
                // rest will be 'unlinked'

                // we know for sure that all fragments in all but last set are definitely full
                // (last one has single 'linked' fragment)
                let without_last = (n - 1) * (u8::max_value() as usize);
                let linked_fragments_without_last = (2 * n - 2) - 1;
                let unlinked_fragments_without_last = without_last - linked_fragments_without_last;

                let final_set_message_len = message_len
                    - linked_fragments_without_last * max_linked
                    - unlinked_fragments_without_last * max_unlinked;

                // we must be careful with the last set as it might be the case that it only
                // consists of a single, linked, non-full fragment
                if final_set_message_len < max_linked {
                    return (without_last + 1, max_linked - final_set_message_len);
                } else if final_set_message_len == max_linked {
                    return (without_last + 1, 0);
                }

                let remaining_len = final_set_message_len - max_linked;

                let quot = remaining_len / max_unlinked;
                let rem = remaining_len % max_unlinked;

                if rem == 0 {
                    (without_last + quot + 1, 0)
                } else {
                    (without_last + quot + 2, max_unlinked - rem)
                }
            }
        }
    }

    /// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
    /// after attaching SURB-ACK, such that the payload of the sphinx packet will be fully
    /// used up.
    /// After receiving they can be combined using `reconstruction::MessageReconstructor`
    /// to obtain the original message back.
    pub fn split_message_to_constant_length_chunks(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        let available_plaintext_per_fragment = self.available_plaintext_size();

        // 1 is added as there will always have to be at least a single byte of padding (1) added
        // to be able to later remove the padding
        let (_, space_left) =
            Self::number_of_required_fragments(message.len() + 1, available_plaintext_per_fragment);

        // TODO: this makes copy of all data and so will a fragment chunker,
        // so a tiny optimization would be to make all
        // methods using this value, i.e. take Vec<u8> rather than &[u8]
        let message: Vec<_> = message
            .into_iter()
            .chain(std::iter::once(1u8))
            .chain(std::iter::repeat(0u8).take(space_left))
            .collect();

        split_into_sets(&mut self.rng, &message, available_plaintext_per_fragment)
            .into_iter()
            .flat_map(|fragment_set| fragment_set.into_iter())
            .collect()
    }

    /// Takes the message that is to be split and prefixes it with either a 0 byte to indicate
    /// lack of reply surb or with a 1 byte followed by an actual reply SURB.
    // TODO: perhaps if we wanted to incldue multiple reply SURBs, we could change 0/1 into
    // number of reply SURBs attached?
    fn prepare_and_attach_reply_surb(&self, message: &[u8]) -> Vec<u8> {
        let prefix: Vec<_> = if self.reply_surbs {
            let reply_surb_bytes_todo: Vec<u8> = Vec::new();
            std::iter::once(1)
                .chain(reply_surb_bytes_todo.into_iter())
                .collect()
        } else {
            std::iter::once(0).collect()
        };

        prefix.into_iter().chain(message.iter().cloned()).collect()
    }

    /// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
    /// after attaching SURB-ACK.
    /// After receiving they can be combined using `reconstruction::MessageReconstructor`
    /// to obtain the original message back.
    pub fn split_message(&mut self, message: &[u8]) -> Vec<Fragment> {
        // TODO: future optimization: message is currently 'unnecessarily' copied two times
        let message_with_reply_surb = self.prepare_and_attach_reply_surb(message);

        if self.should_pad {
            self.split_message_to_constant_length_chunks(message_with_reply_surb)
        } else {
            let available_plaintext_per_fragment = self.available_plaintext_size();

            split_into_sets(
                &mut self.rng,
                &message_with_reply_surb,
                available_plaintext_per_fragment,
            )
            .into_iter()
            .flat_map(|fragment_set| fragment_set.into_iter())
            .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::{max_one_way_linked_set_payload_length, two_way_linked_set_payload_length};

    #[test]
    fn calculating_number_of_required_fragments() {
        // plaintext len should not affect this at all, but let's test it with something tiny
        // and reasonable
        let used_plaintext_len = PacketSize::default().plaintext_size()
            - PacketSize::ACKPacket.size()
            - MAX_NODE_ADDRESS_UNPADDED_LEN;

        let plaintext_lens = vec![17, used_plaintext_len, 20, 42, 10000];
        const SET_LEN: usize = u8::max_value() as usize;

        for plaintext_len in plaintext_lens {
            let unlinked_len = unlinked_fragment_payload_max_len(plaintext_len);
            let linked_len = linked_fragment_payload_max_len(plaintext_len);
            let full_edge_set = max_one_way_linked_set_payload_length(plaintext_len);
            let full_middle_set = two_way_linked_set_payload_length(plaintext_len);

            let single_non_full_frag_message_len = unlinked_len - 5;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                single_non_full_frag_message_len,
                plaintext_len,
            );
            assert_eq!(frags, 1);
            assert_eq!(space_left, unlinked_len - single_non_full_frag_message_len);

            let single_full_frag_message_len = unlinked_len;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                single_full_frag_message_len,
                plaintext_len,
            );
            assert_eq!(frags, 1);
            assert_eq!(space_left, 0);

            let two_non_full_frags_len = unlinked_len + 1;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_non_full_frags_len,
                plaintext_len,
            );
            assert_eq!(frags, 2);
            assert_eq!(space_left, unlinked_len - 1);

            let two_full_frags_len = 2 * unlinked_len;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_full_frags_len,
                plaintext_len,
            );
            assert_eq!(frags, 2);
            assert_eq!(space_left, 0);

            let multi_single_set_frags_non_full = unlinked_len * 42 - 5;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                multi_single_set_frags_non_full,
                plaintext_len,
            );
            assert_eq!(frags, 42);
            assert_eq!(space_left, 5);

            let multi_single_set_frags_full = unlinked_len * 42;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                multi_single_set_frags_full,
                plaintext_len,
            );
            assert_eq!(frags, 42);
            assert_eq!(space_left, 0);

            let two_set_one_non_full_frag = full_edge_set + linked_len - 1;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_set_one_non_full_frag,
                plaintext_len,
            );
            assert_eq!(frags, SET_LEN + 1);
            assert_eq!(space_left, 1);

            let two_set_one_full_frag = full_edge_set + linked_len;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_set_one_full_frag,
                plaintext_len,
            );
            assert_eq!(frags, SET_LEN + 1);
            assert_eq!(space_left, 0);

            let two_set_multi_frags_non_full = full_edge_set + linked_len + unlinked_len * 41 - 5;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_set_multi_frags_non_full,
                plaintext_len,
            );
            assert_eq!(frags, SET_LEN + 42);
            assert_eq!(space_left, 5);

            let two_set_multi_frags_full = full_edge_set + linked_len + unlinked_len * 41;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                two_set_multi_frags_full,
                plaintext_len,
            );
            assert_eq!(frags, SET_LEN + 42);
            assert_eq!(space_left, 0);

            let ten_set_one_non_full_frag = full_edge_set + 8 * full_middle_set + linked_len - 1;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                ten_set_one_non_full_frag,
                plaintext_len,
            );
            assert_eq!(frags, 9 * SET_LEN + 1);
            assert_eq!(space_left, 1);

            let ten_set_one_full_frag = full_edge_set + 8 * full_middle_set + linked_len;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                ten_set_one_full_frag,
                plaintext_len,
            );
            assert_eq!(frags, 9 * SET_LEN + 1);
            assert_eq!(space_left, 0);

            let ten_set_multi_frags_non_full =
                full_edge_set + 8 * full_middle_set + linked_len + 41 * unlinked_len - 5;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                ten_set_multi_frags_non_full,
                plaintext_len,
            );
            assert_eq!(frags, 9 * SET_LEN + 42);
            assert_eq!(space_left, 5);

            let ten_set_multi_frags_full =
                full_edge_set + 8 * full_middle_set + linked_len + 41 * unlinked_len;
            let (frags, space_left) = MessageChunker::<DefaultRng>::number_of_required_fragments(
                ten_set_multi_frags_full,
                plaintext_len,
            );
            assert_eq!(frags, 9 * SET_LEN + 42);
            assert_eq!(space_left, 0);
        }
    }
}
