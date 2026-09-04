// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedData, TimedPayload,
    common::traits::{Framing, FramingUnwrap, Transport, TransportUnwrap},
    nymnodes::traits::NymNodeProcessingPipeline,
    packet::{EncryptedLpPacket, LpFrame, frame::LpFrameHeader},
};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use rand::Rng;
use tracing::warn;

use crate::node::{
    lp::data::{
        handler::{
            error::LpDataHandlerError,
            messages::{MixMessage, NymNodeMessage},
            pipeline::wire::{FramingPipeline, LpTransport},
            processing,
        },
        shared::{SharedGatewayLpDataState, SharedLpDataState},
    },
    lp::error::LpHandlerError,
    routing_filter::RoutingFilter,
};

pub struct NymNodeDataPipeline<R> {
    state: Arc<SharedLpDataState>,
    gateway_state: Arc<SharedGatewayLpDataState>,
    framing: FramingPipeline<R>,
}

impl<R: Rng> NymNodeDataPipeline<R> {
    pub fn new(
        state: Arc<SharedLpDataState>,
        gateway_state: Arc<SharedGatewayLpDataState>,
        rng: R,
    ) -> Self {
        Self {
            state: state.clone(),
            gateway_state,
            framing: FramingPipeline::new(state, rng),
        }
    }

    // Processing logic for packets supported by mixnode enabled node
    pub fn process_mix_packet(
        shared_state: &SharedLpDataState,
        message_kind: MixMessage,
        payload: TimedPayload,
    ) -> Result<PipelinePayload<MixMessage, NymNodeRoutingAddress>, LpDataHandlerError> {
        match message_kind {
            MixMessage::Sphinx(metadata) => {
                processing::sphinx::process(shared_state, payload, metadata)
            }
        }
    }
}

// Processing logic
impl<R: Rng> NymNodeProcessingPipeline<LpFrame, NymNodeRoutingAddress> for NymNodeDataPipeline<R> {
    type Options = NymNodeMessage;
    type MessageKind = NymNodeMessage;

    /// The LP MTU less everything the transport wrap will add on the way out.
    fn frame_size(&self) -> usize {
        nym_lp_data::packet::MTU - EncryptedLpPacket::OVERHEAD
    }

    fn mix(
        &mut self,
        message_kind: NymNodeMessage,
        payload: TimedPayload,
        _: Instant,
    ) -> Vec<PipelinePayload<NymNodeMessage, NymNodeRoutingAddress>> {
        // Everything specific to a given packet type should happen here
        let processing_result = match message_kind {
            NymNodeMessage::Mix(msg) => Self::process_mix_packet(&self.state, msg, payload)
                .map(|payload| payload.options_transform(Into::into)),
            NymNodeMessage::ForwardSphinx(metadata) => {
                processing::sphinx::process_forward(&self.gateway_state, payload, metadata)
            }
        };

        self.state.update_processing_metrics(&processing_result);

        let packet_to_forward = match processing_result {
            Ok(packet) => packet,
            Err(e) => {
                warn!("Error processing {message_kind:?} packet : {e}");
                return Vec::new();
            }
        };

        // Now we are deciding if we are routing the packet and where

        match packet_to_forward.dst {
            NymNodeRoutingAddress::Node(next_hop) => {
                if !self.state.routing_filter.should_route(next_hop.ip(), false) {
                    // SW need to pipe a socketaddr from the pipeline input
                    warn!(
                        event = "packet.dropped.routing_filter",
                        next_hop = %next_hop,
                        "dropping packet: egress address does not belong to any known node"
                    );
                    self.state.routing_filter_dropped(next_hop);
                    Vec::new()
                } else {
                    vec![packet_to_forward]
                }
            }
            NymNodeRoutingAddress::Client(client_address) => {
                if !self.state.processing_config.client_forwarding_enabled {
                    warn!(
                        event = "packet.dropped.client_forwarding_disabled",
                        "dropping packet destined to a client_address on a client_forwarding_disabled node"
                    );
                    self.state.client_forwarding_disabled_dropped();
                    Vec::new()
                } else if self
                    .gateway_state
                    .is_internal_service_provider(client_address)
                {
                    // Handed straight to the provider's channel: it lives in this process, so the
                    // payload never reaches the wire and needs neither framing nor encryption.
                    self.state.internal_sp_routed();
                    if !self
                        .gateway_state
                        .service_providers
                        .deliver(client_address, packet_to_forward.data.data)
                    {
                        warn!(
                            event = "packet.dropped.service_provider_unreachable",
                            client = %client_address,
                            "dropping packet: the service provider is no longer accepting messages"
                        );
                    }
                    Vec::new()
                } else {
                    // deliberately *not* resolved to an address here: the frame is held until its
                    // release time, and the transport layer needs the ID to encrypt then.
                    vec![packet_to_forward]
                }
            }
        }
    }
}

