// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SphinxClient`] — simulated client using full Sphinx encryption.
//!
//! The wrapping pipeline applies chunking, Sphinx encryption (routing security),
//! and Poisson cover traffic obfuscation.  The unwrapping pipeline reconstructs
//! fragmented messages and filters out cover traffic.

use std::sync::Arc;

use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    clients::{
        InputOptions, PipelinePayload,
        traits::{
            Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline, Obfuscation, Reliability,
            RoutingSecurity,
        },
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::{
    Delay, Destination, DestinationAddressBytes, SphinxPacketBuilder,
    chunking::{fragment::Fragment, reconstruction::MessageReconstructor},
    message::{NymMessage, PaddedMessage},
};
use rand::rngs::OsRng;

use crate::{
    client::{
        BaseClient, ClientId, ProcessingClient,
        sphinx::{poisson_cover_traffic::PoissonCoverTraffic, surb_acks::SurbAcksReliability},
    },
    node::NodeId,
    packet::sphinx::{GenerateDelay, SimMixPacket, SphinxMessage, SphinxNoOpWireWrapper, SurbAck},
    topology::{TopologyClient, directory::Directory},
};

mod poisson_cover_traffic;
mod surb_acks;

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Simple*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub type SphinxClient<Ts> = BaseClient<Ts, SphinxProcessingClient<Ts>, SimMixPacket, Vec<u8>>;

impl<Ts: Clone + GenerateDelay + PartialOrd + Send> SphinxClient<Ts> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(
        topology_client: TopologyClient,
        directory: Arc<Directory>,
        current_timestamp: Ts,
    ) -> anyhow::Result<Self> {
        let processing_client = SphinxProcessingClient {
            wrapper: SphinxClientWrappingPipeline {
                wire_wrapper: SphinxNoOpWireWrapper,
                cover_traffic: PoissonCoverTraffic::new(
                    topology_client.client_id,
                    directory.clone(),
                    current_timestamp,
                    OsRng,
                ),
                reliablity: SurbAcksReliability::new(
                    OsRng,
                    topology_client.client_id,
                    directory.clone(),
                ),
                directory: directory.clone(),
            },
            unwrapper: SphinxClientUnwrapping::default(),
        };
        BaseClient::with_pipeline(&topology_client, directory, processing_client)
    }
}

/// [`InputOptions`] for the Sphinx pipeline — routing security and obfuscation
/// are enabled; reliability is not.
#[derive(Clone, Copy)]
pub struct SphinxInputOptions {
    /// Destination client ID, embedded in the Sphinx destination address.
    dst: ClientId,
    /// First-hop node ID.  In a real Nym network this would be the client's
    /// gateway; here it is chosen at random from the topology because there is
    /// no gateway concept in the simulation.
    next_hop: NodeId,
}

impl InputOptions<NodeId> for SphinxInputOptions {
    fn reliability(&self) -> bool {
        true
    }

    fn routing_security(&self) -> bool {
        true
    }

    fn obfuscation(&self) -> bool {
        true
    }

    fn next_hop(&self) -> NodeId {
        self.next_hop
    }
}

/// Bridges [`BaseClient`] to the Sphinx wrapping and unwrapping pipelines.
pub struct SphinxProcessingClient<Ts: Clone + GenerateDelay + PartialOrd> {
    wrapper: SphinxClientWrappingPipeline<Ts>,
    unwrapper: SphinxClientUnwrapping,
}

