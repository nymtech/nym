// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::message::{NymMessage, ACK_OVERHEAD, OUTFOX_ACK_OVERHEAD};
use crate::NymPayloadBuilder;
use nym_crypto::asymmetric::encryption;
use nym_crypto::Digest;
use nym_sphinx_acknowledgements::surb_ack::SurbAck;
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_anonymous_replies::reply_surb::ReplySurb;
use nym_sphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{PacketType, ReplySurbKeyDigestAlgorithm, DEFAULT_NUM_MIX_HOPS};
use nym_sphinx_types::{Delay, NymPacket};
use nym_topology::{NymTopology, NymTopologyError};
use rand::{CryptoRng, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use std::time::Duration;

pub(crate) mod payload;

/// Represents fully packed and prepared [`Fragment`] that can be sent through the mix network.
pub struct PreparedFragment {
    /// Indicates the total expected round-trip time, i.e. delay from the sending of this message
    /// until receiving the acknowledgement included inside of it.
    pub total_delay: Delay,

    /// Indicates all data required to serialize and forward the data. It contains the actual
    /// address of the node to which the message should be sent, the actual 'chunk' of the message
    /// going through the mix network and also the 'mode' of the packet, i.e. VPN or Mix.
    pub mix_packet: MixPacket,

    /// Identifier to uniquely identify a fragment.
    pub fragment_identifier: FragmentIdentifier,
}

impl From<PreparedFragment> for MixPacket {
    fn from(value: PreparedFragment) -> Self {
        value.mix_packet
    }
}

// this is extracted into a trait with default implementation to remove duplicate code
// (which we REALLY want to avoid with crypto)
pub trait FragmentPreparer {
    type Rng: CryptoRng + Rng;

    fn rng(&mut self) -> &mut Self::Rng;
    fn nonce(&self) -> i32;
    fn num_mix_hops(&self) -> u8;
    fn average_packet_delay(&self) -> Duration;
    fn average_ack_delay(&self) -> Duration;

    fn generate_reply_surbs(
        &mut self,
        amount: usize,
        topology: &NymTopology,
        reply_recipient: &Recipient,
    ) -> Result<Vec<ReplySurb>, NymTopologyError> {
        let mut reply_surbs = Vec::with_capacity(amount);
        let packet_delay = self.average_packet_delay();
        for _ in 0..amount {
            let reply_surb =
                ReplySurb::construct(self.rng(), reply_recipient, packet_delay, topology)?;
            reply_surbs.push(reply_surb)
        }

        Ok(reply_surbs)
    }

    fn generate_surb_ack(
        &mut self,
        recipient: &Recipient,
        fragment_id: FragmentIdentifier,
        topology: &NymTopology,
        ack_key: &AckKey,
        packet_type: PacketType,
    ) -> Result<SurbAck, NymTopologyError> {
        let ack_delay = self.average_ack_delay();

        SurbAck::construct(
            self.rng(),
            recipient,
            ack_key,
            fragment_id.to_bytes(),
            ack_delay,
            topology,
            packet_type,
        )
    }

    /// The procedure is as follows:
    /// For each fragment:
    /// - compute SURB_ACK
    /// - generate (x, g^x)
    /// - obtain key k from the reply-surb which was computed as follows:
    ///     k = KDF(remote encryption key ^ x) this is equivalent to KDF( dh(remote, x) )
    /// - compute v_b = AES-128-CTR(k, serialized_fragment)
    /// - compute vk_b = H(k) || v_b
    /// - compute sphinx_plaintext = SURB_ACK || H(k) || v_b
    /// - compute sphinx_packet by applying the reply surb on the sphinx_plaintext
    fn prepare_reply_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        reply_surb: ReplySurb,
        packet_sender: &Recipient,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, NymTopologyError> {
        // each reply attaches the digest of the encryption key so that the recipient could
        // lookup correct key for decryption,
        let reply_overhead = ReplySurbKeyDigestAlgorithm::output_size();
        let expected_plaintext = match packet_type {
            PacketType::Outfox => fragment.serialized_size() + OUTFOX_ACK_OVERHEAD + reply_overhead,
            _ => fragment.serialized_size() + ACK_OVERHEAD + reply_overhead,
        };

        // the reason we're unwrapping (or rather 'expecting') here rather than handling the error
        // more gracefully is that this error should never be reached as it implies incorrect chunking
        // reply packets are always Sphinx
        let packet_size = PacketSize::get_type_from_plaintext(expected_plaintext, PacketType::Mix)
            .expect("the message has been incorrectly fragmented");

        // this is not going to be accurate by any means. but that's the best estimation we can do
        let expected_forward_delay = Delay::new_from_millis(
            (self.average_packet_delay().as_millis() * self.num_mix_hops() as u128) as u64,
        );

        let fragment_identifier = fragment.fragment_identifier();

        // create an ack
        let surb_ack = self.generate_surb_ack(
            packet_sender,
            fragment_identifier,
            topology,
            ack_key,
            packet_type,
        )?;
        let ack_delay = surb_ack.expected_total_delay();

        let packet_payload = match NymPayloadBuilder::new(fragment, surb_ack)
            .build_reply(reply_surb.encryption_key())
        {
            Ok(payload) => payload,
            Err(_e) => return Err(NymTopologyError::PayloadBuilder),
        };

        // the unwrap here is fine as the failures can only originate from attempting to use invalid payload lengths
        // and we just very carefully constructed a (presumably) valid one
        let (sphinx_packet, first_hop_address) = reply_surb
            .apply_surb(packet_payload, packet_size, packet_type)
            .unwrap();

        Ok(PreparedFragment {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            // we don't know the delays inside the reply surbs so we use best-effort estimation from our poisson distribution
            total_delay: expected_forward_delay + ack_delay,
            mix_packet: MixPacket::new(first_hop_address, sphinx_packet, packet_type),
            fragment_identifier,
        })
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
    #[allow(clippy::too_many_arguments)]
    fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        packet_sender: &Recipient,
        packet_recipient: &Recipient,
        packet_type: PacketType,
        mix_hops: Option<u8>,
    ) -> Result<PreparedFragment, NymTopologyError> {
        // each plain or repliable packet (i.e. not a reply) attaches an ephemeral public key so that the recipient
        // could perform diffie-hellman with its own keys followed by a kdf to re-derive
        // the packet encryption key

        let seed = fragment.seed().wrapping_mul(self.nonce());
        let mut rng = ChaCha8Rng::seed_from_u64(seed as u64);
        // nym_metrics::fragment_sent!(seed);

        let non_reply_overhead = encryption::PUBLIC_KEY_SIZE;
        let expected_plaintext = match packet_type {
            PacketType::Outfox => {
                fragment.serialized_size() + OUTFOX_ACK_OVERHEAD + non_reply_overhead
            }
            _ => fragment.serialized_size() + ACK_OVERHEAD + non_reply_overhead,
        };

        // the reason we're unwrapping (or rather 'expecting') here rather than handling the error
        // more gracefully is that this error should never be reached as it implies incorrect chunking
        let packet_size = PacketSize::get_type_from_plaintext(expected_plaintext, packet_type)
            .expect("the message has been incorrectly fragmented");

        let fragment_identifier = fragment.fragment_identifier();

        // create an ack
        let surb_ack = self.generate_surb_ack(
            packet_sender,
            fragment_identifier,
            topology,
            ack_key,
            packet_type,
        )?;
        let ack_delay = surb_ack.expected_total_delay();

        let packet_payload = match NymPayloadBuilder::new(fragment, surb_ack)
            .build_regular(self.rng(), packet_recipient.encryption_key())
        {
            Ok(payload) => payload,
            Err(_e) => return Err(NymTopologyError::PayloadBuilder),
        };

        // generate pseudorandom route for the packet
        let hops = mix_hops.unwrap_or(self.num_mix_hops());
        log::trace!("Preparing chunk for sending with {} mix hops", hops);
        let route = topology.random_route_to_gateway(&mut rng, hops, packet_recipient.gateway())?;
        let destination = packet_recipient.as_sphinx_destination();

        // including set of delays
        let delays =
            nym_sphinx_routing::generate_hop_delays(self.average_packet_delay(), route.len());

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        let packet = match packet_type {
            PacketType::Outfox => NymPacket::outfox_build(
                packet_payload,
                route.as_slice(),
                &destination,
                Some(packet_size.plaintext_size()),
            )?,
            PacketType::Mix => NymPacket::sphinx_build(
                packet_size.payload_size(),
                packet_payload,
                &route,
                &destination,
                &delays,
            )?,
            #[allow(deprecated)]
            PacketType::Vpn => NymPacket::sphinx_build(
                packet_size.payload_size(),
                packet_payload,
                &route,
                &destination,
                &delays,
            )?,
        };

        // from the previously constructed route extract the first hop
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address).unwrap();

        Ok(PreparedFragment {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            // note that the last hop of the packet is a gateway that does not do any delays
            total_delay: delays.iter().take(delays.len() - 1).sum::<Delay>() + ack_delay,
            mix_packet: MixPacket::new(first_hop_address, packet, packet_type),
            fragment_identifier,
        })
    }

    fn pad_and_split_message(
        &mut self,
        message: NymMessage,
        packet_size: PacketSize,
    ) -> Vec<Fragment> {
        let plaintext_per_packet = message.available_sphinx_plaintext_per_packet(packet_size);

        message
            .pad_to_full_packet_lengths(plaintext_per_packet)
            .split_into_fragments(self.rng(), plaintext_per_packet)
    }
}

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
#[derive(Clone)]
#[must_use]
pub struct MessagePreparer<R> {
    /// Instance of a cryptographically secure random number generator.
    rng: R,

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

