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

use crypto::asymmetric::encryption;
use crypto::kdf::blake3_hkdf;
use crypto::symmetric::aes_ctr::{
    self, generic_array::typenum::Unsigned, Aes128Key, Aes128KeySize,
};
use nymsphinx_acknowledgements::surb_ack::SURBAck;
use nymsphinx_acknowledgements::AckAes128Key;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_anonymous_replies::reply_surb::ReplySURB;
use nymsphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nymsphinx_chunking::set::split_into_sets;
use nymsphinx_chunking::MessageChunker;
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::DEFAULT_NUM_MIX_HOPS;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{delays, Delay, SphinxPacket};
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use topology::{NymTopology, NymTopologyError};

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
pub enum PreparationError {
    TopologyError(NymTopologyError),
}

impl From<NymTopologyError> for PreparationError {
    fn from(err: NymTopologyError) -> Self {
        PreparationError::TopologyError(err)
    }
}

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
pub struct MessagePreparer<R: CryptoRng + Rng> {
    rng: R,

    ack_key: Arc<AckAes128Key>,

    /// Size of the target [`SphinxPacket`] into which the underlying is going to get split.
    packet_size: PacketSize,

    /// Address of this client which also represent an address to which all acknowledgements
    /// and surb-based are going to be sent.
    our_address: Recipient,

    average_packet_delay_duration: Duration,
    average_ack_delay_duration: Duration,
}

