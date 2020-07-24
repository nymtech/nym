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

use crate::chunking;
use crypto::asymmetric::encryption;
use crypto::new_ephemeral_shared_key;
use crypto::symmetric::aes_ctr::{self};
use nymsphinx_acknowledgements::surb_ack::SURBAck;
use nymsphinx_acknowledgements::AckAes128Key;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_anonymous_replies::encryption_key::{Digest, SURBEncryptionKey};
use nymsphinx_anonymous_replies::reply_surb::ReplySURB;
use nymsphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::{
    MessageType, PacketHkdfAlgorithm, ReplySURBKeyDigestAlgorithm, DEFAULT_NUM_MIX_HOPS,
};
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{delays, Delay, SphinxPacket};
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::time::Duration;
use topology::{NymTopology, NymTopologyError};

/// Represents fully packed and prepared [`Fragment`] that can be sent through the mix network.
pub struct PreparedFragment {
    /// Indicates the total expected round-trip time, i.e. delay from the sending of this message
    /// until receiving the acknowledgement included inside of it.
    pub total_delay: Delay,

    /// Indicates address of the node to which the message should be sent.
    pub first_hop_address: NymNodeRoutingAddress,

    /// The actual 'chunk' of the message that is going to go through the mix network.
    pub sphinx_packet: SphinxPacket,
}

#[derive(Debug)]
pub enum PreparationError {
    TopologyError(NymTopologyError),
    TooLongReplyMessageError,
}

impl From<NymTopologyError> for PreparationError {
    fn from(err: NymTopologyError) -> Self {
        PreparationError::TopologyError(err)
    }
}

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
#[derive(Debug, Clone)]
pub struct MessagePreparer<R: CryptoRng + Rng> {
    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Size of the target [`SphinxPacket`] into which the underlying is going to get split.
    packet_size: PacketSize,

    /// Address of this client which also represent an address to which all acknowledgements
    /// and surb-based are going to be sent.
    sender_address: Recipient,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Number of mix hops each packet ('real' message, ack, reply) is expected to take.
    /// Note that it does not include gateway hops.
    num_mix_hops: u8,
}

