// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::message::NymMessage;
use crate::NymsphinxPayloadBuilder;
use nym_sphinx_acknowledgements::surb_ack::SurbAck;
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_anonymous_replies::reply_surb::ReplySurb;
use nym_sphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::DEFAULT_NUM_MIX_HOPS;
use nym_sphinx_types::builder::SphinxPacketBuilder;
use nym_sphinx_types::{delays, Delay};
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::time::Duration;
use nym_topology::{NymTopology, NymTopologyError};

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

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
#[derive(Clone)]
#[must_use]
pub struct MessagePreparer<R> {
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
    pub fn with_custom_real_message_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.packet_size = packet_size;
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
    pub fn prepare_reply_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        reply_surb: ReplySurb,
    ) -> Result<PreparedFragment, NymTopologyError> {
        // this is not going to be accurate by any means. but that's the best estimation we can do
        let expected_forward_delay = Delay::new_from_millis(
            (self.average_packet_delay.as_millis() * self.num_mix_hops as u128) as u64,
        );

        let fragment_identifier = fragment.fragment_identifier();

        // create an ack
        let surb_ack = self.generate_surb_ack(fragment_identifier, topology, ack_key)?;
        let ack_delay = surb_ack.expected_total_delay();

        let packet_payload = NymsphinxPayloadBuilder::new(fragment, surb_ack)
            .build_reply(reply_surb.encryption_key());

        // the unwrap here is fine as the failures can only originate from attempting to use invalid payload lenghts
        // and we just very carefully constructed a (presumably) valid one
        let (sphinx_packet, first_hop_address) = reply_surb
            .apply_surb(packet_payload, Some(self.packet_size))
            .unwrap();

        Ok(PreparedFragment {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            // we don't know the delays inside the reply surbs so we use best-effort estimation from our poisson distribution
            total_delay: expected_forward_delay + ack_delay,
            mix_packet: MixPacket::new(first_hop_address, sphinx_packet, Default::default()),
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
    pub fn prepare_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymTopology,
        ack_key: &AckKey,
        packet_recipient: &Recipient,
    ) -> Result<PreparedFragment, NymTopologyError> {
        let fragment_identifier = fragment.fragment_identifier();

        // create an ack
        let surb_ack = self.generate_surb_ack(fragment_identifier, topology, ack_key)?;
        let ack_delay = surb_ack.expected_total_delay();

        let packet_payload = NymsphinxPayloadBuilder::new(fragment, surb_ack)
            .build_regular(&mut self.rng, packet_recipient.encryption_key());

        // generate pseudorandom route for the packet
        let route = topology.random_route_to_gateway(
            &mut self.rng,
            self.num_mix_hops,
            packet_recipient.gateway(),
        )?;
        let destination = packet_recipient.as_sphinx_destination();

        // including set of delays
        let delays = delays::generate_from_average_duration(route.len(), self.average_packet_delay);

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        let sphinx_packet = SphinxPacketBuilder::new()
            .with_payload_size(self.packet_size.payload_size())
            .build_packet(packet_payload, &route, &destination, &delays)
            .unwrap();

        // from the previously constructed route extract the first hop
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address).unwrap();

        Ok(PreparedFragment {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            // note that the last hop of the packet is a gateway that does not do any delays
            total_delay: delays.iter().take(delays.len() - 1).sum::<Delay>() + ack_delay,
            mix_packet: MixPacket::new(first_hop_address, sphinx_packet, Default::default()),
            fragment_identifier,
        })
    }

    /// Construct an acknowledgement SURB for the given [`FragmentIdentifier`]
    fn generate_surb_ack(
        &mut self,
        fragment_id: FragmentIdentifier,
        topology: &NymTopology,
        ack_key: &AckKey,
    ) -> Result<SurbAck, NymTopologyError> {
        SurbAck::construct(
            &mut self.rng,
            &self.sender_address,
            ack_key,
            fragment_id.to_bytes(),
            self.average_ack_delay,
            topology,
        )
    }

    pub fn pad_and_split_message(&mut self, message: NymMessage) -> Vec<Fragment> {
        let plaintext_per_packet = message.available_plaintext_per_packet(self.packet_size);

        message
            .pad_to_full_packet_lengths(plaintext_per_packet)
            .split_into_fragments(&mut self.rng, plaintext_per_packet)
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
