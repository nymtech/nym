// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimNymClient`] — simulated client that produces sphinx-in-LP packets
//! consumed by the [`SimNymNode`](crate::node::nymnode::SimNymNode).
//!
//! The wrapping pipeline applies sphinx-style chunking, full Sphinx encryption
//! over a 3-hop route, and LP framing/transport. Reliability and obfuscation
//! are no-ops to keep the wire trace easy to follow.

use std::{convert::Infallible, sync::Arc, time::Instant};

use nym_crypto::asymmetric::x25519;
use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline, RoutingSecurity},
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    fragmentation::{fragment::fragment_lp_message, reconstruction::MessageReconstructor},
    packet::{
        EncryptedLpPacket, LpFrame, LpHeader, LpPacket, MalformedLpPacketError,
        frame::{LpFrameHeader, LpFrameKind},
        version,
    },
};
use nym_node::node::lp::data::handler::messages::ForwardSphinxMessage;
use nym_sphinx::{
    Delay, ProcessedPacketData, SphinxPacket, SphinxPacketBuilder,
    chunking::{
        fragment::Fragment, reconstruction::MessageReconstructor as SphinxMessageReconstructor,
    },
    message::{NymMessage, PaddedMessage},
};
use nym_sphinx_params::SphinxKeyRotation;
use rand::Rng;

use crate::{
    client::{BaseClient, ClientId, ProcessingClient},
    helpers,
    topology::{
        TopologyClient,
        directory::{Directory, DirectoryClient},
    },
};

// SW To be replaced with actual client implementation

/// A simulated client that produces sphinx-in-LP packets.
///
/// `Ts` is fixed to [`Instant`] because the real [`NymNodeDataPipeline`] only
/// works on wall-clock time.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
///
/// [`NymNodeDataPipeline`]: nym_node::node::lp::data::handler::pipeline::NymNodeDataPipeline
pub type SimNymClient<R> = BaseClient<SimNymProcesssingClient<R>, EncryptedLpPacket>;

impl<R: Rng + Send> SimNymClient<R> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(
        topology_client: TopologyClient,
        directory: Arc<Directory>,
        rng: R,
    ) -> anyhow::Result<Self> {
        let processing_client = SimNymProcesssingClient {
            wrapper: SimNymClientWrappingPipeline {
                directory: directory.clone(),
                rng,
            },
            unwrapper: NymNodeUnwrappingPipeline {
                message_reconstructor: Default::default(),
                sphinx_message_reconstructor: SphinxMessageReconstructor::default(),
                sphinx_secret_key: topology_client.sphinx_private_key,
            },
        };
        BaseClient::with_pipeline(
            topology_client.client_id,
            topology_client.mixnet_address,
            topology_client.app_address,
            processing_client,
        )
    }
}

///
/// `dst` is the final destination [`ClientId`] embedded in the sphinx packet's
/// destination address. `first_hop` is the [`std::net::SocketAddr`] of the
/// first mix node; the client sends the LP packet there.
#[derive(Clone, Copy)]
pub struct SimNymClientInputOptions {
    pub dst: DirectoryClient,
}

pub struct SimNymProcesssingClient<R: Rng> {
    wrapper: SimNymClientWrappingPipeline<R>,
    unwrapper: NymNodeUnwrappingPipeline,
}

impl<R: Rng + Send> ProcessingClient<EncryptedLpPacket> for SimNymProcesssingClient<R> {
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<EncryptedLpPacket>> {
        if input.is_empty() {
            return Vec::new();
        }
        let Some(&destination_client) = self.wrapper.directory.client(dst) else {
            tracing::error!("Destination {dst} does not exist in the topology");
            return Vec::new();
        };

        let first_hop = self
            .wrapper
            .directory
            .random_next_hop(&mut self.wrapper.rng);

        let input_options = SimNymClientInputOptions {
            dst: destination_client,
        };

        // SAFETY: this pipeline's transport is `Infallible`
        #[allow(clippy::unwrap_used)]
        self.wrapper
            .process(Some((input, input_options, first_hop.addr)), timestamp)
            .unwrap()
    }

