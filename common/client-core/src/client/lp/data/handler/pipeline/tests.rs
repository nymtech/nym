// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Bytes in one end of the client, the same bytes out of the other.
//!
//! Both pipelines meet here, and so do the two things that make the LP path its own: a payload that
//! is a bare `Fragment` with no ack and no wrapper, and a sphinx packet that spans more than one LP
//! frame. Neither is visible from either pipeline alone.
//!
//! What sits between them is the network a client cannot test: the gateway decrypting the LP layer
//! and forwarding, and each mix peeling one sphinx layer. Both are done here by hand, with keys this
//! test generated, which is what makes this an assertion about the *format* rather than about any
//! node's behaviour.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use nym_client_core_config_types::DebugConfig;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt_ciphersuite::KEM;
use nym_lp::{LpTransportSession, SessionsMock};
use nym_lp_data::clients::traits::{ClientUnwrappingPipeline, ClientWrappingPipeline};
use nym_lp_data::fragmentation::fragment::Fragment as LpFragment;
use nym_lp_data::fragmentation::reconstruction::MessageReconstructor as LpFrameReconstructor;
use nym_lp_data::packet::frame::{
    ForwardSphinxFrameAttributes, LpFrameKind, SphinxFrameAttributes,
};
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_data::{AddressedTimedData, TimedPayload};
use nym_lp_gateway_client::extract_forwarded_response;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::{NodeAddressBytes, ProcessedPacketData, SphinxPacket};
use nym_topology::{
    CachedEpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles,
};
use rand::rngs::OsRng;
use time::OffsetDateTime;

use super::{LpInboundPipeline, LpOutboundOptions, LpOutboundPipeline};
use crate::client::lp::data::handler::messages::ClientMessage;
use crate::client::lp::data::shared::{LpGatewaySession, LpGatewaySessions, SharedLpDataState};
use crate::client::topology_control::TopologyAccessor;

/// How many mix hops a packet takes, which is what `random_path_to_egress` builds.
const MIX_LAYERS: usize = 3;

/// A node in the test network, with the private key its own layer is peeled with.
struct TestNode {
    routing: RoutingNode,
    sphinx_keys: x25519::KeyPair,
}

impl TestNode {
    fn new(node_id: NodeId, mixnode: bool) -> Self {
        let identity = ed25519::KeyPair::new(&mut OsRng);
        let sphinx_keys = x25519::KeyPair::new(&mut OsRng);

        TestNode {
            routing: RoutingNode {
                node_id,
                // distinct per node, so a peeled packet says which one it is addressed to
                mix_host: format!("10.0.0.{node_id}:1789").parse().unwrap(),
                ip_addresses: Vec::new(),
                entry: None,
                identity_key: *identity.public_key(),
                sphinx_key: *sphinx_keys.public_key(),
                supported_roles: SupportedRoles {
                    mixnode,
                    mixnet_entry: !mixnode,
                    mixnet_exit: !mixnode,
                },
                lp: None,
                build_version: None,
            },
            sphinx_keys,
        }
    }

    /// How a sphinx packet names this node as its next hop.
    fn sphinx_address(&self) -> String {
        let routing: NymNodeRoutingAddress = self.routing.mix_host.into();
        let address: NodeAddressBytes = routing.try_into().unwrap();

        address.as_base58_string()
    }
}

/// Everything the two pipelines need, plus the keys to play the network between them.
struct TestNetwork {
    /// Every node on the route - the mixes and the egress gateway - by the address a sphinx packet
    /// names them with.
    node_keys: HashMap<String, x25519::KeyPair>,

    /// The mixes by id, which is how the LP frame names the first of them.
    mix_addresses: HashMap<NodeId, String>,

    /// Where this client's packets go, and the gateway's half of the session they arrive on.
    gateway_address: SocketAddr,
    gateway_session: LpTransportSession,

    /// Who the message is for, and the key its last sphinx layer is peeled with.
    recipient: Recipient,
    recipient_keys: Arc<x25519::KeyPair>,

    topology_accessor: TopologyAccessor,
    shared_state: Arc<SharedLpDataState>,
}

impl TestNetwork {
    fn new() -> Self {
        // one mix per layer to route through, plus the gateway the recipient sits behind
        let mixes: Vec<_> = (1..=MIX_LAYERS)
            .map(|i| TestNode::new(i as NodeId, true))
            .collect();
        let egress = TestNode::new(MIX_LAYERS as NodeId + 1, false);

        // layer membership is what route selection draws from, so it has to be said explicitly
        let mut rewarded_set = CachedEpochRewardedSet::default();
        for (layer, mix) in mixes.iter().enumerate() {
            let id = mix.routing.node_id;
            match layer {
                0 => rewarded_set.layer1.insert(id),
                1 => rewarded_set.layer2.insert(id),
                _ => rewarded_set.layer3.insert(id),
            };
        }

        let nodes = mixes
            .iter()
            .chain(std::iter::once(&egress))
            .map(|node| node.routing.clone())
            .collect();

        let metadata = NymTopologyMetadata::new(0, 1, OffsetDateTime::now_utc());

        // `true`: the egress role is read off `supported_roles` rather than the rewarded set, which
        // saves this test from modelling gateway assignment as well
        let topology_accessor = TopologyAccessor::new(true);
        topology_accessor.manually_change_topology(NymTopology::new(metadata, rewarded_set, nodes));

        // the recipient is behind the egress gateway, which is how a route to them is found
        let recipient_keys = Arc::new(x25519::KeyPair::new(&mut OsRng));
        let recipient_identity = ed25519::KeyPair::new(&mut OsRng);
        let recipient = Recipient::new(
            *recipient_identity.public_key(),
            *recipient_keys.public_key(),
            egress.routing.identity_key,
        );

        // the session the client would have established with its own gateway at startup
        let sessions = SessionsMock::mock_seeded_post_handshake(42, KEM::default());
        let gateway_address: SocketAddr = "10.0.0.42:51264".parse().unwrap();

        let gateway_sessions = LpGatewaySessions::default();
        gateway_sessions.insert(LpGatewaySession {
            session: sessions.initiator,
            data_address: gateway_address,
        });

        TestNetwork {
            mix_addresses: mixes
                .iter()
                .map(|mix| (mix.routing.node_id, mix.sphinx_address()))
                .collect(),
            // the egress gateway peels a layer too, so its key belongs here with the mixes'
            node_keys: mixes
                .into_iter()
                .chain(std::iter::once(egress))
                .map(|node| (node.sphinx_address(), node.sphinx_keys))
                .collect(),
            gateway_address,
            gateway_session: sessions.responder,
            recipient,
            recipient_keys,
            topology_accessor,
            shared_state: Arc::new(SharedLpDataState::new(gateway_sessions)),
        }
    }