    nonce: i32,
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
        let mut rng = rng;
        let nonce = rng.gen();
        MessagePreparer {
            rng,
            sender_address,
            average_packet_delay,
            average_ack_delay,
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
            nonce,
        }
    }

    /// Allows setting non-default number of expected mix hops in the network.
    pub fn with_mix_hops(mut self, hops: u8) -> Self {
        self.num_mix_hops = hops;
        self
    }

    /// Overwrites existing sender address with the provided value.
    pub fn set_sender_address(&mut self, sender_address: Recipient) {
        self.sender_address = sender_address;
    }

    pub fn generate_reply_surbs(
        &mut self,
        amount: usize,
        topology: &NymTopology,
    ) -> Result<Vec<ReplySurb>, NymTopologyError> {
        let mut reply_surbs = Vec::with_capacity(amount);
        for _ in 0..amount {
            let reply_surb = ReplySurb::construct(
                &mut self.rng,
                &self.sender_address,
                self.average_packet_delay,
                topology,
            )?;
            reply_surbs.push(reply_surb)
        }

        Ok(reply_surbs)
    }

    pub fn prepare_reply_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        reply_surb: ReplySurb,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, NymTopologyError> {
        let sender = self.sender_address;

        <Self as FragmentPreparer>::prepare_reply_chunk_for_sending(
            self,
            fragment,
            topology,
            ack_key,
            reply_surb,
            &sender,
            packet_type,
        )
    }

    pub fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        packet_recipient: &Recipient,
        packet_type: PacketType,
        mix_hops: Option<u8>,
    ) -> Result<PreparedFragment, NymTopologyError> {
        let sender = self.sender_address;

        <Self as FragmentPreparer>::prepare_chunk_for_sending(
            self,
            fragment,
            topology,
            ack_key,
            &sender,
            packet_recipient,
            packet_type,
            mix_hops,
        )
    }

    /// Construct an acknowledgement SURB for the given [`FragmentIdentifier`]
    pub fn generate_surb_ack(
        &mut self,
        fragment_id: FragmentIdentifier,
        topology: &NymTopology,
        ack_key: &AckKey,
        packet_type: PacketType,
    ) -> Result<SurbAck, NymTopologyError> {
        let sender = self.sender_address;
        <Self as FragmentPreparer>::generate_surb_ack(
            self,
            &sender,
            fragment_id,
            topology,
            ack_key,
            packet_type,
        )
    }

    pub fn pad_and_split_message(
        &mut self,
        message: NymMessage,
        packet_size: PacketSize,
    ) -> Vec<Fragment> {
        <Self as FragmentPreparer>::pad_and_split_message(self, message, packet_size)
    }
}

impl<R: CryptoRng + Rng> FragmentPreparer for MessagePreparer<R> {
    type Rng = R;

    fn rng(&mut self) -> &mut Self::Rng {
        &mut self.rng
    }

    fn num_mix_hops(&self) -> u8 {
        self.num_mix_hops
    }

    fn average_packet_delay(&self) -> Duration {
        self.average_packet_delay
    }

    fn average_ack_delay(&self) -> Duration {
        self.average_ack_delay
    }

    fn nonce(&self) -> i32 {
        self.nonce
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
