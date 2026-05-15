// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{net::SocketAddr, sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    fragmentation::fragment::fragment_payload,
    mixnodes::traits::MixnodeProcessingPipeline,
    packet::{
        EncryptedLpPacket, LpFrame, LpHeader, LpPacket, MalformedLpPacketError,
        frame::LpFrameHeader,
    },
};
use rand::Rng;
use tracing::warn;

use crate::node::{
    lp::data::{
        handler::{messages::MixMessage, processing},
        shared::SharedLpDataState,
    },
    routing_filter::RoutingFilter,
};

pub struct MixnodeDataPipeline<R>
where
    R: Rng,
{
    /// Shared data state
    state: Arc<SharedLpDataState>,
    rng: R,
}

impl<R> MixnodeDataPipeline<R>
where
    R: Rng,
{
    pub fn new(state: Arc<SharedLpDataState>, rng: R) -> Self {
        Self { state, rng }
    }
}

// Mixing logic
impl<R> MixnodeProcessingPipeline<Instant, EncryptedLpPacket, MixMessage, MixMessage, SocketAddr>
    for MixnodeDataPipeline<R>
where
    R: Rng,
{
    fn mix(
        &mut self,
        message_kind: MixMessage,
        payload: TimedPayload<Instant>,
        _: Instant,
    ) -> Vec<PipelinePayload<Instant, MixMessage, SocketAddr>> {
        let processing_result = match message_kind {
            MixMessage::Sphinx {
                key_rotation,
                reserved: _,
            } => processing::sphinx::process(&self.state, payload, key_rotation),
            MixMessage::Outfox {
                key_rotation,
                reserved: _,
            } => processing::outfox::process(&self.state, payload, key_rotation),
        };

        self.state
            .update_processing_metrics(&processing_result, message_kind);

        let packet_to_forward = match processing_result {
            Ok(packet) => packet,
            Err(e) => {
                warn!("Error processing {message_kind:?} packet : {e}");
                return Vec::new();
            }
        };

        let next_hop = packet_to_forward.dst;
        if !self.state.routing_filter.should_route(next_hop.ip()) {
            warn!(
                event = "packet.dropped.routing_filter",
                next_hop = %next_hop,
                "dropping packet: egress address does not belong to any known node"
            );
            self.state.routing_filter_dropped(next_hop);
            Vec::new()
        } else {
            vec![packet_to_forward.with_options(message_kind)]
        }
    }
}

impl<R> Framing<Instant, MixMessage, SocketAddr> for MixnodeDataPipeline<R>
where
    R: Rng,
{
    type Frame = LpFrame;

    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<Instant, MixMessage, SocketAddr>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Instant, Self::Frame, SocketAddr>> {
        let content = payload.data.data;
        let fragments =
            fragment_payload(&mut self.rng, &content, payload.options.into(), frame_size);

        fragments
            .into_iter()
            .map(|f| {
                AddressedTimedData::new_addressed(
                    payload.data.timestamp,
                    f.into_lp_frame(),
                    payload.dst,
                )
            })
            .collect()
    }
}

impl<R> Transport<Instant, EncryptedLpPacket, SocketAddr> for MixnodeDataPipeline<R>
where
    R: Rng,
{
    type Frame = LpFrame;

    const OVERHEAD_SIZE: usize = LpHeader::SIZE;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Instant, Self::Frame, SocketAddr>,
    ) -> AddressedTimedData<Instant, EncryptedLpPacket, SocketAddr> {
        // Here be LP encryption. For not, just wrap into an EncryptedLpPacket, we don't care at reception anyway
        frame.data_transform(|f| LpPacket::new(LpHeader::new(0, 0, 0), f).encode())
    }
}

impl<R> WireWrappingPipeline<Instant, EncryptedLpPacket, MixMessage, SocketAddr>
    for MixnodeDataPipeline<R>
where
    R: Rng,
{
    fn packet_size(&self) -> usize {
        nym_lp_data::packet::MTU
    }
}

impl<R> TransportUnwrap<Instant, EncryptedLpPacket> for MixnodeDataPipeline<R>
where
    R: Rng,
{
    type Frame = LpFrame;
    type Error = MalformedLpPacketError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Instant, Self::Frame>, Self::Error> {
        // Here be LP decryption. For now we do as is it's not encrypted
        let lp_packet = LpPacket::decode(packet).inspect_err(|_| {
            self.state.malformed_packet();
        })?;
        Ok(TimedData {
            timestamp,
            data: lp_packet.into_frame(),
        })
    }
}