    fn unwrap(
        &mut self,
        input: EncryptedLpPacket,
        timestamp: Instant,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.unwrapper.unwrap(input, timestamp)?)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wrapping pipeline

/// Full wrapping pipeline for [`SimNymClient`].
///
/// Applies, in order: sphinx-style chunking, Sphinx onion encryption over a
/// random 3-hop route, LP framing (with fragmentation when the encrypted packet
/// exceeds the frame size), and LP transport.
pub struct SimNymClientWrappingPipeline<R: Rng> {
    /// Shared routing table; used to sample the 3-hop route in `encrypt`.
    directory: Arc<Directory>,
    /// RNG used for route selection, sphinx delays, and LP fragmentation.
    rng: R,
}

impl<R: Rng> Chunking<SimNymClientInputOptions> for SimNymClientWrappingPipeline<R> {
    /// Split `input` into sphinx-sized chunks using the standard sphinx
    /// fragmentation. Every chunk is addressed to the configured first hop so
    /// the LP packet reaches the network entry node.
    fn chunked(
        &mut self,
        input: PipelinePayload<SimNymClientInputOptions>,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<SimNymClientInputOptions>> {
        let fragments = NymMessage::new_plain(input.data.data)
            .pad_to_full_packet_lengths(chunk_size)
            .split_into_fragments(&mut self.rng, chunk_size);

        fragments
            .into_iter()
            .map(|fragment| {
                PipelinePayload::new(timestamp, fragment.into_bytes(), input.options, input.dst)
            })
            .collect()
    }
}

impl<R: Rng> NoOpReliability for SimNymClientWrappingPipeline<R> {}
impl<R: Rng> NoOpObfuscation for SimNymClientWrappingPipeline<R> {}

impl<R: Rng> RoutingSecurity<SimNymClientInputOptions> for SimNymClientWrappingPipeline<R> {
    // We are wrapping the sphinx packet in an LpFrame, hence the extra header overhead
    const OVERHEAD_SIZE: usize =
        nym_sphinx::HEADER_SIZE + nym_sphinx::PAYLOAD_OVERHEAD_SIZE + LpFrameHeader::SIZE;

    fn nb_frames(&self) -> usize {
        2
    }

    /// Wrap `input` in a Sphinx onion packet with a 3-hop route.
    ///
    /// The route is built by taking `options.first_hop` as the first hop and
    /// choosing two additional hops at random. The final destination address
    /// is derived from `options.dst`. Per-hop delays come from
    /// [`crate::helpers::generate_mix_delay`].
    fn encrypt(
        &mut self,
        input: PipelinePayload<SimNymClientInputOptions>,
    ) -> PipelinePayload<SimNymClientInputOptions> {
        // We need to forbid the first hop as the first node in the route
        let route = self
            .directory
            .random_route(3, &mut self.rng, Some(input.dst));

        let first_mix_hop = route[0].id;

        let sphinx_route = route
            .into_iter()
            .map(|n| n.as_sphinx_node_socket())
            .chain(std::iter::once(input.options.dst.as_sphinx_node()))
            .collect::<Vec<_>>();

        let delays = (0..sphinx_route.len())
            .map(|_| Delay::new_from_millis(helpers::generate_mix_delay(&mut self.rng)))
            .collect::<Vec<_>>();

        let plaintext_size = (<Self as WireWrappingPipeline<
            EncryptedLpPacket,
            SimNymClientInputOptions,
        >>::packet_size(self)
            - <Self as Framing<SimNymClientInputOptions>>::OVERHEAD_SIZE
            - <Self as Transport<EncryptedLpPacket>>::OVERHEAD_SIZE)
            * self.nb_frames()
            - <Self as RoutingSecurity<_>>::OVERHEAD_SIZE;

        let packet_builder = SphinxPacketBuilder::new()
            .with_payload_size(plaintext_size + nym_sphinx::PAYLOAD_OVERHEAD_SIZE);

        // SAFETY : If the pipeline is built correctly, the packet building should not fail.
        // If it does, something is wrong with the code. If it crashes it's fine since it's a simulator anyway
        #[allow(clippy::unwrap_used)]
        let packet = packet_builder
            .build_packet(
                input.data.data,
                &sphinx_route,
                &input.options.dst.as_sphinx_destination(),
                &delays,
            )
            .unwrap();

        let attributes = ForwardSphinxMessage {
            key_rotation: SphinxKeyRotation::EvenRotation, // Doesn't matter at all
            next_hop: first_mix_hop as u32,
        };
        let framed_packet = LpFrame::new_with_attributes(
            LpFrameKind::ForwardSphinxPacket,
            attributes,
            packet.to_bytes(),
        );

        PipelinePayload::new(
            input.data.timestamp,
            framed_packet.to_bytes(),
            input.options,
            input.dst,
        )
    }
}

impl<R: Rng> Framing<SimNymClientInputOptions> for SimNymClientWrappingPipeline<R> {
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<SimNymClientInputOptions>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame>> {
        // SAFETY : we know the inupt is long enough
        #[allow(clippy::unwrap_used)]
        let input_frame = LpFrame::decode(&payload.data.data).unwrap();

        fragment_lp_message(&mut self.rng, input_frame, frame_size)
            .into_iter()
            .map(|f| f.into_lp_frame())
            .map(|f| AddressedTimedData::new_addressed(payload.data.timestamp, f, payload.dst))
            .collect()
    }
}

impl<R: Rng> Transport<EncryptedLpPacket> for SimNymClientWrappingPipeline<R> {
    type Frame = LpFrame;
    // the simulated client has no LP session with its entry gateway, so its wrap cannot fail
    type Error = Infallible;
    const OVERHEAD_SIZE: usize = EncryptedLpPacket::OVERHEAD;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame>,
    ) -> Result<AddressedTimedData<EncryptedLpPacket>, Self::Error> {
        Ok(frame
            .data_transform(|f| LpPacket::new(LpHeader::new(0, 0, version::CURRENT), f).encode()))
    }
}

impl<R: Rng> WireWrappingPipeline<EncryptedLpPacket, SimNymClientInputOptions>
    for SimNymClientWrappingPipeline<R>
{
    fn packet_size(&self) -> usize {
        nym_lp_data::packet::MTU
    }
}

impl<R: Rng> ClientWrappingPipeline<EncryptedLpPacket, SimNymClientInputOptions>
    for SimNymClientWrappingPipeline<R>
{
}

// ─────────────────────────────────────────────────────────────────────────────
// Unwrapping pipeline
//
// The NymNodeDataPipeline currently drops final-hop packets, so in practice
// this client never receives anything useful. The unwrapper is still wired up
// for completeness — it decodes LP packets and would surface reassembled
// payloads if delivery were ever enabled.

/// Unwrapping pipeline for [`SimNymClient`].
pub struct NymNodeUnwrappingPipeline {
    message_reconstructor: MessageReconstructor,
    sphinx_message_reconstructor: SphinxMessageReconstructor,
    sphinx_secret_key: x25519::PrivateKey,
}

impl TransportUnwrap<EncryptedLpPacket> for NymNodeUnwrappingPipeline {
    type Frame = LpFrame;
    type Error = MalformedLpPacketError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Self::Frame>, Self::Error> {
        let lp = LpPacket::decode(packet)?;
        Ok(TimedData::new(timestamp, lp.into_frame()))
    }
}

impl FramingUnwrap<()> for NymNodeUnwrappingPipeline {
    type Frame = LpFrame;