impl<Ts: Clone + GenerateDelay + PartialOrd + Send> ProcessingClient<Ts, SimMixPacket, Vec<u8>>
    for SphinxProcessingClient<Ts>
{
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, SimMixPacket, NodeId>> {
        let input_options = SphinxInputOptions {
            dst,
            next_hop: self.wrapper.directory.random_next_hop(), // This substitutes for a real gateway selection — in the simulation every node is equally eligible as a first hop
        };
        self.wrapper.process(input, input_options, timestamp)
    }

    fn unwrap(&mut self, input: Vec<u8>, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>> {
        self.unwrapper.unwrap(input, timestamp)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

/// Full wrapping pipeline for [`SphinxClient`].
///
/// Applies, in order: chunking (using standard Sphinx fragmentation),
/// Poisson cover traffic obfuscation, Sphinx onion encryption, and the no-op
/// wire wrapper.
pub struct SphinxClientWrappingPipeline<Ts: Clone + GenerateDelay + PartialOrd> {
    wire_wrapper: SphinxNoOpWireWrapper,
    cover_traffic: PoissonCoverTraffic<Ts, OsRng>,
    reliablity: SurbAcksReliability<OsRng>,
    directory: Arc<Directory>,
}

pub(crate) type SphinxPipelinePayload<Ts> = PipelinePayload<Ts, SphinxInputOptions, NodeId>;

impl<Ts: Clone + GenerateDelay + PartialOrd> Chunking<Ts, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    fn chunked(
        &self,
        input: Vec<u8>,
        options: SphinxInputOptions,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<SphinxPipelinePayload<Ts>> {
        if input.is_empty() {
            return Vec::new();
        }

        // This is using standard sphinx chunking. Proper LP should use a different one
        let fragments = NymMessage::new_plain(input)
            .pad_to_full_packet_lengths(chunk_size)
            .split_into_fragments(&mut OsRng, chunk_size);

        fragments
            .into_iter()
            .map(|fragment| {
                SphinxPipelinePayload::new(timestamp.clone(), fragment.into_bytes(), options)
            })
            .collect()
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd> Reliability<Ts, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    const OVERHEAD_SIZE: usize =
        <SurbAcksReliability<OsRng> as Reliability<Ts, SphinxInputOptions, _>>::OVERHEAD_SIZE;
    fn reliable_encode(
        &mut self,
        input: Option<SphinxPipelinePayload<Ts>>,
        timestamp: Ts,
    ) -> Vec<SphinxPipelinePayload<Ts>> {
        self.reliablity.reliable_encode(input, timestamp)
    }
}
// impl<Ts: Clone + GenerateDelay + PartialOrd> NoOpReliability for SphinxClientWrappingPipeline<Ts> {}
//impl<Ts: Clone + GenerateDelay + PartialOrd> NoOpObfuscation for SphinxClientWrappingPipeline<Ts> {}
impl<Ts: Clone + GenerateDelay + PartialOrd> Obfuscation<Ts, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    fn obfuscate(
        &mut self,
        input: Option<SphinxPipelinePayload<Ts>>,
        timestamp: Ts,
    ) -> Vec<SphinxPipelinePayload<Ts>> {
        self.cover_traffic.obfuscate(input, timestamp)
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd> RoutingSecurity<Ts, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    const OVERHEAD_SIZE: usize = nym_sphinx::HEADER_SIZE + nym_sphinx::PAYLOAD_OVERHEAD_SIZE;
    fn nb_frames(&self) -> usize {
        1
    }
    /// Wrap `input` in a Sphinx onion packet with a 3-hop route.
    ///
    /// The route is built by taking `input_options.next_hop` as the first hop
    /// and choosing two additional hops at random from the directory (repeats are
    /// allowed).  The final destination is the client identified by
    /// `input_options.dst`.  Per-hop delays are drawn from
    /// [`GenerateDelay::generate_mix_delay`].
    fn encrypt(&self, input: SphinxPipelinePayload<Ts>) -> SphinxPipelinePayload<Ts> {
        // SAFETY: IDs were sampled from the directory, so they are guaranteed to exist.
        #[allow(clippy::unwrap_used)]
        let first_hop = self.directory.node(input.options.next_hop).unwrap().into();

        let route = std::iter::once(first_hop)
            .chain(
                self.directory
                    .random_route(2, &mut OsRng)
                    .iter()
                    .map(Into::into),
            )
            .collect::<Vec<_>>();

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([input.options.dst; 32]),
            [input.options.dst; 16],
        );

        let delays = (0..route.len())
            .map(|_| Delay::new_from_millis(Ts::generate_mix_delay(&mut OsRng)))
            .collect::<Vec<_>>();

        // Useful payload size is packet size - transport overhead - framing overhead - routing overhead
        let plaintext_size =
            <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, _, _, _>>::packet_size(
                &self.wire_wrapper,
            ) - <Self as Framing<Ts, _, _>>::OVERHEAD_SIZE
                - <Self as Transport<Ts, _, _, _>>::OVERHEAD_SIZE
                - <Self as RoutingSecurity<Ts, _, _>>::OVERHEAD_SIZE;

        // Packet builder's size includes the payload overhead so we have to add it
        let packet_builder = SphinxPacketBuilder::new()
            .with_payload_size(plaintext_size + nym_sphinx::PAYLOAD_OVERHEAD_SIZE);

        // SAFETY : Shut up clippy
        #[allow(clippy::unwrap_used)]
        let packet = packet_builder
            .build_packet(input.data.data, &route, &destination, &delays)
            .unwrap();

        SphinxPipelinePayload::new(input.data.timestamp, packet.to_bytes(), input.options)
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd> Framing<Ts, Vec<u8>, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Framing<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NodeId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedPayload<Ts, NodeId>> {
        self.wire_wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd> Transport<Ts, Vec<u8>, SimMixPacket, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Transport<Ts, _, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedPayload<Ts, NodeId>,
    ) -> AddressedTimedData<Ts, SimMixPacket, NodeId> {
        self.wire_wrapper.to_transport_packet(frame)
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd> WireWrappingPipeline<Ts, Vec<u8>, SimMixPacket, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
    fn packet_size(&self) -> usize {
        <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, _, _, _>>::packet_size(
            &self.wire_wrapper,
        )
    }
}

impl<Ts: Clone + GenerateDelay + PartialOrd>
    ClientWrappingPipeline<Ts, Vec<u8>, SimMixPacket, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline<Ts>
{
}
// ─────────────────────────────────────────────────────────────────────────────

/// Unwrapping pipeline for [`SphinxClient`].
///
/// Receives the raw final-hop payload (the last Sphinx layer has already been
/// stripped by the terminal mix node), recovers the plaintext, filters cover
/// traffic, and reassembles Sphinx fragments into complete messages.
#[derive(Default)]
pub struct SphinxClientUnwrapping {
    message_reconstructor: MessageReconstructor,
}

impl<Ts> FramingUnwrap<Ts, Vec<u8>, SphinxMessage> for SphinxClientUnwrapping {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, Vec<u8>>,
    ) -> Option<(TimedPayload<Ts>, SphinxMessage)> {
        Some((frame, SphinxMessage))
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, Vec<u8>, Vec<u8>> for SphinxClientUnwrapping {
    fn packet_to_frame(
        &self,
        packet: Vec<u8>,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, Vec<u8>>> {
        Ok(TimedData {
            timestamp,
            data: packet,
        })
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, Vec<u8>, Vec<u8>, SphinxMessage>
    for SphinxClientUnwrapping
{
}

impl<Ts: Clone> ClientUnwrappingPipeline<Ts, Vec<u8>, Vec<u8>, SphinxMessage>
    for SphinxClientUnwrapping
{
    fn process_unwrapped(
        &mut self,
        timed_plaintext: TimedPayload<Ts>,
        _kind: SphinxMessage,
    ) -> Option<Vec<u8>> {
        let plaintext = timed_plaintext.data;

        // Ditch cover traffic
        if nym_sphinx::cover::is_cover(&plaintext) {
            tracing::debug!("Received cover traffic packet");
            return None;
        }

        // TODO Route acks elsewhere HERE
        if SurbAck::is_surb_ack(&plaintext) {
            // SAFETY : casting slice of len 8 into array of len 8
            #[allow(clippy::unwrap_used)]
            let id = u64::from_le_bytes(plaintext[8..16].try_into().unwrap());
            tracing::debug!("Received a SURB_ACK for id : {id}");
            return None;
        }

        let fragment = Fragment::try_from_bytes(&plaintext)
            .inspect_err(|e| tracing::warn!("Failed to deserialize fragment : {e}"))
            .ok()?;

        if let Some(reconstructed_message) =
            self.message_reconstructor.insert_new_fragment(fragment)
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