impl<R> MessagePreparer<R>
where
    R: CryptoRng + Rng,
{
    pub fn new(
        rng: R,
        sender_address: Recipient,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
    ) -> Self {
        MessagePreparer {
            rng,
            packet_size: Default::default(),
            sender_address,
            average_packet_delay,
            average_ack_delay,
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
        }
    }

    /// Allows setting non-default number of expected mix hops in the network.
    pub fn with_mix_hops(mut self, hops: u8) -> Self {
        self.num_mix_hops = hops;
        self
    }

    /// Allows setting non-default size of the sphinx packets sent out.
    pub fn with_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.packet_size = packet_size;
        self
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
        let (_, space_left) = chunking::number_of_required_fragments(
            message.len() + 1,
            self.available_plaintext_per_packet(),
        );

        message
            .into_iter()
            .chain(std::iter::once(1u8))
            .chain(std::iter::repeat(0u8).take(space_left))
            .collect()
    }

    /// Attaches reply-SURB to the message alongside the reply key.
    fn optionally_attach_reply_surb(
        &mut self,
        message: Vec<u8>,
        should_attach: bool,
        topology: &NymTopology,
    ) -> Result<(Vec<u8>, Option<SURBEncryptionKey>), PreparationError> {
        if should_attach {
            let reply_surb = ReplySURB::construct(
                &mut self.rng,
                &self.sender_address,
                self.average_packet_delay,
                topology,
            )?;

            let reply_key = reply_surb.encryption_key();
            // if there's a reply surb, the message takes form of `1 || REPLY_KEY || REPLY_SURB || MSG`
            Ok((
                std::iter::once(MessageType::WithReplySURB as u8)
                    .chain(reply_surb.to_bytes().iter().cloned())
                    .chain(message.into_iter())
                    .collect(),
                Some(reply_key.clone()),
            ))
        } else {
            // but if there's no reply surb, the message takes form of `0 || MSG`
            Ok((
                std::iter::once(MessageType::WithoutReplySURB as u8)
                    .chain(message.into_iter())
                    .collect(),
                None,
            ))
        }
    }

    /// Splits the message into [`Fragment`] that are going to be put later put into sphinx packets.
    fn split_message(&mut self, message: Vec<u8>) -> Vec<Fragment> {
        let plaintext_per_packet = self.available_plaintext_per_packet();
        chunking::split_into_sets(&mut self.rng, &message, plaintext_per_packet)
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
    ///
    /// The procedure is as follows:
    /// For each fragment:
    /// - compute SURB_ACK
    /// - generate (x, g^x)
    /// - compute k = KDF(remote encryption key ^ x) this is equivalent to KDF( dh(remote, x) )
    /// - compute v_b = AES-128-CTR(k, serialized_fragment)
    /// - compute vk_b = g^x || v_b
    /// - compute sphinx_plaintext = SURB_ACK || g^x || v_b
    /// - compute sphinx_packet = Sphinx(recipient, sphinx_plaintext)
    pub fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckAes128Key,
        packet_recipient: &Recipient,
    ) -> Result<PreparedFragment, NymTopologyError> {
        // create an ack
        let (ack_delay, surb_ack_bytes) = self
            .generate_surb_ack(fragment.fragment_identifier(), topology, ack_key)?
            .prepare_for_sending();

        // create keys for 'payload' encryption
        let (ephemeral_keypair, shared_key) = new_ephemeral_shared_key::<PacketHkdfAlgorithm, _>(
            &mut self.rng,
            packet_recipient.encryption_key(),
        );

        // serialize fragment and encrypt its content
        let mut chunk_data = fragment.into_bytes();
        aes_ctr::encrypt_in_place(&shared_key, &aes_ctr::zero_iv(), &mut chunk_data);

        // combine it together as follows:
        // SURB_ACK_FIRST_HOP || SURB_ACK_DATA || EPHEMERAL_KEY || CHUNK_DATA
        // (note: surb_ack_bytes contains SURB_ACK_FIRST_HOP || SURB_ACK_DATA )
        let packet_payload: Vec<_> = surb_ack_bytes
            .into_iter()
            .chain(ephemeral_keypair.public_key().to_bytes().iter().cloned())
            .chain(chunk_data.into_iter())
            .collect();

        // generate pseudorandom route for the packet
        let route = topology.random_route_to_gateway(
            &mut self.rng,
            self.num_mix_hops,
            &packet_recipient.gateway(),
        )?;
        let destination = packet_recipient.as_sphinx_destination();

        // including set of delays
        let delays = delays::generate_from_average_duration(route.len(), self.average_packet_delay);

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

        Ok(PreparedFragment {
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
        fragment_id: FragmentIdentifier,
        topology: &NymTopology,
        ack_key: &AckAes128Key,
    ) -> Result<SURBAck, NymTopologyError> {
        SURBAck::construct(
            &mut self.rng,
            &self.sender_address,
            ack_key,
            fragment_id.to_bytes(),
            self.average_ack_delay,
            topology,
        )
    }

    /// Attaches an optional reply-surb and correct padding to the underlying message
    /// and splits it into [`Fragment`] that can be later packed into sphinx packets to be
    /// sent through the mix network.
    pub fn prepare_and_split_message(
        &mut self,
        message: Vec<u8>,
        with_reply_surb: bool,
        topology: &NymTopology,
    ) -> Result<(Vec<Fragment>, Option<SURBEncryptionKey>), PreparationError> {
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
        Ok((self.split_message(message), reply_key))

        // 4. For each fragment:
        // - generate (x, g^x)
        // - compute k = KDF(remote encryption key ^ x) this is equivalent to KDF( dh(remote, x) )
        // - compute v_b = AES-128-CTR(k, serialized_fragment)
        // - compute vk_b = g^x || v_b
        // - compute sphinx = Sphinx(recipient, vk_b)
    }

    // TODO: perhaps the return type could somehow be combined with [`PreparedFragment`] ?
    pub fn prepare_reply_for_use(
        &mut self,
        message: Vec<u8>,
        reply_surb: ReplySURB,
        topology: &NymTopology,
        ack_key: &AckAes128Key,
    ) -> Result<(FragmentIdentifier, SphinxPacket, NymNodeRoutingAddress), PreparationError> {
        // there's no chunking in reply-surbs so there's a hard limit on message,
        // we also need to put the key digest into the message (same size as ephemeral key)
        // and need 1 byte to indicate padding length (this is not the case for 'normal' messages
        // as there the padding is added for the whole message)
        // so before doing any processing, let's see if we have enough space for it all
        let ack_overhead = MAX_NODE_ADDRESS_UNPADDED_LEN + PacketSize::ACKPacket.size();
        if message.len()
            > self.packet_size.plaintext_size()
                - ack_overhead
                - ReplySURBKeyDigestAlgorithm::output_size()
                - 1
        {
            return Err(PreparationError::TooLongReplyMessageError);
        }

        let reply_id = FragmentIdentifier::new_reply(&mut self.rng);

        // create an ack
        // even though it won't be used for retransmission, it must be present so that
        // gateways could not distinguish reply packets from normal messages due to lack of said acks
        // note: the ack delay is irrelevant since we do not know the delay of actual surb
        let (_, surb_ack_bytes) = self
            .generate_surb_ack(reply_id, topology, ack_key)?
            .prepare_for_sending();

        let zero_pad_len = self.packet_size.plaintext_size()
            - message.len()
            - ack_overhead
            - ReplySURBKeyDigestAlgorithm::output_size()
            - 1;

        // create reply message that will reach the recipient:
        let mut reply_content: Vec<_> = message
            .into_iter()
            .chain(std::iter::once(1))
            .chain(std::iter::repeat(0).take(zero_pad_len))
            .collect();

        // encrypt the reply message
        aes_ctr::encrypt_in_place(
            reply_surb.encryption_key(),
            &aes_ctr::zero_iv(),
            &mut reply_content,
        );

        // combine it together as follows:
        // SURB_ACK_FIRST_HOP || SURB_ACK_DATA || KEY_DIGEST || E (REPLY_MESSAGE || 1 || 0*)
        // (note: surb_ack_bytes contains SURB_ACK_FIRST_HOP || SURB_ACK_DATA )
        let packet_payload: Vec<_> = surb_ack_bytes
            .into_iter()
            .chain(
                reply_surb
                    .encryption_key()
                    .compute_digest()
                    .to_vec()
                    .into_iter(),
            )
            .chain(reply_content.into_iter())
            .collect();

        // finally put it all inside a sphinx packet
        // this can only fail if packet payload has incorrect size, but if it does, it means
        // there's a bug in the above code
        let (packet, first_hop) = reply_surb
            .apply_surb(&packet_payload, Some(self.packet_size))
            .unwrap();

        Ok((reply_id, packet, first_hop))
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> MessagePreparer<rand::rngs::OsRng> {
        let rng = rand::rngs::OsRng;
        let dummy_address = Recipient::try_from_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();

        MessagePreparer {
            rng,
            packet_size: Default::default(),
            sender_address: dummy_address,
            average_packet_delay: Default::default(),
            average_ack_delay: Default::default(),
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
        }
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