    fn frame_to_message(&mut self, frame: TimedData<Self::Frame>) -> Option<(TimedPayload, ())> {
        let recovered_message = match frame.data.kind() {
            LpFrameKind::FragmentedData => {
                let fragment = frame.data.try_into().ok()?; // This should never fail
                self.message_reconstructor
                    .insert_new_fragment(fragment, frame.timestamp)?
                    .inspect_err(|e| tracing::warn!("Failed to reconstruct message : {e}"))
                    .ok()?
            }
            LpFrameKind::SphinxPacket => frame.data,
            f => {
                tracing::warn!("Unsupported lp frame : {f:?}");
                return None;
            }
        };
        Some((
            TimedPayload::new(frame.timestamp, recovered_message.content.to_vec()),
            (),
        ))
    }
}

impl WireUnwrappingPipeline<EncryptedLpPacket, ()> for NymNodeUnwrappingPipeline {}

impl ClientUnwrappingPipeline<EncryptedLpPacket, ()> for NymNodeUnwrappingPipeline {
    fn process_unwrapped(&mut self, timed_plaintext: TimedPayload, _: ()) -> Option<Vec<u8>> {
        let sphinx_packet = SphinxPacket::from_bytes(&timed_plaintext.data)
            .inspect_err(|e| tracing::warn!("Impossible to recover sphinx packet : {e}"))
            .ok()?;
        let processed_packet = sphinx_packet
            .process(self.sphinx_secret_key.inner())
            .inspect_err(|e| tracing::warn!("Impossible to process sphinx packet : {e}"))
            .ok()?
            .data;

        let ProcessedPacketData::FinalHop { payload, .. } = processed_packet else {
            tracing::warn!("Received a forward hop packet in a client, this shouldn't happen");
            return None;
        };

        let plaintext = payload
            .recover_plaintext()
            .inspect_err(|e| tracing::warn!("Impossible to recover plaintext : {e}"))
            .ok()?;

        let fragment = Fragment::try_from_bytes(&plaintext)
            .inspect_err(|e| tracing::warn!("Failed to deserialize fragment : {e}"))
            .ok()?;

        if let Some(reconstructed_message) = self
            .sphinx_message_reconstructor
            .insert_new_fragment(fragment)
        {
            let message = PaddedMessage::from(reconstructed_message.0)
                .remove_padding()
                .inspect_err(|e| tracing::warn!("Failed to remove padding : {e}"))
                .ok()?;
            Some(message.into_inner_data())
        } else {
            None
        }
    }
}