impl<R> FramingUnwrap<Instant, MixMessage> for MixnodeDataPipeline<R>
where
    R: Rng,
{
    type Frame = LpFrame;
    fn frame_to_message(
        &mut self,
        frame: TimedData<Instant, Self::Frame>,
    ) -> Option<(TimedPayload<Instant>, MixMessage)> {
        if frame.data.kind().is_fragmented() {
            let fragment = frame
                .data
                .try_into()
                .inspect_err(|e| {
                    tracing::error!("Failed to recover a fragment : {e}");
                    self.state.malformed_packet();
                })
                .ok()?;
            let (payload, metadata) = self
                .state
                .message_reconstructor
                .insert_new_fragment(fragment, frame.timestamp)?;
            let message_kind = metadata
                .try_into()
                .inspect_err(|e| {
                    tracing::warn!(
                        "Somehow got a non fragmented message kind from reconstruction buffer : {e}"
                    );
                })
                .ok()?;
            self.state.message_received(message_kind);
            Some((TimedPayload::new(frame.timestamp, payload), message_kind))
        } else {
            warn!("unimplemented yet");
            None
        }
    }
}

impl<R> WireUnwrappingPipeline<Instant, EncryptedLpPacket, MixMessage> for MixnodeDataPipeline<R> where
    R: Rng
{
}

