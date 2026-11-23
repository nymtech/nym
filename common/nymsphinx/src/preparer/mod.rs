// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymPayloadBuilder;
use crate::message::{ACK_OVERHEAD, NymMessage};
use nym_crypto::Digest;
use nym_crypto::asymmetric::x25519;
use nym_lp_data::packet::LpFrame;
use nym_lp_data::packet::frame::{ForwardSphinxFrameAttributes, LpFrameKind};
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_acknowledgements::surb_ack::SurbAck;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_anonymous_replies::reply_surb::ReplySurb;
use nym_sphinx_chunking::fragment::{Fragment, FragmentIdentifier};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{PacketType, ReplySurbKeyDigestAlgorithm, SphinxKeyRotation};
use nym_sphinx_types::{
    Delay, Node as SphinxNode, NymPacket, NymPacketError, PAYLOAD_OVERHEAD_SIZE,
};
use nym_topology::lp::LpRouteProvider;
use nym_topology::{NodeId, NymRouteProvider, NymTopologyError};
use rand::{CryptoRng, Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tracing::*;

use nym_sphinx_anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx_chunking::monitoring;
use std::time::Duration;

pub(crate) mod payload;

/// Fully packed and prepared [`Fragment`] that is sent to its first hop inside an LP frame.
///
/// No `total_delay` and no `fragment_identifier`: both exist to track an acknowledgement, and this
/// path carries none.
pub struct PreparedLpFragment {
    /// The packet and the address of the node it goes to first.
    pub mix_packet: MixPacket,

    /// That same first hop, named the way an LP frame names it.
    pub first_hop_id: NodeId,
}

impl PreparedLpFragment {
    /// Wrap the packet in the frame that tells a gateway to forward it.
    pub fn into_lp_frame(self) -> Result<LpFrame, NymPacketError> {
        let attributes = ForwardSphinxFrameAttributes {
            key_rotation: self.mix_packet.key_rotation(),
            next_hop: self.first_hop_id,
        };

        Ok(LpFrame::new_with_attributes(
            LpFrameKind::ForwardSphinxPacket,
            attributes,
            self.mix_packet.into_packet().to_bytes()?,
        ))
    }
}

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

    fn mix_hops_disabled(&self) -> bool {
        // Unless otherwise configured, mix hops are enabled
        false
    }

    fn deterministic_route_selection(&self) -> bool;
    fn rng(&mut self) -> &mut Self::Rng;
    fn nonce(&self) -> i32;
    fn average_packet_delay(&self) -> Duration;
    fn average_ack_delay(&self) -> Duration;

    fn generate_surb_ack_legacy(
        &mut self,
        recipient: &Recipient,
        fragment_id: FragmentIdentifier,
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        packet_type: PacketType,
    ) -> Result<SurbAck, NymTopologyError> {
        let ack_delay = self.average_ack_delay();
        let disable_mix_hops = self.mix_hops_disabled();

        SurbAck::construct_legacy(
            self.rng(),
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
        let surb_ack = self.generate_surb_ack_legacy(
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

    // r[impl cover.same-format]
    // r[impl sphinx.route]
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
        let surb_ack = self.generate_surb_ack_legacy(
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
            topology.random_route_to_egress(self.rng(), destination)?
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
            PacketType::Mix => NymPacket::sphinx_build_legacy(
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
        topology: &LpRouteProvider,
        packet_recipient: &Recipient,
    ) -> Result<PreparedLpFragment, NymTopologyError> {
        debug!("Preparing chunk for LP sending");

        let fragment_header = fragment.header();
        let destination = packet_recipient.gateway();
        monitoring::fragment_sent(&fragment, self.nonce(), destination);

        // the fragment fills the sphinx plaintext exactly, so the payload is that plus what the
        // sphinx layer wraps it in - which is what looking the size up as a `PacketSize` would
        // arrive at, without requiring it to be one of that fixed set. On this path it is not: LP
        // chunks to whatever its frame budget leaves.
        let payload_size = fragment.serialized_size() + PAYLOAD_OVERHEAD_SIZE;

        let sphinx_key_rotation = SphinxKeyRotation::from(topology.current_key_rotation());

        let packet_payload = fragment.into_bytes();

        // generate pseudorandom route for the packet. Unless mix hops are disabled then build an empty route.

        trace!("Preparing chunk for sending");
        // the path runs through the mix layers and ends at the recipient's egress gateway, which is
        // the hop that can actually reach them
        let mix_path = if self.mix_hops_disabled() {
            // the egress is the whole path
            topology.empty_path_to_egress(destination)?
        } else if self.deterministic_route_selection() {
            trace!("using deterministic route selection");
            let seed = fragment_header.seed().wrapping_mul(self.nonce());
            let mut rng = ChaCha8Rng::seed_from_u64(seed as u64);
            topology.random_path_to_egress(&mut rng, destination)?
        } else {
            trace!("using pseudorandom route selection");
            topology.random_path_to_egress(self.rng(), destination)?
        };

        let first_hop_id = mix_path
            .first()
            .ok_or(NymTopologyError::NoMixnodesAvailable)?
            .node_id;

        // an `LpRoutingNode` carries no mix address at all, so this cannot name the legacy listener
        let mut route = mix_path
            .into_iter()
            .map(Into::into)
            .collect::<Vec<SphinxNode>>();

        // the recipient is a hop, not merely the destination: on this path it is the one that
        // performs final-hop processing
        // SAFETY: a client address fits the sphinx addressing scheme
        #[expect(clippy::unwrap_used)]
        route.push(SphinxNode::new(
            packet_recipient.as_sphinx_hop().try_into().unwrap(),
            (*packet_recipient.encryption_key()).into(),
        ));

        let delays =
            nym_sphinx_routing::generate_hop_delays(self.average_packet_delay(), route.len());

        // create the actual sphinx packet here. With valid route and correct payload size,
        // there's absolutely no reason for this call to fail.
        let packet = NymPacket::sphinx_build(
            payload_size,
            packet_payload,
            &route,
            &packet_recipient.as_sphinx_destination(),
            &delays,
        )?;

        // from the previously constructed route extract the first hop
        // SAFETY: the route is non-empty, having just been built with at least the recipient
        #[expect(clippy::unwrap_used)]
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

    nonce: i32,

    /// Indicates whether to mix hops or not. If mix hops are enabled, traffic
    /// will be routed as usual, to the entry gateway, through three mix nodes, egressing
    /// through the exit gateway. If mix hops are disabled, traffic will be routed directly
    /// from the entry gateway to the exit gateway, bypassing the mix nodes.
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
        disable_mix_hops: bool,
    ) -> Self {
        let mut rng = rng;
        let nonce = rng.random();
        MessagePreparer {
            rng,
            deterministic_route_selection,
            sender_address,
            average_packet_delay,
            average_ack_delay,
            nonce,
            disable_mix_hops,
        }
    }

    /// Overwrites existing sender address with the provided value.
    pub fn set_sender_address(&mut self, sender_address: Recipient) {
        self.sender_address = sender_address;
    }

    // r[impl delay.acks-replies-cover]
    pub fn generate_reply_surbs(
        &mut self,
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
        topology: &LpRouteProvider,
        packet_recipient: &Recipient,
    ) -> Result<PreparedLpFragment, NymTopologyError> {
        <Self as FragmentPreparer>::prepare_chunk_for_lp(self, fragment, topology, packet_recipient)
    }

    /// Construct an acknowledgement SURB for the given [`FragmentIdentifier`]
    pub fn generate_surb_ack_legacy(
        &mut self,
        fragment_id: FragmentIdentifier,
        topology: &NymRouteProvider,
        ack_key: &AckKey,
        packet_type: PacketType,
    ) -> Result<SurbAck, NymTopologyError> {
        let sender = self.sender_address;
        <Self as FragmentPreparer>::generate_surb_ack_legacy(
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::receiver::{MessageReceiver, SphinxMessageReceiver};
    use nym_crypto::asymmetric::ed25519;
    use nym_sphinx_acknowledgements::identifier::recover_identifier;
    use nym_sphinx_anonymous_replies::requests::ReplyMessage;
    use nym_sphinx_framing::packet::FramedNymPacket;
    use nym_sphinx_framing::processing::{
        MixProcessingResultData, ProcessedFinalHop, process_framed_packet,
    };
    use nym_topology::{
        CachedEpochRewardedSet, NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles,
    };
    use std::net::SocketAddr;

    const EGRESS: u8 = 4;
    const CLIENT: u8 = 10;
    const ONE_HOUR: Duration = Duration::from_secs(3600);

    fn identity(seed: u8) -> ed25519::PublicKey {
        ed25519::PrivateKey::from_bytes(&[seed; 32])
            .unwrap()
            .public_key()
    }

    fn sphinx_key(node: u8) -> x25519::PrivateKey {
        x25519::PrivateKey::from_secret([node; 32])
    }

    fn mix_address(node: u8) -> NymNodeRoutingAddress {
        SocketAddr::from(([10, 0, 0, node], 1789)).into()
    }

    /// Nodes 1, 2 and 3 are the mixes of layers 1, 2 and 3, and node 4 is the egress gateway.
    fn topology() -> NymRouteProvider {
        let mut rewarded_set = CachedEpochRewardedSet::default();
        rewarded_set.layer1.insert(1);
        rewarded_set.layer2.insert(2);
        rewarded_set.layer3.insert(3);
        rewarded_set.exit_gateways.insert(EGRESS as u32);
        let nodes = (1..=EGRESS)
            .map(|node| RoutingNode {
                node_id: node as u32,
                mix_host: SocketAddr::from(([10, 0, 0, node], 1789)),
                entry: None,
                identity_key: identity(node),
                sphinx_key: sphinx_key(node).public_key(),
                supported_roles: SupportedRoles {
                    mixnode: true,
                    mixnet_entry: true,
                    mixnet_exit: true,
                },
            })
            .collect();
        NymTopology::new(NymTopologyMetadata::default(), rewarded_set, nodes).into()
    }

    /// The client sends to itself through the egress.
    fn client() -> Recipient {
        let encryption_key = sphinx_key(CLIENT).public_key();
        Recipient::new(identity(CLIENT), encryption_key, identity(EGRESS))
    }

    fn ack_key() -> AckKey {
        AckKey::new(&mut ChaCha8Rng::seed_from_u64(1))
    }

    fn preparer(
        average_packet_delay: Duration,
        average_ack_delay: Duration,
        disable_mix_hops: bool,
    ) -> MessagePreparer<ChaCha8Rng> {
        MessagePreparer::new(
            ChaCha8Rng::seed_from_u64(42),
            false,
            client(),
            average_packet_delay,
            average_ack_delay,
            disable_mix_hops,
        )
    }

    fn prepare_short_message(
        preparer: &mut MessagePreparer<ChaCha8Rng>,
        ack_key: &AckKey,
    ) -> (Fragment, PreparedFragment) {
        let mut fragments = preparer.pad_and_split_message(
            NymMessage::new_plain(vec![42; 10]),
            PacketSize::RegularPacket,
        );
        assert_eq!(fragments.len(), 1);
        let fragment = fragments.pop().unwrap();
        let prepared = preparer
            .prepare_chunk_for_sending(
                fragment.clone(),
                &topology(),
                ack_key,
                &client(),
                PacketType::Mix,
            )
            .unwrap();
        (fragment, prepared)
    }

    struct Delivery {
        hops: Vec<NymNodeRoutingAddress>,
        delays: Vec<Delay>,
        final_hop: ProcessedFinalHop,
    }

    /// Processes the packet at each hop with the Sphinx key of that hop, as the nodes do.
    fn deliver(mut packet: MixPacket) -> Delivery {
        let mut hops = vec![packet.next_hop()];
        let mut delays = Vec::new();
        loop {
            let node = (1..=EGRESS)
                .find(|node| mix_address(*node) == packet.next_hop())
                .expect("the packet left the topology");
            let framed = FramedNymPacket::from_mix_packet(packet, false);
            match process_framed_packet(framed, sphinx_key(node).inner())
                .unwrap()
                .processing_data
            {
                MixProcessingResultData::ForwardHop {
                    packet: next,
                    delay,
                } => {
                    hops.push(next.next_hop());
                    delays.push(delay.unwrap());
                    packet = next;
                }
                MixProcessingResultData::FinalHop { final_hop_data } => {
                    return Delivery {
                        hops,
                        delays,
                        final_hop: final_hop_data,
                    };
                }
            }
        }
    }

    // r[verify sphinx.route]
    #[test]
    fn a_data_packet_visits_one_mix_per_layer_then_the_egress() {
        for (disable_mix_hops, expected_nodes) in
            [(false, vec![1, 2, 3, EGRESS]), (true, vec![EGRESS])]
        {
            let mut preparer = preparer(Duration::ZERO, Duration::ZERO, disable_mix_hops);
            let (_, prepared) = prepare_short_message(&mut preparer, &ack_key());

            let expected_hops: Vec<_> = expected_nodes.into_iter().map(mix_address).collect();
            assert_eq!(deliver(prepared.mix_packet).hops, expected_hops);
        }
    }

    // r[verify sphinx.route]
    #[test]
    fn with_mix_hops_disabled_the_surb_ack_goes_to_the_egress_only() {
        let mut preparer = preparer(Duration::ZERO, Duration::ZERO, true);
        let (_, prepared) = prepare_short_message(&mut preparer, &ack_key());

        let forward_ack = deliver(prepared.mix_packet)
            .final_hop
            .forward_ack
            .expect("the egress did not forward a SURB-ACK");
        assert_eq!(deliver(forward_ack).hops, vec![mix_address(EGRESS)]);
    }

    // r[verify sphinx.payload-fills]
    #[test]
    fn a_short_message_travels_in_one_full_regular_packet() {
        let mut preparer = preparer(Duration::ZERO, Duration::ZERO, false);
        let (fragment, prepared) = prepare_short_message(&mut preparer, &ack_key());
        assert_eq!(
            prepared.mix_packet.packet().len(),
            PacketSize::RegularPacket.size()
        );

        // the egress removed the SURB-ACK, so the message holds the ephemeral key and the fragment
        let mut message = deliver(prepared.mix_packet).final_hop.message;
        assert_eq!(
            message.len(),
            x25519::PUBLIC_KEY_SIZE + fragment.serialized_size()
        );
        let receiver = SphinxMessageReceiver::new();
        let fragment_data = receiver
            .recover_plaintext_from_regular_packet(&sphinx_key(CLIENT), &mut message)
            .unwrap();
        assert!(receiver.recover_fragment(fragment_data).unwrap() == fragment);
    }

    // r[verify sphinx.ack-packet]
    #[test]
    fn the_egress_forwards_a_surb_ack_that_brings_back_the_fragment_id() {
        let ack_key = ack_key();
        let mut preparer = preparer(Duration::ZERO, Duration::ZERO, false);
        let (fragment, prepared) = prepare_short_message(&mut preparer, &ack_key);

        let forward_ack = deliver(prepared.mix_packet)
            .final_hop
            .forward_ack
            .expect("the egress did not forward a SURB-ACK");
        assert_eq!(forward_ack.packet().len(), PacketSize::AckPacket.size());

        let ack = deliver(forward_ack);
        let expected_hops: Vec<_> = [1, 2, 3, EGRESS].into_iter().map(mix_address).collect();
        assert_eq!(ack.hops, expected_hops);
        assert!(ack.final_hop.forward_ack.is_none());
        // a 16-byte IV, then the encrypted 5-byte fragment id
        assert_eq!(ack.final_hop.message.len(), 16 + 5);
        assert_eq!(
            recover_identifier(&ack_key, &ack.final_hop.message),
            Some(fragment.fragment_identifier().to_bytes())
        );
    }

    // r[verify delay.acks-replies-cover]
    #[test]
    fn a_surb_ack_takes_its_delays_from_the_ack_delay() {
        for (packet_delay, ack_delay) in [(ONE_HOUR, Duration::ZERO), (Duration::ZERO, ONE_HOUR)] {
            let mut preparer = preparer(packet_delay, ack_delay, false);
            let (_, prepared) = prepare_short_message(&mut preparer, &ack_key());

            let data = deliver(prepared.mix_packet);
            let ack = deliver(data.final_hop.forward_ack.unwrap());
            assert_eq!(data.delays.len(), 3);
            assert_eq!(ack.delays.len(), 3);
            assert!(
                data.delays
                    .iter()
                    .all(|delay| (delay.to_nanos() == 0) == packet_delay.is_zero())
            );
            assert!(
                ack.delays
                    .iter()
                    .all(|delay| (delay.to_nanos() == 0) == ack_delay.is_zero())
            );
        }
    }

    // r[verify delay.acks-replies-cover]
    #[test]
    fn a_reply_keeps_the_delays_that_the_creator_of_the_surb_drew() {
        let topology = topology();
        for (creator_delay, destination_delay) in
            [(Duration::ZERO, ONE_HOUR), (ONE_HOUR, Duration::ZERO)]
        {
            let reply_surb = preparer(creator_delay, Duration::ZERO, false)
                .generate_reply_surbs(1, &topology)
                .unwrap()
                .pop()
                .unwrap();

            let mut destination = preparer(destination_delay, Duration::ZERO, false);
            let fragment = destination
                .pad_and_split_message(
                    NymMessage::new_reply(ReplyMessage::new_data_message(vec![42; 10])),
                    PacketSize::RegularPacket,
                )
                .pop()
                .unwrap();
            let prepared = destination
                .prepare_reply_chunk_for_sending(
                    fragment,
                    &topology,
                    &ack_key(),
                    reply_surb,
                    PacketType::Mix,
                )
                .unwrap();

            let reply = deliver(prepared.mix_packet);
            assert_eq!(reply.delays.len(), 3);
            assert!(
                reply
                    .delays
                    .iter()
                    .all(|delay| (delay.to_nanos() == 0) == creator_delay.is_zero())
            );
        }
    }
}
