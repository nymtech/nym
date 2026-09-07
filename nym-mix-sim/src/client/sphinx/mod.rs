// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SphinxClient`] — simulated client using full Sphinx encryption.
//!
//! The wrapping pipeline applies chunking, Sphinx encryption (routing security),
//! and Poisson cover traffic obfuscation.  The unwrapping pipeline reconstructs
//! fragmented messages and filters out cover traffic.

use std::{sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedPayload,
    clients::traits::{
        Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline, Obfuscation, Reliability,
        RoutingSecurity,
    },
    common::{
        helpers::{NoOpWireUnwrapper, NoOpWireWrapper},
        traits::{Framing, Transport, WireWrappingPipeline},
    },
};
use nym_sphinx::{
    Delay, SphinxPacketBuilder,
    chunking::{fragment::Fragment, reconstruction::MessageReconstructor},
    message::{NymMessage, PaddedMessage},
};
use rand::Rng;

use crate::{
    client::{
        BaseClient, ClientId, ProcessingClient,
        sphinx::{poisson_cover_traffic::PoissonCoverTraffic, surb_acks::SurbAcksReliability},
    },
    helpers,
    packet::sphinx::{SimMixPacket, SurbAck},
    topology::{
        TopologyClient,
        directory::{Directory, DirectoryClient, DirectoryNode},
    },
};

mod poisson_cover_traffic;
mod surb_acks;

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Sphinx*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub type SphinxClient<R> = BaseClient<SphinxProcessingClient<R>, SimMixPacket, Vec<u8>>;

impl<R: Rng + Clone + Send> SphinxClient<R> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(
        topology_client: TopologyClient,
        directory: Arc<Directory>,
        current_timestamp: Instant,
        rng: R,
    ) -> anyhow::Result<Self> {
        let processing_client = SphinxProcessingClient {
            wrapper: SphinxClientWrappingPipeline {
                cover_traffic: PoissonCoverTraffic::new(
                    (&topology_client).into(),
                    directory.clone(),
                    current_timestamp,
                    rng.clone(),
                ),
                reliability: SurbAcksReliability::new(
                    rng.clone(),
                    (&topology_client).into(),
                    directory.clone(),
                ),
                directory,
                rng,
            },
            unwrapper: SphinxClientUnwrapping::default(),
        };
        BaseClient::with_pipeline(
            topology_client.client_id,
            topology_client.mixnet_address,
            topology_client.app_address,
            processing_client,
        )
    }
}

#[derive(Clone, Copy)]
pub struct SphinxInputOptions {
    /// Destination client
    dst: DirectoryClient,
    first_hop: DirectoryNode,
}

/// Bridges [`BaseClient`] to the Sphinx wrapping and unwrapping pipelines.
pub struct SphinxProcessingClient<R: Rng> {
    wrapper: SphinxClientWrappingPipeline<R>,
    unwrapper: SphinxClientUnwrapping,
}

impl<R: Rng + Send> ProcessingClient<SimMixPacket, Vec<u8>> for SphinxProcessingClient<R> {
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<SimMixPacket>> {
        let first_hop = self
            .wrapper
            .directory
            .random_next_hop(&mut self.wrapper.rng);

        let Some(&destination_client) = self.wrapper.directory.client(dst) else {
            tracing::error!("Destination {dst} does not exist in the topology");
            return Vec::new();
        };

        let input_options = SphinxInputOptions {
            dst: destination_client,
            first_hop,
        };
        // SAFETY: this pipeline's transport is `Infallible`
        #[allow(clippy::unwrap_used)]
        self.wrapper
            .process(Some((input, input_options, first_hop.addr)), timestamp)
            .unwrap()
    }

    fn unwrap(&mut self, input: Vec<u8>, timestamp: Instant) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.unwrapper.unwrap(input, timestamp)?)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