// ============== Framing: delegation to FramingPipeline ==============

impl<R: Rng> Framing<NymNodeMessage, NymNodeRoutingAddress> for NymNodeDataPipeline<R> {
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<NymNodeMessage, NymNodeRoutingAddress>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame, NymNodeRoutingAddress>> {
        let frame = LpFrame {
            header: payload.options.into(),
            content: payload.data.data.into(),
        };
        self.framing
            .message_to_frame(payload.data.timestamp, frame, payload.dst, frame_size)
    }
}

impl<R: Rng> Transport<EncryptedLpPacket, NymNodeRoutingAddress> for NymNodeDataPipeline<R> {
    type Frame = LpFrame;
    type Error = LpHandlerError;
    const OVERHEAD_SIZE: usize = EncryptedLpPacket::OVERHEAD;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame, NymNodeRoutingAddress>,
    ) -> Result<AddressedTimedData<EncryptedLpPacket>, Self::Error> {
        LpTransport::frame_to_packet(&self.state, frame)
    }
}

impl<R: Rng> TransportUnwrap<EncryptedLpPacket> for NymNodeDataPipeline<R> {
    type Frame = LpFrame;
    type Error = LpHandlerError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Self::Frame>, Self::Error> {
        LpTransport::packet_to_frame(&self.state, packet, timestamp)
    }
}

impl<R: Rng> FramingUnwrap<NymNodeMessage> for NymNodeDataPipeline<R> {
    type Frame = LpFrame;

    fn frame_to_message(
        &mut self,
        frame: TimedData<Self::Frame>,
    ) -> Option<(TimedPayload, NymNodeMessage)> {
        let reassembled = self.framing.frame_to_maybe_message(frame)?;
        let message_kind = reassembled
            .data
            .header
            .try_into()
            .inspect_err(|e| warn!("{e}"))
            .ok()?;

        self.state.message_received(message_kind);
        Some((
            TimedPayload::new(reassembled.timestamp, reassembled.data.content.to_vec()),
            message_kind,
        ))
    }
}

