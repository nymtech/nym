// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymPayloadBuilder;
use crate::message::{ACK_OVERHEAD, NymMessage};
use nym_crypto::Digest;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_acknowledgements::surb_ack::SurbAck;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_anonymous_replies::reply_surb::ReplySurb;
use nym_sphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{PacketType, ReplySurbKeyDigestAlgorithm, SphinxKeyRotation};
use nym_sphinx_types::{Delay, Node as SphinxNode, NymPacket};
use nym_topology::{NodeId, NymRouteProvider, NymTopologyError};
use rand::{CryptoRng, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tracing::*;

use nym_sphinx_anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx_chunking::monitoring;
use std::time::Duration;

pub(crate) mod payload;

/// Represents fully packed and prepared [`Fragment`] that can be sent through the mix network.
/// A fragment prepared for the Lewes Protocol path.
///
/// No `total_delay` and no `fragment_identifier`: both exist to track an acknowledgement, and this
/// path carries none.
pub struct PreparedLpFragment {
    /// The packet and the address of the node it goes to first.
    pub mix_packet: MixPacket,

    /// That same first hop, named the way an LP frame names it.
    pub first_hop_id: NodeId,
}

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

    fn use_legacy_sphinx_format(&self) -> bool;
    fn mix_hops_disabled(&self) -> bool {
        // Unless otherwise configured, mix hops are enabled
        false
    }

    fn deterministic_route_selection(&self) -> bool;
    fn rng(&mut self) -> &mut Self::Rng;
    fn nonce(&self) -> i32;
    fn average_packet_delay(&self) -> Duration;
    fn average_ack_delay(&self) -> Duration;

    fn generate_surb_ack(
        &mut self,
        recipient: &Recipient,
        fragment_id: FragmentIdentifier,
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        packet_type: PacketType,
    ) -> Result<SurbAck, NymTopologyError> {
        let ack_delay = self.average_ack_delay();
        let use_legacy_sphinx_format = self.use_legacy_sphinx_format();
        let disable_mix_hops = self.mix_hops_disabled();

        SurbAck::construct(
            self.rng(),
            use_legacy_sphinx_format,
            recipient,
            ack_key,
            fragment_id.to_bytes(),
            ack_delay,
            topology,
            packet_type,
            disable_mix_hops,
        )
    }

    /// The procedure is as follows:
    /// For each fragment:
    /// - compute SURB_ACK
    /// - generate (x, g^x)
    /// - obtain key k from the reply-surb which was computed as follows:
    ///   k = KDF(remote encryption key ^ x) this is equivalent to KDF( dh(remote, x) )
    /// - compute v_b = AES-128-CTR(k, serialized_fragment)
    /// - compute vk_b = H(k) || v_b
    /// - compute sphinx_plaintext = SURB_ACK || H(k) || v_b
    /// - compute sphinx_packet by applying the reply surb on the sphinx_plaintext
    fn prepare_reply_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        reply_surb: ReplySurbWithKeyRotation,
        packet_sender: &Recipient,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, NymTopologyError> {
        debug!("Preparing reply chunk for sending");

        // each reply attaches the digest of the encryption key so that the recipient could
        // lookup correct key for decryption,
        let reply_overhead = ReplySurbKeyDigestAlgorithm::output_size();
        let PacketType::Mix = packet_type else {
            return Err(NymTopologyError::PacketTypeNotSupported);
        };
        let expected_plaintext = fragment.serialized_size() + ACK_OVERHEAD + reply_overhead;

        // the reason we're unwrapping (or rather 'expecting') here rather than handling the error
        // more gracefully is that this error should never be reached as it implies incorrect chunking
        // reply packets are always Sphinx
        let packet_size = PacketSize::get_type_from_plaintext(expected_plaintext, PacketType::Mix)
            .expect("the message has been incorrectly fragmented");

        // this is not going to be accurate by any means. but that's the best estimation we can do
        let expected_forward_delay =
            Delay::new_from_millis((self.average_packet_delay().as_millis() * 3) as u64);

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
        let applied_surb = reply_surb
            .apply_surb(packet_payload, packet_size, packet_type)
            .unwrap();

        Ok(PreparedFragment {
            // the round-trip delay is the sum of delays of all hops on the forward route as
            // well as the total delay of the ack packet.
            // we don't know the delays inside the reply surbs so we use best-effort estimation from our poisson distribution
            total_delay: expected_forward_delay + ack_delay,
            mix_packet: MixPacket::from_applied_surb(applied_surb, packet_type),
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
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        packet_sender: &Recipient,
        packet_recipient: &Recipient,
        packet_type: PacketType,
    ) -> Result<PreparedFragment, NymTopologyError> {
        debug!("Preparing chunk for sending");
        // each plain or repliable packet (i.e. not a reply) attaches an ephemeral public key so that the recipient
        // could perform diffie-hellman with its own keys followed by a kdf to re-derive
        // the packet encryption key

        let fragment_header = fragment.header();
        let destination = packet_recipient.gateway();
        monitoring::fragment_sent(&fragment, self.nonce(), destination);

        let non_reply_overhead = x25519::PUBLIC_KEY_SIZE;
        let PacketType::Mix = packet_type else {
            return Err(NymTopologyError::PacketTypeNotSupported);
        };
        let expected_plaintext = fragment.serialized_size() + ACK_OVERHEAD + non_reply_overhead;

        // the reason we're unwrapping (or rather 'expecting') here rather than handling the error
        // more gracefully is that this error should never be reached as it implies incorrect chunking
        let packet_size = PacketSize::get_type_from_plaintext(expected_plaintext, packet_type)
            .expect("the message has been incorrectly fragmented");

        let rotation_id = topology.current_key_rotation();
        let sphinx_key_rotation = SphinxKeyRotation::from(rotation_id);

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

        // generate pseudorandom route for the packet. Unless mix hops are disabled then build an empty route.
        trace!("Preparing chunk for sending");
        let route = if self.mix_hops_disabled() {
            topology.empty_route_to_egress(destination)?
        } else if self.deterministic_route_selection() {
            trace!("using deterministic route selection");
            let seed = fragment_header.seed().wrapping_mul(self.nonce());
            let mut rng = ChaCha8Rng::seed_from_u64(seed as u64);
            topology.random_route_to_egress(&mut rng, destination)?
        } else {
            trace!("using pseudorandom route selection");
            let mut rng = self.rng();
            topology.random_route_to_egress(&mut rng, destination)?
        };

        let destination = packet_recipient.as_sphinx_destination();

        // including set of delays
        let delays =
            nym_sphinx_routing::generate_hop_delays(self.average_packet_delay(), route.len());

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        #[allow(deprecated)]
        let packet = match packet_type {
            PacketType::Outfox => return Err(NymTopologyError::PacketTypeNotSupported),
            PacketType::Mix => NymPacket::sphinx_build(
                self.use_legacy_sphinx_format(),
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
            mix_packet: MixPacket::new(first_hop_address, packet, packet_type, sphinx_key_rotation),
            fragment_identifier,
        })
    }

    /// As [`Self::prepare_chunk_for_sending`], but for the Lewes Protocol path.
    ///
    /// Three differences, all following from the recipient client being the **last sphinx hop**
    /// rather than a destination behind a gateway that does final-hop processing:
    ///
    /// 1. **No SURB-ack.** There is no gateway to peel one off and send it back, and the recipient
    ///    reads the payload directly - a leading ack would simply corrupt the message. The packet
    ///    is sized without [`ACK_OVERHEAD`] accordingly.
    /// 2. **No payload wrapper.** The ephemeral-DH layer that [`NymPayloadBuilder`] adds exists so
    ///    that a gateway performing final-hop processing cannot read the message. Here the
    ///    recipient performs it, so sphinx's own final layer already gives that and the payload is
    ///    the serialised [`Fragment`] alone.
    /// 3. **The route ends at the recipient**, appended as a hop, rather than at its egress
    ///    gateway.
    ///
    /// Returns the first hop's [`NodeId`] alongside the packet: an LP frame names its next hop by
    /// id, not by address, and the id is known here because the route was chosen from
    /// [`RoutingNode`]s. Recovering it afterwards would mean searching the topology by address,
    /// which it does not index.
    ///
    /// [`RoutingNode`]: nym_topology::RoutingNode
    fn prepare_chunk_for_lp(
        &mut self,
        fragment: Fragment,
        topology: &NymRouteProvider,
        packet_recipient: &Recipient,
    ) -> Result<PreparedLpFragment, NymTopologyError> {
        debug!("Preparing chunk for LP sending");

        let fragment_header = fragment.header();
        let destination = packet_recipient.gateway();
        monitoring::fragment_sent(&fragment, self.nonce(), destination);

        // as in `prepare_chunk_for_sending`: reaching this means the message was chunked wrongly
        let packet_size =
            PacketSize::get_type_from_plaintext(fragment.serialized_size(), PacketType::Mix)
                .expect("the message has been incorrectly fragmented");

        let sphinx_key_rotation = SphinxKeyRotation::from(topology.current_key_rotation());

        let packet_payload = fragment.into_bytes();

        // generate pseudorandom route for the packet. Unless mix hops are disabled then build an empty route.

        trace!("Preparing chunk for sending");
        let mix_path = if self.mix_hops_disabled() {
            topology.empty_path_to_egress(destination)?
        } else if self.deterministic_route_selection() {
            trace!("using deterministic route selection");
            let seed = fragment_header.seed().wrapping_mul(self.nonce());
            let mut rng = ChaCha8Rng::seed_from_u64(seed as u64);
            topology.random_path_to_egress(&mut rng, destination)?.0
        } else {
            trace!("using pseudorandom route selection");
            topology.random_path_to_egress(self.rng(), destination)?.0
        };

        let first_hop_id = mix_path
            .first()
            .ok_or(NymTopologyError::NoMixnodesAvailable)?
            .node_id;

        let mut route = mix_path
            .into_iter()
            .map(Into::into)
            .collect::<Vec<SphinxNode>>();

        // the recipient is a hop, not merely the destination: on this path it is the one that
        // performs final-hop processing
        // SAFETY: a client address fits the sphinx addressing scheme
        #[allow(clippy::unwrap_used)]
        route.push(SphinxNode::new(
            packet_recipient.as_sphinx_hop().try_into().unwrap(),
            (*packet_recipient.encryption_key()).into(),
        ));

        let delays =
            nym_sphinx_routing::generate_hop_delays(self.average_packet_delay(), route.len());

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        let packet = NymPacket::sphinx_build(
            self.use_legacy_sphinx_format(),
            packet_size.payload_size(),
            packet_payload,
            &route,
            &packet_recipient.as_sphinx_destination(),
            &delays,
        )?;

        // from the previously constructed route extract the first hop
        // SAFETY: the route is non-empty, having just been built with at least the recipient
        #[allow(clippy::unwrap_used)]
        let first_hop_address =
            NymNodeRoutingAddress::try_from(route.first().unwrap().address).unwrap();

        Ok(PreparedLpFragment {
            mix_packet: MixPacket::new(
                first_hop_address,
                packet,
                PacketType::Mix,
                sphinx_key_rotation,
            ),
            first_hop_id,
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

/// Prepares the message that is to be sent through the mix network.
///
/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
#[derive(Clone)]
#[must_use]
pub struct MessagePreparer<R> {
    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Specify whether route selection should be determined by the packet header.
    deterministic_route_selection: bool,

    /// Address of this client which also represent an address to which all acknowledgements
    /// and surb-based are going to be sent.
    sender_address: Recipient,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Specify whether any constructed packets should use the legacy format,
    /// where the payload keys are explicitly attached rather than using the seeds
    use_legacy_sphinx_format: bool,

    nonce: i32,

    /// Indicates whether to mix hops or not. If mix hops are enabled, traffic
    /// will be routed as usual, to the entry gateway, through three mix nodes, egressing
    /// through the exit gateway. If mix hops are disabled, traffic will be routed directly
    /// from the entry gateway to the exit gateway, bypassing the mix nodes.
    ///
    /// This overrides the `use_legacy_sphinx_format` setting as reduced/disabled mix hops
    /// requires use of the updated SURB packet format.
    pub disable_mix_hops: bool,
}

impl<R> MessagePreparer<R>
where
    R: CryptoRng + Rng,
{
    pub fn new(
        rng: R,
        deterministic_route_selection: bool,
        sender_address: Recipient,
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        use_legacy_sphinx_format: bool,
        disable_mix_hops: bool,
    ) -> Self {
        let mut rng = rng;
        let nonce = rng.r#gen();
        MessagePreparer {
            rng,
            deterministic_route_selection,
            sender_address,
            average_packet_delay,
            average_ack_delay,
            use_legacy_sphinx_format,
            nonce,
            disable_mix_hops,
        }
    }

    /// Overwrites existing sender address with the provided value.
    pub fn set_sender_address(&mut self, sender_address: Recipient) {
        self.sender_address = sender_address;
    }

    pub fn generate_reply_surbs(
        &mut self,
        use_legacy_reply_surb_format: bool,
        amount: usize,
        topology: &NymRouteProvider,
    ) -> Result<Vec<ReplySurbWithKeyRotation>, NymTopologyError> {
        let mut reply_surbs = Vec::with_capacity(amount);
        let disabled_mix_hops = self.mix_hops_disabled();

        let key_rotation = SphinxKeyRotation::from(topology.current_key_rotation());

        for _ in 0..amount {
            let reply_surb = ReplySurb::construct(
                &mut self.rng,
                &self.sender_address,
                self.average_packet_delay,
                use_legacy_reply_surb_format,
                topology,
                disabled_mix_hops,
            )?
            .with_key_rotation(key_rotation);
            reply_surbs.push(reply_surb)
        }

        Ok(reply_surbs)
    }

    pub fn prepare_reply_chunk_for_sending(
        &mut self,
        fragment: Fragment,
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        reply_surb: ReplySurbWithKeyRotation,
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
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        packet_recipient: &Recipient,
        packet_type: PacketType,
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
        )
    }

    /// Prepare a chunk for the Lewes Protocol path: no SURB-ack, and the recipient is the last hop.
    ///
    /// See [`FragmentPreparer::prepare_chunk_for_lp`].
    pub fn prepare_chunk_for_lp(
        &mut self,
        fragment: Fragment,
        topology: &NymRouteProvider,
        packet_recipient: &Recipient,
    ) -> Result<PreparedLpFragment, NymTopologyError> {
        <Self as FragmentPreparer>::prepare_chunk_for_lp(self, fragment, topology, packet_recipient)
    }

    /// Construct an acknowledgement SURB for the given [`FragmentIdentifier`]
    pub fn generate_surb_ack(
        &mut self,
        fragment_id: FragmentIdentifier,
        topology: &NymRouteProvider,
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

    fn mix_hops_disabled(&self) -> bool {
        self.disable_mix_hops
    }

    fn use_legacy_sphinx_format(&self) -> bool {
        self.use_legacy_sphinx_format
    }

    fn deterministic_route_selection(&self) -> bool {
        self.deterministic_route_selection
    }

    fn rng(&mut self) -> &mut Self::Rng {
        &mut self.rng
    }

    fn nonce(&self) -> i32 {
        self.nonce
    }

    fn average_packet_delay(&self) -> Duration {
        self.average_packet_delay
    }

    fn average_ack_delay(&self) -> Duration {
        self.average_ack_delay
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
