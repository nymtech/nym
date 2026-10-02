// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Bytes in one end of the pair, the same bytes out of the other.
//!
//! What sits between them is the part a provider cannot test: the gateway reading the frame it was
//! handed and forwarding the sphinx packet inside. That is done here by hand, with keys this test
//! generated, which makes these assertions about the *format* rather than about any node's
//! behaviour.
//!
//! Mix hops are disabled throughout, so a route is one gateway rather than four nodes. The layer
//! that matters is the same either way - the provider is the last sphinx hop, and that is the layer
//! the inbound half peels.

use std::sync::Arc;
use std::time::Instant;

use nym_client_core::client::lp::data::handler::pipeline::outbound::LpOutboundOptions;
use nym_client_core::client::topology_control::TopologyAccessor;
use nym_client_core::config::DebugConfig;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp_data::clients::traits::{ClientUnwrappingPipeline, ClientWrappingPipeline};
use nym_lp_data::common::traits::WireWrappingPipeline;
use nym_lp_data::packet::frame::{ForwardSphinxFrameAttributes, LpFrameKind};
use nym_lp_data::packet::{LpFrame, MAX_FRAME_PAYLOAD_SIZE};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::{ProcessedPacketData, SphinxPacket};
use nym_topology::{
    CachedEpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata, RoutingNode, SupportedRoles,
};
use rand::rngs::OsRng;
use time::OffsetDateTime;

use super::{SpInboundPipeline, SpOutboundPipeline};

/// The gateway the provider's traffic is forwarded through, and the key it peels with.
struct TestNetwork {
    gateway_id: NodeId,
    gateway_keys: x25519::KeyPair,

    /// Who the message is for, and the key its last sphinx layer is peeled with.
    recipient: Recipient,
    recipient_keys: Arc<x25519::KeyPair>,

    topology_accessor: TopologyAccessor,
}

impl TestNetwork {
    fn new() -> Self {
        let gateway_id: NodeId = 1;
        let gateway_identity = ed25519::KeyPair::new(&mut OsRng);
        let gateway_keys = x25519::KeyPair::new(&mut OsRng);

        let routing = RoutingNode {
            node_id: gateway_id,
            mix_host: "10.0.0.1:1789".parse().unwrap(),
            ip_addresses: Vec::new(),
            entry: None,
            identity_key: *gateway_identity.public_key(),
            sphinx_key: *gateway_keys.public_key(),
            supported_roles: SupportedRoles {
                mixnode: false,
                mixnet_entry: true,
                mixnet_exit: true,
            },
            lp: None,
            build_version: None,
        };

        let mut rewarded_set = CachedEpochRewardedSet::default();
        rewarded_set.entry_gateways.insert(gateway_id);
        rewarded_set.exit_gateways.insert(gateway_id);

        // `true`: the egress role is read off `supported_roles` rather than the rewarded set
        let topology_accessor = TopologyAccessor::new(true);
        topology_accessor.manually_change_topology(NymTopology::new(
            NymTopologyMetadata::new(0, 1, OffsetDateTime::now_utc()),
            rewarded_set,
            vec![routing.clone()],
        ));

        // the recipient sits behind that gateway, which is how a route to them is found
        let recipient_keys = Arc::new(x25519::KeyPair::new(&mut OsRng));
        let recipient_identity = ed25519::KeyPair::new(&mut OsRng);
        let recipient = Recipient::new(
            *recipient_identity.public_key(),
            *recipient_keys.public_key(),
            routing.identity_key,
        );

        TestNetwork {
            gateway_id,
            gateway_keys,
            recipient,
            recipient_keys,
            topology_accessor,
        }
    }

    fn debug_config() -> DebugConfig {
        let mut config = DebugConfig::default();
        // one gateway is a whole route, which is all this test needs to reach a final hop
        config.traffic.disable_mix_hops = true;
        config
    }

    fn outbound(&self) -> SpOutboundPipeline<OsRng> {
        SpOutboundPipeline::new(OsRng, Self::debug_config(), self.topology_accessor.clone())
    }