// ================================================================================================================================================

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use nym_lp_data::common::traits::WireWrappingPipeline;
    use nym_lp_data::fragmentation::fragment::{FragmentMetadata, fragment_payload};
    use nym_lp_data::fragmentation::reconstruction::MessageReconstructor;
    use nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline;
    use nym_lp_data::packet::{
        EncryptedLpPacket, LpFrame, LpHeader, LpPacket, OuterHeader, version,
    };
    use nym_node_metrics::NymNodeMetrics;
    use nym_node_metrics::mixnet::PacketKind;
    use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
    use nym_sphinx_params::SphinxKeyRotation;
    use nym_sphinx_types::{
        DESTINATION_ADDRESS_LENGTH, Destination, DestinationAddressBytes, HEADER_SIZE,
        IDENTIFIER_LENGTH, Node, OUTFOX_PACKET_OVERHEAD, OutfoxPacket, PrivateKey, PublicKey,
        SphinxPacketBuilder, header::delays::Delay,
    };
    use nym_task::ShutdownToken;
    use nym_test_utils::helpers::{DeterministicRng, deterministic_rng, seeded_rng};

    use crate::config::{LpConfig, ReplayProtectionDebug};
    use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
    use crate::node::key_rotation::key::SphinxPrivateKey;
    use crate::node::lp::data::handler::messages::MixMessage;
    use crate::node::lp::data::handler::pipeline::MixnodeDataPipeline;
    use crate::node::lp::data::shared::{ProcessingConfig, SharedLpDataState};
    use crate::node::replay_protection::bloomfilter::{
        ReplayProtectionBloomfilters, RotationFilter,
    };
    use crate::node::routing_filter::network_filter::NetworkRoutingFilter;

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
            },
            sphinx_keys: ActiveSphinxKeys::new_loaded(primary, None),
            replay_protection_filter: ReplayProtectionBloomfilters::new(primary_bloom_filter, None),
            message_reconstructor: MessageReconstructor::default(),
            routing_filter: NetworkRoutingFilter::new_empty(true),
            metrics: NymNodeMetrics::default(),
            shutdown_token: ShutdownToken::new(),
        }
    }

    /// Build a [`MixnodeDataPipeline`] driven by a deterministic RNG.
    ///
    /// Returns the pipeline together with the shared state (so tests can
    /// inspect metrics or trigger replays directly)
    fn mock_pipeline() -> (
        MixnodeDataPipeline<DeterministicRng>,
        Arc<SharedLpDataState>,
    ) {
        let mut rng = deterministic_rng();
        let state = Arc::new(mock_shared_state(&mut rng));
        let pipeline = MixnodeDataPipeline::new(state.clone(), rng);
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

    /// Build an outfox packet whose first hop's key is the provided one.
    /// First hop forwards to `second_hop_address`, with a dummy key
    /// The first-hop delay is `first_hop_delay`; second hop's is zero.
    /// The rest of the route is dummy an irrelevant
    /// Unwrapping this packet will reveal a ForwardHop, with first_hop_delay and second_hop_address
    fn build_outfox_bytes(
        first_hop_key: PublicKey,
        second_hop_address: SocketAddr,
        final_packet_size: usize,
        rng: &mut DeterministicRng,
    ) -> Vec<u8> {
        let payload_size = final_packet_size
            .checked_sub(OUTFOX_PACKET_OVERHEAD)
            .unwrap();

        let first_hop_address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8000);
        let first_hop_node = mock_mix_node(first_hop_address, first_hop_key);

        let second_hop_key = PrivateKey::random_from_rng(&mut *rng);
        let second_hop_node = mock_mix_node(second_hop_address, (&second_hop_key).into());

        let node3_address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8001);
        let node3_key = PrivateKey::random_from_rng(&mut *rng);
        let node3 = mock_mix_node(node3_address, (&node3_key).into());

        let node4_address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8002);
        let node4_key = PrivateKey::random_from_rng(&mut *rng);
        let node4 = mock_mix_node(node4_address, (&node4_key).into());

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
            [0u8; IDENTIFIER_LENGTH],
        );

        let route = [first_hop_node, second_hop_node, node3, node4];
        let payload = b"Never gonna turn around".to_vec();

        OutfoxPacket::build(&payload, &route, &destination, Some(payload_size))
            .unwrap()
            .to_bytes()
            .unwrap()
    }

    /// Wrap an [`LpFrame`] into an [`EncryptedLpPacket`] that the pipeline can
    /// decode.
    fn lp_frame_to_encrypted_packet(frame: LpFrame) -> EncryptedLpPacket {
        LpPacket::new(LpHeader::new(0, 0, version::CURRENT), frame).encode()
    }

    /// Fragment `bytes` into `EncryptedLpPacket`s carrying the given mix-message
    /// metadata. `fragment_payload_size` controls how the payload is split:
    /// pass at least `bytes.len()` to get a single fragment, or smaller to force
    /// multiple fragments.
    fn fragment_into_lp_packets(
        bytes: &[u8],
        message: MixMessage,
        fragment_payload_size: usize,
        rng: &mut DeterministicRng,
    ) -> Vec<EncryptedLpPacket> {
        let metadata: FragmentMetadata = message.into();
        fragment_payload(rng, bytes, metadata, fragment_payload_size.max(1))
            .into_iter()
            .map(|f| lp_frame_to_encrypted_packet(f.into_lp_frame()))
            .collect()
    }

    /// Default sphinx mix-message metadata used by tests (rotation matching the
    /// even primary key in [`mock_shared_state`]).
    fn sphinx_mix_message() -> MixMessage {
        MixMessage::Sphinx {
            key_rotation: SphinxKeyRotation::EvenRotation,
            reserved: [0u8; 3],
        }
    }

    fn outfox_mix_message() -> MixMessage {
        MixMessage::Outfox {
            key_rotation: SphinxKeyRotation::EvenRotation,
            reserved: [0u8; 3],
        }
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
            pipeline.frame_size(),
            &mut rng,
        );

        // Sanity check
        assert_eq!(sphinx_bytes.len(), pipeline.frame_size());

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(input_packet, arrival).unwrap();

        assert_eq!(outputs.len(), 1, "expected exactly one output fragment");

        let output_packet = outputs[0].clone();

        assert_eq!(
            output_packet.dst, next_hop,
            "output fragment must target the next hop"
        );
        assert_eq!(
            output_packet.data.timestamp,
            arrival + delay.to_duration(),
            "output fragment delay must match arrival + delay"
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
    fn process_forwards_valid_outfox_packet() {
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = seeded_rng([52; 32]);

        let next_hop = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);

        // Packet fits exactly in a frame
        let outfox_bytes = build_outfox_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            next_hop,
            pipeline.frame_size(),
            &mut rng,
        );

        // Sanity check
        assert_eq!(outfox_bytes.len(), pipeline.frame_size());

        let inputs = fragment_into_lp_packets(
            &outfox_bytes,
            outfox_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(input_packet, arrival).unwrap();

        assert_eq!(outputs.len(), 1, "expected exactly one output fragment");

        let output_packet = outputs[0].clone();

        assert_eq!(
            output_packet.dst, next_hop,
            "output fragment must target the next hop"
        );
        assert_eq!(
            output_packet.data.timestamp, arrival,
            "outfox output fragment should not have any delay"
        );
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpOutfox),
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
            pipeline.frame_size(),
        );

        // Sanity check
        assert_eq!(sphinx_bytes.len(), pipeline.frame_size());

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();

        let outputs = pipeline.process(input_packet, Instant::now()).unwrap();

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
            pipeline.frame_size(),
            &mut rng,
        );

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();
        // This also replays the LP encryption. This is fine for now since there is none, but once LP has replay protection by itself, we should test sphinx replay here
        let replayed_packet = inputs[0].clone();

        let arrival = Instant::now();
        let first = pipeline.process(input_packet, arrival).unwrap();
        assert_eq!(
            first.len(),
            1,
            "first send should be forwarded in one fragment"
        );
        assert_eq!(
            state
                .metrics
                .mixnet
                .lp
                .messages_processed_for(PacketKind::LpSphinx),
            1
        );

        let second = pipeline.process(replayed_packet, arrival).unwrap();
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

    #[test]
    fn process_drops_malformed_lp_packet() {
        let (mut pipeline, state) = mock_pipeline();

        // Empty ciphertext: InnerHeader::parse fails with InsufficientData,
        // which the pipeline reports as a malformed packet.
        let bad = EncryptedLpPacket::new(OuterHeader::new(0, 0), Vec::new());
        let result = pipeline.process(bad, Instant::now());
        assert!(result.is_err(), "malformed LP packet must surface an error");
        assert_eq!(state.metrics.mixnet.lp.malformed_packets(), 1);
        assert_eq!(state.metrics.mixnet.lp.messages_processed(), 0);
    }

    #[test]
    fn process_drops_garbage_sphinx_payload() {
        // A well-formed LP packet whose sphinx payload is garbage exercises the
        // *processing* malformed path (not the LP-decode one).
        let (mut pipeline, state) = mock_pipeline();
        let mut rng = deterministic_rng();

        let garbage = vec![0xAAu8; pipeline.frame_size()];
        let inputs =
            fragment_into_lp_packets(&garbage, sphinx_mix_message(), garbage.len(), &mut rng);

        let outputs = pipeline
            .process(inputs.into_iter().next().unwrap(), Instant::now())
            .unwrap();
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

        // Packet fits exactly in one frame
        let sphinx_bytes = build_sphinx_bytes(
            state.sphinx_keys.primary().x25519_pubkey().into(),
            delay,
            next_hop,
            nb_fragments * pipeline.frame_size(),
            &mut rng,
        );

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
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
            let out = pipeline.process(inputs[i].clone(), arrivals[i]).unwrap();
            assert!(
                out.is_empty(),
                "fragment #{i} should not have produced output"
            );
        }

        // Last fragments should reconstruct and forward
        let out = pipeline
            .process(inputs[nb_fragments - 1].clone(), arrivals[nb_fragments - 1])
            .unwrap();

        assert_eq!(
            out.len(),
            nb_fragments,
            "last fragment should reconstruct the message and produce {nb_fragments} fragments"
        );

        for out_pkt in out {
            assert_eq!(
                out_pkt.dst, next_hop,
                "output fragment must target the next hop"
            );

            // All fragments should have a ts of the last arrival plus delay
            assert_eq!(
                out_pkt.data.timestamp,
                arrivals[nb_fragments - 1] + delay.to_duration(),
                "output fragment delay must match arrival + delay"
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
            pipeline.frame_size(),
            &mut rng,
        );

        // Sanity check
        assert_eq!(sphinx_bytes.len(), pipeline.frame_size());

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(input_packet, arrival).unwrap();

        assert_eq!(outputs.len(), 1, "expected exactly one output fragment");

        let output_packet = outputs[0].clone();

        assert_eq!(
            output_packet.dst, next_hop,
            "output fragment must target the next hop"
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
            pipeline.frame_size(),
            &mut rng,
        );

        // Sanity check
        assert_eq!(sphinx_bytes.len(), pipeline.frame_size());

        let inputs = fragment_into_lp_packets(
            &sphinx_bytes,
            sphinx_mix_message(),
            pipeline.frame_size(),
            &mut rng,
        );
        assert_eq!(inputs.len(), 1, "expected a single input fragment");

        let input_packet = inputs[0].clone();

        let arrival = Instant::now();
        let outputs = pipeline.process(input_packet, arrival).unwrap();

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