// ================================================================================================================================================

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use nym_lp_data::fragmentation::fragment::fragment_lp_message;
    use nym_lp_data::fragmentation::reconstruction::MessageReconstructor;
    use nym_lp_data::packet::{EncryptedLpPacket, LpFrame, OuterHeader, frame::LpFrameHeader};
    use nym_lp_data::{AddressedTimedData, TimedData};
    use nym_node_metrics::NymNodeMetrics;
    use nym_node_metrics::mixnet::PacketKind;
    use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
    use nym_sphinx_params::SphinxKeyRotation;
    use nym_sphinx_types::{
        DESTINATION_ADDRESS_LENGTH, Destination, DestinationAddressBytes, HEADER_SIZE,
        IDENTIFIER_LENGTH, Node, PrivateKey, PublicKey, SphinxPacketBuilder, header::delays::Delay,
    };
    use nym_task::ShutdownToken;
    use nym_test_utils::helpers::{DeterministicRng, deterministic_rng, seeded_rng};

    use crate::config::{LpConfig, ReplayProtectionDebug};
    use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
    use crate::node::key_rotation::key::SphinxPrivateKey;
    use crate::node::lp::active_sessions::{ActiveLpSessions, LpPeer};
    use crate::node::lp::data::handler::messages::{MixMessage, SphinxMixMessage};
    use crate::node::lp::data::handler::pipeline::NymNodeDataPipeline;
    use crate::node::lp::data::handler::pipeline::wire::LpTransport;
    use crate::node::lp::data::shared::{
        ProcessingConfig, SharedGatewayLpDataState, SharedLpDataState,
    };
    use crate::node::lp::error::LpHandlerError;
    use crate::node::replay_protection::bloomfilter::{
        ReplayProtectionBloomfilters, RotationFilter,
    };
    use crate::node::routing_filter::network_filter::NetworkRoutingFilter;
    use crate::node::shared_network::CachedFullTopology;
    use nym_lp_data::nymnodes::traits::NymNodeProcessingPipeline;

    // ==================== Test Helpers ====================

    /// Default rotation ids used by the mock state.
    const DEFAULT_ROTATION_ID: u32 = 0;

    /// Maximum forward packet delay used in tests. Matches the production default
    /// closely enough that delay-clamping behavior is exercised realistically.
    const TEST_MAX_FORWARD_DELAY: Duration = Duration::from_secs(10);

    /// Build a [`SharedLpDataState`] suitable for unit/integration tests of the
    /// mixnode data pipeline.
    ///
    /// - The sphinx primary key is generated from `rng` so the keypair is
    ///   reproducible across runs (given the same seed).
    /// - The replay-protection bloomfilter is enabled with a small capacity.
    /// - Metrics are fresh, no shutdown is signalled.
    fn mock_shared_state(rng: &mut DeterministicRng) -> SharedLpDataState {
        let primary = SphinxPrivateKey::new(rng, DEFAULT_ROTATION_ID);

        let primary_bloom_filter = RotationFilter::new(
            100,
            ReplayProtectionDebug::DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE,
            0,
            DEFAULT_ROTATION_ID,
        )
        .unwrap();

        SharedLpDataState {
            lp_config: LpConfig::default(),
            processing_config: ProcessingConfig {
                maximum_packet_delay: TEST_MAX_FORWARD_DELAY,
                client_forwarding_enabled: true,
            },
            sphinx_keys: ActiveSphinxKeys::new_loaded(primary, None),
            replay_protection_filter: ReplayProtectionBloomfilters::new(primary_bloom_filter, None),
            message_reconstructor: MessageReconstructor::default(),
            routing_filter: NetworkRoutingFilter::new_empty(true),
            sessions: ActiveLpSessions::new(),

            clients: Default::default(),
            metrics: NymNodeMetrics::default(),
            shutdown_token: ShutdownToken::new(),
        }
    }

    /// Build a [`MixnodeDataPipeline`] driven by a deterministic RNG.
    ///
    /// Returns the pipeline together with the shared state (so tests can
    /// inspect metrics or trigger replays directly)
    fn mock_pipeline() -> (
        NymNodeDataPipeline<DeterministicRng>,
        Arc<SharedLpDataState>,
    ) {
        let mut rng = deterministic_rng();
        let state = Arc::new(mock_shared_state(&mut rng));
        let gateway_state = Arc::new(SharedGatewayLpDataState::new(
            CachedFullTopology::new_empty(),
            Default::default(),
        ));
        let pipeline = NymNodeDataPipeline::new(state.clone(), gateway_state, rng);
        (pipeline, state)
    }

    /// Build a sphinx route node given a socket address and a private key
    fn mock_mix_node(socket: SocketAddr, key: PublicKey) -> Node {
        let addr_bytes = NymNodeRoutingAddress::from(socket).try_into().unwrap();
        Node::new(addr_bytes, key)
    }

    /// Build a sphinx packet whose first hop's key is the provided one.
    /// First hop forwards to `second_hop_address`, with a dummy key
    /// The first-hop delay is `first_hop_delay`; second hop's is zero.
    /// Unwrapping this packet will reveal a ForwardHop, with first_hop_delay and second_hop_address
    fn build_sphinx_bytes(
        first_hop_key: PublicKey,
        first_hop_delay: Delay,
        second_hop_address: SocketAddr,
        final_packet_size: usize,
        rng: &mut DeterministicRng,
    ) -> Vec<u8> {
        let payload_size = final_packet_size.checked_sub(HEADER_SIZE).unwrap();

        let first_hop_address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8000);
        let first_hop_node = mock_mix_node(first_hop_address, first_hop_key);

        let second_hop_key = PrivateKey::random_from_rng(rng);
        let second_hop_node = mock_mix_node(second_hop_address, (&second_hop_key).into());

        let route = [first_hop_node, second_hop_node];
        let delays = [first_hop_delay, Delay::new_from_millis(0)];

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
            [0u8; IDENTIFIER_LENGTH],
        );

        SphinxPacketBuilder::new()
            .with_payload_size(payload_size)
            .build_packet(b"Never gonna give you up", &route, &destination, &delays)
            .unwrap()
            .to_bytes()
    }

    /// Build a single-hop sphinx packet that the test mixnode will identify as
    /// a final-hop packet (no further forwarding).
    fn build_final_hop_sphinx_bytes(
        first_hop_key: PublicKey,
        first_hop_delay: Delay,
        final_packet_size: usize,
    ) -> Vec<u8> {
        let payload_size = final_packet_size.checked_sub(HEADER_SIZE).unwrap();

        let first_hop_address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8000);
        let first_hop_node = mock_mix_node(first_hop_address, first_hop_key);

        let route = [first_hop_node];
        let delays = [first_hop_delay];

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
            [0u8; IDENTIFIER_LENGTH],
        );

        SphinxPacketBuilder::new()
            .with_payload_size(payload_size)
            .build_packet(b"Never gonna let you down", &route, &destination, &delays)
            .unwrap()
            .to_bytes()
    }

    /// Wrap `bytes` into an [`LpFrame`] using `message` as the header, then
    /// either pass it through unfragmented (if it fits within `frame_size`)
    /// or split it into fragments. Mirrors the pipeline's framing logic so
    /// tests can build the exact packet sequence a peer would emit.
    /// Mirrors [`FramingPipeline::message_to_frame`]: `frame_payload_size` is what one frame can
    /// carry, its own header already accounted for.
    fn fragment_into_lp_frames(
        bytes: &[u8],
        message: MixMessage,
        frame_payload_size: usize,
        rng: &mut DeterministicRng,
    ) -> Vec<LpFrame> {
        let frame = LpFrame {
            header: message.into(),
            content: bytes.to_vec().into(),
        };

        if frame.content.len() > frame_payload_size {
            fragment_lp_message(rng, frame, frame_payload_size)
                .into_iter()
                .map(|f| f.into_lp_frame())
                .collect()
        } else {
            vec![frame]
        }
    }

    /// Default sphinx mix-message metadata used by tests (rotation matching the
    /// even primary key in [`mock_shared_state`]).
    fn sphinx_mix_message() -> MixMessage {
        MixMessage::Sphinx(SphinxMixMessage {
            key_rotation: SphinxKeyRotation::EvenRotation,
        })
    }

    // ==================== Tests ====================

    #[test]
    fn process_forwards_valid_sphinx_packet() {
        let (mut pipeline, state) = mock_pipeline();

        let mut rng = seeded_rng([52; 32]);

        let next_hop = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
        let delay = Delay::new_from_millis(50);

        // Packet fits exactly in one frame
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            delay,
            next_hop,
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(TimedData::new(arrival, input_packet), arrival);

        assert_eq!(outputs.len(), 1, "expected exactly one output frame");

        let output_packet = outputs[0].clone();

        assert_eq!(
            output_packet.dst,
            NymNodeRoutingAddress::Node(next_hop),
            "output frame must target the next hop"
        );
        assert_eq!(
            output_packet.data.timestamp,
            arrival + delay.to_duration(),
            "output frame delay must match arrival + delay"
        );
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );
        assert_eq!(state.metrics.mixnet.lp.malformed_packets(), 0);
    }

    #[test]
    fn process_drops_final_hop_packet() {
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = seeded_rng([52; 32]);

        let sphinx_bytes = build_final_hop_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            Delay::new_from_millis(50),
            pipeline.frame_size() - LpFrameHeader::SIZE,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let input_packet = inputs[0].clone();

        let outputs =
            pipeline.process(TimedData::new(Instant::now(), input_packet), Instant::now());

        assert!(
            outputs.is_empty(),
            "final-hop packets must not be forwarded"
        );
        assert_eq!(state.metrics.mixnet.lp.final_hop_packets_dropped(), 1);
        assert_eq!(state.metrics.mixnet.lp.messages_processed(), 0);
    }

    #[test]
    fn process_drops_replayed_packet() {
        let (mut pipeline, state) = mock_pipeline();

        let mut rng = seeded_rng([52; 32]);

        let next_hop = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
        let delay = Delay::new_from_millis(50);

        // Packet fits exactly in one frame
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            delay,
            next_hop,
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let input_packet = inputs[0].clone();
        // This also replays the LP encryption. This is fine for now since there is none, but once LP has replay protection by itself, we should test sphinx replay here
        let replayed_packet = inputs[0].clone();

        let arrival = Instant::now();
        let first = pipeline.process(TimedData::new(arrival, input_packet), arrival);
        assert_eq!(
            first.len(),
            1,
            "first send should be forwarded in one frame"
        );
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );

        let second = pipeline.process(TimedData::new(arrival, replayed_packet), arrival);
        assert!(second.is_empty(), "replay must not be forwarded");
        assert_eq!(state.metrics.mixnet.lp.replayed_packets(), 1);
        // Processing counter must not advance on the replayed packet.
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );
    }

    /// A frame wrapped on one side of a session decrypts to the same frame on the other, and the
    /// bytes on the wire are genuinely encrypted.
    ///
    /// This is the property the whole release-time-wrap arrangement exists to serve, so it asserts
    /// all of it: the outer header names the shared session, the counter advances per packet, the
    /// payload is not readable in the ciphertext, and the frame survives the round trip intact.
    #[test]
    fn transport_round_trips_a_frame_through_a_real_session() {
        use nym_lp::SessionsMock;
        use nym_lp_data::packet::frame::LpFrameKind;

        let mut rng = deterministic_rng();
        let sender = mock_shared_state(&mut rng);
        let receiver = mock_shared_state(&mut rng);

        // one session, two halves - both sides derive the same receiver index
        let pair = SessionsMock::mock_seeded_post_handshake(42, nym_lp::KEM::MlKem768);
        let session_index = pair.initiator.receiver_index();
        assert_eq!(session_index, pair.responder.receiver_index());

        let sender_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let receiver_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));
        sender
            .sessions
            .insert_addressed_session(LpPeer::node(receiver_ip), pair.initiator)
            .unwrap();
        receiver
            .sessions
            .insert_addressed_session(LpPeer::node(sender_ip), pair.responder)
            .unwrap();

        let secret = b"the payload an observer must not read".to_vec();
        let dst = NymNodeRoutingAddress::Node(SocketAddr::new(receiver_ip, 51264));
        let now = Instant::now();

        let mut counters = Vec::new();
        for _ in 0..3 {
            let frame = AddressedTimedData::new_addressed(
                now,
                LpFrame::new(LpFrameKind::Opaque, secret.clone()),
                dst,
            );

            let wrapped = LpTransport::frame_to_packet(&sender, frame).unwrap();
            let packet = wrapped.data.data;

            // the outer header is cleartext and names the session both sides share
            assert_eq!(packet.outer_header().receiver_idx, session_index);
            counters.push(packet.outer_header().counter);

            // ... and the payload is not readable in it
            assert!(
                !packet
                    .ciphertext()
                    .windows(secret.len())
                    .any(|w| w == secret.as_slice()),
                "the payload appears verbatim in the ciphertext - it is not encrypted"
            );

            let unwrapped = LpTransport::packet_to_frame(&receiver, packet, now).unwrap();
            assert_eq!(unwrapped.data.content.to_vec(), secret);
        }

        // one counter per packet, strictly increasing - what the release-time wrap guarantees
        assert!(
            counters.windows(2).all(|w| w[0] < w[1]),
            "counters must advance in send order, got {counters:?}"
        );
    }

    /// The client twin of the round trip above: a client is reached through the address it was
    /// last seen at, but its session is keyed by its [`ClientAddress`].
    ///
    /// The wrap only ever has a socket address to go on, so the registry is what closes the gap.
    /// A client with no entry there cannot be sent to at all - which is also what stops an
    /// unregistered address from borrowing somebody's session.
    #[test]
    fn transport_reaches_a_client_through_its_registered_address() {
        use nym_lp::SessionsMock;
        use nym_lp_data::packet::frame::LpFrameKind;
        use nym_sphinx_addressing::ClientAddress;

        let mut rng = deterministic_rng();
        let gateway = mock_shared_state(&mut rng);
        let client_state = mock_shared_state(&mut rng);

        let pair = SessionsMock::mock_seeded_post_handshake(7, nym_lp::KEM::MlKem768);
        let client = ClientAddress::from_bytes([9u8; 20]);
        let client_at = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)), 51264);

        gateway
            .sessions
            .insert_addressed_session(LpPeer::client(client), pair.responder)
            .unwrap();
        client_state
            .sessions
            .insert_new_session(pair.initiator)
            .unwrap();

        let to_client = NymNodeRoutingAddress::Client(client);
        let frame = || {
            AddressedTimedData::new_addressed(
                Instant::now(),
                LpFrame::new(LpFrameKind::Opaque, b"for the client".to_vec()),
                to_client,
            )
        };

        // registration bound the session, but nothing has said where that client is yet
        assert!(!gateway.has_session_for(to_client));
        assert!(matches!(
            LpTransport::frame_to_packet(&gateway, frame()),
            Err(LpHandlerError::NoSessionForPeer { .. })
        ));

        gateway.clients.refresh(client, client_at);
        assert!(gateway.has_session_for(to_client));

        let wrapped = LpTransport::frame_to_packet(&gateway, frame()).unwrap();
        assert_eq!(
            wrapped.dst, client_at,
            "the wire address is resolved from the client's identity, not carried with the frame"
        );

        let unwrapped =
            LpTransport::packet_to_frame(&client_state, wrapped.data.data, Instant::now()).unwrap();
        assert_eq!(unwrapped.data.content.to_vec(), b"for the client".to_vec());
    }

    /// A client that moves is still reached, because the wrap resolves its address at release time
    /// rather than when the frame was routed.
    ///
    /// The frame is addressed by [`ClientAddress`] throughout, so a refresh landing between the two
    /// wraps changes where the second one goes without the frame knowing anything about it.
    #[test]
    fn the_wrap_follows_a_client_that_moved() {
        use nym_lp::SessionsMock;
        use nym_lp_data::packet::frame::LpFrameKind;
        use nym_sphinx_addressing::ClientAddress;

        let mut rng = deterministic_rng();
        let gateway = mock_shared_state(&mut rng);

        let pair = SessionsMock::mock_seeded_post_handshake(11, nym_lp::KEM::MlKem768);
        let client = ClientAddress::from_bytes([4u8; 20]);
        gateway
            .sessions
            .insert_addressed_session(LpPeer::client(client), pair.responder)
            .unwrap();

        let first = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 4)), 51264);
        let second = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 5)), 41000);
        let frame = || {
            AddressedTimedData::new_addressed(
                Instant::now(),
                LpFrame::new(LpFrameKind::Opaque, b"follow me".to_vec()),
                NymNodeRoutingAddress::Client(client),
            )
        };

        gateway.clients.refresh(client, first);
        let before = LpTransport::frame_to_packet(&gateway, frame()).unwrap();
        assert_eq!(before.dst, first);

        // an inbound packet from a new address would do this in production
        gateway.clients.refresh(client, second);
        let after = LpTransport::frame_to_packet(&gateway, frame()).unwrap();
        assert_eq!(after.dst, second, "the frame must follow the client");
    }

    /// A packet naming no session is rejected by the *transport* layer, before anything reaches
    /// the pipeline - hence this exercises `packet_to_frame` rather than `process`.
    ///
    /// This is the peer-restart signature: the sender still holds a session this node has lost, so
    /// its packets carry a receiver index that resolves to nothing. It is also what any garbage
    /// arriving on the UDP port looks like, since the index is checked before the ciphertext is
    /// touched.
    #[test]
    fn transport_rejects_packet_for_unknown_session() {
        let (_pipeline, state) = mock_pipeline();

        let orphan = EncryptedLpPacket::new(OuterHeader::new(0xDEAD, 0), Vec::new());
        let result = LpTransport::packet_to_frame(&state, orphan, Instant::now());

        assert!(matches!(
            result,
            Err(LpHandlerError::MissingLpSession {
                receiver_index: 0xDEAD
            })
        ));
        assert_eq!(state.metrics.mixnet.lp.messages_processed(), 0);
    }

    #[test]
    fn process_drops_garbage_sphinx_payload() {
        // A well-formed LP packet whose sphinx payload is garbage exercises the
        // *processing* malformed path (not the LP-decode one).
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = deterministic_rng();

        let garbage = vec![0xAAu8; pipeline.frame_size() - LpFrameHeader::SIZE];
        let inputs = fragment_into_lp_frames(
            &garbage,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let now = Instant::now();
        let outputs =
            pipeline.process(TimedData::new(now, inputs.into_iter().next().unwrap()), now);
        assert!(
            outputs.is_empty(),
            "garbage sphinx payload must yield no output"
        );
        // Sphinx-level decode failures surface as a misc processing error,
        // distinct from LP-decode malformed packets.
        assert_eq!(state.metrics.mixnet.lp.processing_misc_errors(), 1);
        assert_eq!(state.metrics.mixnet.lp.malformed_packets(), 0);
    }

    #[test]
    fn fragmented_message_reconstructs_across_frames() {
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = deterministic_rng();

        let next_hop = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
        let delay = Delay::new_from_millis(50);

        let nb_fragments = 3;
        let frame_payload_size = pipeline.frame_size() - LpFrameHeader::SIZE;

        // Sized so the wrapping LpFrame serialises to exactly `nb_fragments` payloads' worth -
        // its own header included, since fragmentation splits the serialised frame.
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            delay,
            next_hop,
            nb_fragments * frame_payload_size - LpFrameHeader::SIZE,
            &mut rng,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            frame_payload_size,
            &mut rng,
        );
        assert_eq!(
            inputs.len(),
            nb_fragments,
            "test setup should produce {nb_fragments} fragments",
        );

        let now = Instant::now();

        // Simulate different arrival times
        let arrivals = (0..nb_fragments as u32)
            .map(|i| now + (Duration::from_millis(40) * i))
            .collect::<Vec<_>>();

        // Send all fragments but one
        for i in 0..nb_fragments - 1 {
            let out = pipeline.process(TimedData::new(arrivals[i], inputs[i].clone()), arrivals[i]);
            assert!(
                out.is_empty(),
                "fragment #{i} should not have produced output"
            );
        }

        // Last fragments should reconstruct and forward
        let last = nb_fragments - 1;
        let out = pipeline.process(
            TimedData::new(arrivals[last], inputs[last].clone()),
            arrivals[last],
        );

        assert_eq!(
            out.len(),
            nb_fragments,
            "last fragment should reconstruct the message and produce {nb_fragments} fragments"
        );

        for out_pkt in out {
            assert_eq!(
                out_pkt.dst,
                NymNodeRoutingAddress::Node(next_hop),
                "output frame must target the next hop"
            );

            // All fragments should have a ts of the last arrival plus delay
            assert_eq!(
                out_pkt.data.timestamp,
                arrivals[nb_fragments - 1] + delay.to_duration(),
                "output frame delay must match arrival + delay"
            );
        }

        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );
    }

    #[test]
    fn excessive_delay_is_clamped() {
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = seeded_rng([52; 32]);

        let next_hop = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
        // 30s well exceeds TEST_MAX_FORWARD_DELAY (10s); the pipeline must clamp.
        let huge_delay = Delay::new_from_millis(30_000);

        // Packet fits exactly in one frame
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            huge_delay,
            next_hop,
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(TimedData::new(arrival, input_packet), arrival);

        assert_eq!(outputs.len(), 1, "expected exactly one output frame");

        let output_packet = outputs[0].clone();

        assert_eq!(
            output_packet.dst,
            NymNodeRoutingAddress::Node(next_hop),
            "output frame must target the next hop"
        );
        assert_eq!(
            output_packet.data.timestamp,
            arrival + TEST_MAX_FORWARD_DELAY,
            "delay must be clamped to TEST_MAX_FORWARD_DELAY"
        );

        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );

        assert_eq!(state.metrics.mixnet.lp.excessive_delay_packets(), 1);
    }

    #[test]
    fn process_out_of_network_sphinx_packet() {
        let (mut pipeline, state) = mock_pipeline();

        let mut rng = seeded_rng([52; 32]);

        // Routing filters is in local mode so public address will fail
        let next_hop = "1.1.1.1:1234".parse().unwrap();
        let delay = Delay::new_from_millis(50);

        // Packet fits exactly in one frame
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            delay,
            next_hop,
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );

        let inputs = fragment_into_lp_frames(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size() - LpFrameHeader::SIZE,
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input frame");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(TimedData::new(arrival, input_packet), arrival);

        assert!(outputs.is_empty(), "expected no output");

        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );
        assert_eq!(state.metrics.mixnet.lp.routing_filter_dropped(), 1);
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .routing_filter_dropped_per_dst()
                .get(&next_hop)
                .map(|v| *v),
            Some(1)
        );
    }
}