    fn inbound(&self) -> SpInboundPipeline {
        SpInboundPipeline::new(self.recipient_keys.clone())
    }

    /// What the gateway does with a frame its own provider handed it: read where the sphinx packet
    /// goes, peel its own layer, and pass on what is left.
    ///
    /// Returns the packet as it would reach the recipient - who is the final hop, and whose keys the
    /// inbound half peels with.
    fn forward(&self, frame: LpFrame) -> Vec<u8> {
        assert_eq!(
            LpFrameKind::ForwardSphinxPacket,
            frame.kind(),
            "the provider framed something the gateway would not forward"
        );

        let attributes = ForwardSphinxFrameAttributes::try_from(frame.header.frame_attributes)
            .expect("the frame did not carry forwarding attributes");
        assert_eq!(
            self.gateway_id, attributes.next_hop,
            "the frame named a first hop that is not the gateway"
        );

        let packet = SphinxPacket::from_bytes(&frame.content).unwrap();

        match packet
            .process(self.gateway_keys.private_key().as_ref())
            .unwrap()
            .data
        {
            ProcessedPacketData::ForwardHop {
                next_hop_packet,
                next_hop_address,
                ..
            } => {
                assert_eq!(
                    NymNodeRoutingAddress::try_from(next_hop_address).unwrap(),
                    NymNodeRoutingAddress::Client(*self.recipient.client_address()),
                    "the gateway's next hop was not the recipient"
                );
                next_hop_packet.to_bytes()
            }
            ProcessedPacketData::FinalHop { .. } => {
                panic!("the packet ended at the gateway, before reaching the recipient")
            }
        }
    }

    /// The whole path: wrap it, forward every frame, unwrap whatever completes.
    fn round_trip(&self, message: Vec<u8>) -> Option<Vec<u8>> {
        let mut outbound = self.outbound();
        let mut inbound = self.inbound();
        let now = Instant::now();

        let frames = outbound
            .process(
                Some((
                    message,
                    LpOutboundOptions {
                        recipient: self.recipient,
                    },
                    // the provider's frames go to the gateway in its own process, so nothing reads
                    // this; it is here because the pipeline traits name destinations by address
                    "127.0.0.1:0".parse().unwrap(),
                )),
                now,
            )
            .unwrap();

        assert!(!frames.is_empty(), "the message produced no frames at all");

        let mut delivered = None;
        for frame in frames {
            let sphinx = self.forward(frame.data.data);
            if let Some(message) = inbound.unwrap(sphinx, now).unwrap() {
                assert!(
                    delivered.replace(message).is_none(),
                    "one message completed twice"
                );
            }
        }

        delivered
    }
}

/// A message that fits in a single sphinx packet, there and back.
#[test]
fn a_small_message_survives_the_provider_path() {
    let network = TestNetwork::new();
    let original = b"a request, from a provider".to_vec();

    assert_eq!(Some(original.clone()), network.round_trip(original));
}

/// One that does not fit, so it is split into several packets and has to be put back together.
///
/// This is the assertion that message-level reassembly survived dropping the client's delivery
/// machinery: the pipeline holds the fragments itself and completes on the last one.
#[test]
fn a_fragmented_message_is_reassembled() {
    let network = TestNetwork::new();
    let original = vec![0xab; 10 * 1024];

    assert_eq!(Some(original.clone()), network.round_trip(original));
}

/// The provider chunks exactly as a client does.
///
/// Not a detail: chunk size decides where a message is split, and that is visible in the size of the
/// sphinx packets leaving the node. A provider that chunked differently from every real client
/// would be recognisable from a distance, however correct it otherwise was.
#[test]
fn the_provider_chunks_like_a_client() {
    let network = TestNetwork::new();

    // both pipelines charge the same routing-security overhead and span the same two frames, so
    // matching chunk sizes is exactly the question of whether the frame size matches
    let frame_size =
        WireWrappingPipeline::<LpFrame, LpOutboundOptions>::frame_size(&network.outbound());

    assert_eq!(MAX_FRAME_PAYLOAD_SIZE, frame_size);
}
