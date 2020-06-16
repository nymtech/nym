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

use crate::fragment::{Fragment, FragmentIdentifier};
use crate::set::split_into_sets;
use nymsphinx_acknowledgements::identifier::AckAes128Key;
use nymsphinx_acknowledgements::surb_ack::SURBAck;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_params::packet_sizes::PacketSize;
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
    average_packet_delay_duration: Duration,
    average_ack_delay_duration: Duration,
}

impl MessageChunker<DefaultRng> {
    pub fn new(
        ack_recipient: Recipient,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
    ) -> Self {
        Self::new_with_rng(
            DEFAULT_RNG,
            ack_recipient,
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
        Self::new(empty_recipient, Default::default(), Default::default())
    }
}

impl<R: CryptoRng + Rng> MessageChunker<R> {
    pub fn new_with_rng(
        rng: R,
        ack_recipient: Recipient,
        average_packet_delay_duration: Duration,
        average_ack_delay_duration: Duration,
    ) -> Self {
        MessageChunker {
            rng,
            ack_recipient,
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
    pub fn prepare_chunk_for_sending<T: NymTopology>(
        &mut self,
        fragment: Fragment,
        topology: &T,
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

        let route = topology.random_route_to_gateway(&packet_recipient.gateway())?;
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

    fn generate_surb_ack<T>(
        &mut self,
        fragment_id: &FragmentIdentifier,
        topology: &T,
        ack_key: &AckAes128Key,
    ) -> Result<SURBAck, NymTopologyError>
    where
        T: NymTopology,
    {
        SURBAck::construct(
            &mut self.rng,
            &self.ack_recipient,
            ack_key,
            &fragment_id.to_bytes(),
            self.average_ack_delay_duration,
            topology,
        )
    }

    /// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
    /// after attaching SURB-ACK.
    /// After receiving they can be combined using `reconstruction::MessageReconstructor`
    /// to obtain the original message back.
    pub fn split_message(&mut self, message: &[u8]) -> Vec<Fragment> {
        let available_plaintext_per_fragment = self.available_plaintext_size();

        split_into_sets(&mut self.rng, message, available_plaintext_per_fragment)
            .into_iter()
            .flat_map(|fragment_set| fragment_set.into_iter())
            .collect()
    }
}