impl<R> MessagePreparer<R>
where
    R: CryptoRng + Rng,
{
    fn new(rng: R) -> Self {
        // let chunker=  MessageChunker::new_with_rng(rng,)

        todo!()
        // MessagePreparer {
        //     rng,
        //     chunker,
        //     ack_key,
        //     our_address,
        //     average_packet_delay_duration,
        //     average_ack_delay_duration
        // }
    }

    /// Length of plaintext (from the sphinx point of view) data that is available per sphinx
    /// packet.
    fn available_plaintext_per_packet(&self) -> usize {
        // we need to put first hop's destination alongside the actual ack data
        // TODO: a possible optimization way down the line: currently we're always assuming that
        // the addresses will have `MAX_NODE_ADDRESS_UNPADDED_LEN`, i.e. be ipv6. In most cases
        // they're actually going to be ipv4 hence wasting few bytes every packet.
        // To fully utilise all available space, I guess first we'd need to generate routes for ACKs
        // and only then perform the chunking with `available_plaintext_size` being called per chunk.
        // However this will probably introduce bunch of complexity
        // for relatively not a lot of gain, so it shouldn't be done just yet.
        let ack_overhead = MAX_NODE_ADDRESS_UNPADDED_LEN + PacketSize::ACKPacket.size();
        let ephemeral_public_key_overhead = encryption::PUBLIC_KEY_SIZE;

        self.packet_size.plaintext_size() - ack_overhead - ephemeral_public_key_overhead
    }

    /// Pads the message so that after it gets chunked, it will occupy exactly N sphinx packets.
    fn pad_message(&self, message: Vec<u8>) -> Vec<u8> {
        // 1 is added as there will always have to be at least a single byte of padding (1) added
        // to be able to later distinguish the actual padding from the underlying message
        let (_, space_left) = MessageChunker::number_of_required_fragments(
            message.len() + 1,
            self.available_plaintext_per_packet(),
        );

        message
            .into_iter()
            .chain(std::iter::once(1u8))
            .chain(std::iter::repeat(0u8).take(space_left))
            .collect()
    }

    /// Generate fresh set of ephemeral keys for one data chunk.
    // we need to keep our ephemeral public key for sending and the actual derived key for encryption
    fn new_ephemeral_shared_key(
        &mut self,
        remote_key: &encryption::PublicKey,
    ) -> (encryption::PublicKey, Aes128Key) {
        let ephemeral_keypair = encryption::KeyPair::new_with_rng(&mut self.rng);

        // after performing diffie-hellman we don't care about the private component anymore
        let dh_result = ephemeral_keypair.private_key().diffie_hellman(remote_key);

        // there is no reason for this to fail as our okm is expected to be only 16 bytes
        let okm =
            blake3_hkdf::extract_then_expand(None, &dh_result, None, Aes128KeySize::to_usize())
                .expect("somehow too long okm was provided");

        let derived_shared_key =
            Aes128Key::from_exact_iter(okm).expect("okm was expanded to incorrect length!");

        (ephemeral_keypair.public_key().clone(), derived_shared_key)
    }

    /// Generates fresh pseudorandom key that is going to be used by the recipient of the message
    /// to encrypt payload of the reply. It is only generated when reply-SURB is attached.
    fn new_reply_key(&mut self) -> Aes128Key {
        aes_ctr::generate_key(&mut self.rng)
    }

    /// Attaches reply-SURB to the message alongside the reply key.
    fn optionally_attach_reply_surb(
        &mut self,
        message: Vec<u8>,
        should_attach: bool,
        topology: &NymTopology,
    ) -> Result<(Vec<u8>, Option<Aes128Key>), PreparationError> {
        if should_attach {
            let reply_surb = ReplySURB::construct(
                &mut self.rng,
                &self.our_address,
                self.average_packet_delay_duration,
                topology,
            )?;

            let reply_key = self.new_reply_key();

            // if there's a reply surb, the message takes form of `1 || REPLY_KEY || REPLY_SURB || MSG`
            Ok((
                std::iter::once(1u8)
                    .chain(reply_key.as_bytes().iter().cloned())
                    .chain(reply_surb.to_bytes().iter().cloned())
                    .chain(message.into_iter())
                    .collect(),
                Some(reply_key),
            ))
        } else {
            // but if there's no reply surb, the message takes form of `0 || MSG`
            Ok((
                std::iter::once(0u8).chain(message.into_iter()).collect(),
                None,
            ))
        }
    }

    /// Splits the message into [`Fragment`] that are going to be put later put into sphinx packets.
    fn split_message(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        split_into_sets(
            &mut self.rng,
            &message,
            self.available_plaintext_per_packet(),
        )
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .collect()
    }

    /// Tries to convert this [`Fragment`] into a [`SphinxPacket`] that can be sent through the Nym mix-network,
    /// such that it contains required SURB-ACK and public component of the ephemeral key used to
    /// derive the shared key.
    /// Also all the data, apart from the said public component, is encrypted with an ephemeral shared key.
    /// This method can fail if the provided network topology is invalid.
    /// It returns total expected delay as well as the [`SphinxPacket`] (including first hop address)
    /// to be sent through the network.
    fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        packet_recipient: &Recipient,
    ) -> Result<PreparedMessage, NymTopologyError> {
        // create an ack
        let (ack_delay, surb_ack_bytes) = self
            .generate_surb_ack(&fragment.fragment_identifier(), topology, ack_key)?
            .prepare_for_sending();

        // create keys for 'payload' encryption
        let (ephemeral_key, shared_key) = self.new_ephemeral_shared_key(recipient.encryption_key());

        // serialize fragment and encrypt its content
        let mut chunk_data = fragment.into_bytes();
        aes_ctr::encrypt_in_place(&shared_key, &aes_ctr::zero_iv(), &mut chunk_data);

        // combine it together as follows:
        // SURB_ACK_FIRST_HOP || SURB_ACK_DATA || EPHEMERAL_KEY || CHUNK_DATA
        // (note: surb_ack_bytes contains SURB_ACK_FIRST_HOP || SURB_ACK_DATA )
        let packet_payload: Vec<_> = surb_ack_bytes
            .into_iter()
            .chain(ephemeral_key.to_bytes().iter().cloned)
            .chain(chunk_data.into_iter())
            .collect();

        // generate pseudorandom route for the packet
        let route = topology.random_route_to_gateway(
            &mut self.rng,
            DEFAULT_NUM_MIX_HOPS,
            &packet_recipient.gateway(),
        )?;
        let destination = packet_recipient.as_sphinx_destination();

        // including set of delays
        let delays =
            delays::generate_from_average_duration(route.len(), self.average_packet_delay_duration);

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        // note: once merged, that's an easy rng injection point for sphinx packets : )
        let sphinx_packet = SphinxPacketBuilder::new()
            .with_payload_size(self.packet_size.payload_size())
            .build_packet(packet_payload, &route, &destination, &delays)
            .unwrap();

        // from the previously constructed route extract the first hop
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();

        Ok(PreparedMessage {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            total_delay: delays.iter().sum::<Delay>() + ack_delay,
            first_hop_address,
            sphinx_packet,
        })
    }

    /// Construct an acknowledgement SURB for the given [`FragmentIdentifier`]
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

    pub fn prepare_message(
        &mut self,
        message: Vec<u8>,
        recipient: &Recipient,
        with_reply_surb: bool,
        topology: &NymTopology,
    ) -> Result<(Vec<PreparedMessage>, Option<Aes128Key>), PreparationError> {
        // 1. attach (or not) the reply-surb
        // new_message = 0 || message
        // OR
        // new_message = 1 || REPLY_KEY || REPLY_SURB || message
        let (message, reply_key) =
            self.optionally_attach_reply_surb(message, with_reply_surb, topology)?;

        // 2. pad the message, so that when chunked it fits into EXACTLY N sphinx packets
        // new_message = message || 1 || 0000....
        let message = self.pad_message(message);

        // 3. chunk the message so that each chunk fits into a sphinx packet. Note, extra 32 bytes
        // are left in each chunk
        let fragments = self.split_message(message);

        // 4. For each fragment:
        // - generate (x, g^x)
        // - compute k = KDF(remote encryption key ^ x) this is equivalent to KDF( dh(remote, x) )
        // - compute v_b = AES-128-CTR(k, serialized_fragment)
        // - compute vk_b = g^x || v_b
        // - compute sphinx = Sphinx(recipient, vk_b)
        let mut prepared_messages = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            let prepared_message = self.prepare_chunk_for_sending(fragment, topology, recipient)?;
            prepared_messages.push(prepared_message)
        }

        Ok((prepared_messages, reply_key))
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