    fn outbound(&self) -> LpOutboundPipeline<OsRng> {
        LpOutboundPipeline::new(
            OsRng,
            DebugConfig::default(),
            self.topology_accessor.clone(),
            self.shared_state.clone(),
        )
    }

    fn inbound(&self) -> LpInboundPipeline {
        LpInboundPipeline::new(self.shared_state.clone(), self.recipient_keys.clone())
    }

    /// What the entry gateway does: decrypt the LP layer and read the frame inside.
    fn gateway_unwrap(
        &mut self,
        packets: Vec<AddressedTimedData<EncryptedLpPacket>>,
    ) -> Vec<LpFrame> {
        packets
            .into_iter()
            .map(|packet| {
                assert_eq!(
                    self.gateway_address, packet.dst,
                    "a packet went somewhere other than our own gateway"
                );

                extract_forwarded_response(packet.data.data, &mut self.gateway_session).unwrap()
            })
            .collect()
    }

    /// What each node on the route does: peel its own layer and pass the packet on.
    ///
    /// Returns the packet as it would reach the recipient, after the mix layers and then the
    /// recipient's egress gateway - the hop that can actually reach them.
    fn route_to_recipient(&self, first_hop: NodeId, sphinx_bytes: &[u8]) -> Vec<u8> {
        let mut address = self
            .mix_addresses
            .get(&first_hop)
            .expect("the frame named a first hop that is not a mix in the topology")
            .clone();
        let mut packet = SphinxPacket::from_bytes(sphinx_bytes).unwrap();

        for hop in 0..=MIX_LAYERS {
            let keys = self
                .node_keys
                .get(&address)
                .unwrap_or_else(|| panic!("hop {hop} names a node that is not in the topology"));

            match packet.process(keys.private_key().as_ref()).unwrap().data {
                ProcessedPacketData::ForwardHop {
                    next_hop_packet,
                    next_hop_address,
                    ..
                } => {
                    packet = next_hop_packet;
                    address = next_hop_address.as_base58_string();
                }
                ProcessedPacketData::FinalHop { .. } => {
                    panic!("the packet ended at hop {hop}, before reaching the recipient")
                }
            }
        }

        packet.to_bytes()
    }
}

/// Reassemble the LP frames a sphinx packet was split across.
///
/// A sphinx packet does not fit in one frame, so this is the counterpart of the outbound side's
/// fragmentation - the gateway would do the same before forwarding.
fn rebuild_frames(frames: Vec<LpFrame>, now: Instant) -> Vec<LpFrame> {
    let reconstructor = LpFrameReconstructor::default();
    let mut rebuilt = Vec::new();

    for frame in frames {
        match frame.kind() {
            LpFrameKind::FragmentedData => {
                let fragment = LpFragment::try_from(frame).unwrap();
                if let Some(whole) = reconstructor.insert_new_fragment(fragment, now) {
                    rebuilt.push(whole.unwrap());
                }
            }
            // small enough to have travelled whole
            _ => rebuilt.push(frame),
        }
    }

    rebuilt
}

#[test]
fn a_message_survives_the_whole_client_path() {
    let mut network = TestNetwork::new();
    let mut outbound = network.outbound();
    let mut inbound = network.inbound();

    let original = vec![0xab; 10 * 1024];
    let now = Instant::now();

    // the client wraps it
    let packets = outbound
        .process(
            Some((
                original.clone(),
                LpOutboundOptions {
                    recipient: network.recipient,
                },
                network.gateway_address,
            )),
            now,
        )
        .unwrap();
    assert!(
        !packets.is_empty(),
        "a message this size produced no packets at all"
    );

    // its gateway decrypts and reassembles what it was asked to forward
    let frames = network.gateway_unwrap(packets);
    let frames = rebuild_frames(frames, now);
    assert!(
        frames.len() > 1,
        "the point of this test is a message that had to be split across packets"
    );

    // ... the mixes carry each packet to the recipient, who unwraps it
    let mut recovered = None;
    for frame in frames {
        assert_eq!(
            LpFrameKind::ForwardSphinxPacket,
            frame.kind(),
            "the gateway was asked to do something other than forward"
        );

        let attributes = ForwardSphinxFrameAttributes::try_from(frame.header.frame_attributes)
            .expect("the forwarding attributes did not parse back");

        let arriving = network.route_to_recipient(attributes.next_hop, &frame.content);

        let out = inbound.process_unwrapped(
            TimedPayload::new(now, arriving),
            ClientMessage::Sphinx(SphinxFrameAttributes {
                key_rotation: attributes.key_rotation,
            }),
        );

        if out.is_some() {
            assert!(
                recovered.is_none(),
                "more than one packet completed the message"
            );
            recovered = out;
        }
    }

    assert_eq!(Some(original), recovered);
}