/// Full wrapping pipeline for [`SphinxClient`].
///
/// Applies, in order: chunking (using standard Sphinx fragmentation), SURB-ACK
/// reliability prefix, Poisson cover traffic obfuscation, Sphinx onion
/// encryption, and a no-op wire wrapper (a Sphinx packet is already its own
/// wire unit).
pub struct SphinxClientWrappingPipeline<R: Rng> {
    /// Poisson cover traffic generator providing the [`Obfuscation`] stage.
    cover_traffic: PoissonCoverTraffic<R>,
    /// SURB-ACK reliability layer providing the [`Reliability`] stage.
    reliability: SurbAcksReliability<R>,
    /// Shared routing table; used to sample the 3-hop Sphinx route in `encrypt`.
    directory: Arc<Directory>,
    /// RNG used for random route selection and Sphinx delay sampling.
    rng: R,
}

impl<R: Rng> Chunking<SphinxInputOptions> for SphinxClientWrappingPipeline<R> {
    fn chunked(
        &mut self,
        input: PipelinePayload<SphinxInputOptions>,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<SphinxInputOptions>> {
        let input_data = input.data.data;
        if input_data.is_empty() {
            return Vec::new();
        }

        // This is using standard sphinx chunking. Proper LP should use a different one
        let fragments = NymMessage::new_plain(input_data)
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

impl<R: Rng> Reliability<SphinxInputOptions> for SphinxClientWrappingPipeline<R> {
    const OVERHEAD_SIZE: usize =
        <SurbAcksReliability<R> as Reliability<SphinxInputOptions>>::OVERHEAD_SIZE;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<SphinxInputOptions>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<SphinxInputOptions>> {
        self.reliability.reliable_encode(input, timestamp)
    }
}

impl<R: Rng> Obfuscation<SphinxInputOptions> for SphinxClientWrappingPipeline<R> {
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<SphinxInputOptions>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<SphinxInputOptions>> {
        self.cover_traffic.obfuscate(input, timestamp)
    }
}

impl<R: Rng> RoutingSecurity<SphinxInputOptions> for SphinxClientWrappingPipeline<R> {
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
    /// [`crate::helpers::generate_mix_delay`].
    fn encrypt(
        &mut self,
        input: PipelinePayload<SphinxInputOptions>,
    ) -> PipelinePayload<SphinxInputOptions> {
        let first_hop = input.options.first_hop.as_sphinx_node_socket();

        let route = std::iter::once(first_hop)
            .chain(
                self.directory
                    .random_route(2, &mut self.rng, None)
                    .iter()
                    .map(|n| n.as_sphinx_node_socket()),
            )
            .collect::<Vec<_>>();

        let destination = input.options.dst.as_sphinx_destination();

        let delays = (0..route.len())
            .map(|_| Delay::new_from_millis(helpers::generate_mix_delay(&mut self.rng)))
            .collect::<Vec<_>>();

        // Useful payload size is packet size - transport overhead - framing overhead - routing overhead
        let plaintext_size =
            <Self as WireWrappingPipeline<SimMixPacket, SphinxInputOptions>>::packet_size(self)
                - <Self as Framing<SphinxInputOptions>>::OVERHEAD_SIZE
                - <Self as Transport<SimMixPacket>>::OVERHEAD_SIZE
                - <Self as RoutingSecurity<_>>::OVERHEAD_SIZE;

        // Packet builder's size includes the payload overhead so we have to add it
        let packet_builder = SphinxPacketBuilder::new()
            .with_payload_size(plaintext_size + nym_sphinx::PAYLOAD_OVERHEAD_SIZE);

        // SAFETY : If the pipeline is built correctly, the packet building should not fail.
        // If it does, something is wrong with the code. If it crashes it's fine since it's a simulator anyway
        #[allow(clippy::unwrap_used)]
        let packet = packet_builder
            .build_packet(input.data.data, &route, &destination, &delays)
            .unwrap();

        PipelinePayload::new(
            input.data.timestamp,
            packet.to_bytes(),
            input.options,
            input.dst,
        )
    }
}

impl<R: Rng> NoOpWireWrapper for SphinxClientWrappingPipeline<R> {}

impl<R: Rng> ClientWrappingPipeline<SimMixPacket, SphinxInputOptions>
    for SphinxClientWrappingPipeline<R>
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

impl NoOpWireUnwrapper for SphinxClientUnwrapping {}

impl ClientUnwrappingPipeline<Vec<u8>, ()> for SphinxClientUnwrapping {
    fn process_unwrapped(&mut self, timed_plaintext: TimedPayload, _: ()) -> Option<Vec<u8>> {
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
