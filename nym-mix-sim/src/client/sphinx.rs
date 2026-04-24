// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nym_lp_data::{
    AddressedTimedData, TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline, RoutingSecurity},
    },
    common::traits::{
        Framing, FramingUnwrap, InputOptions, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::{Delay, Destination, DestinationAddressBytes, Payload, SphinxPacket};
use rand::{Rng, rngs::OsRng, seq::SliceRandom};

use crate::{
    client::{BaseClient, ClientId, ProcessingClient},
    node::NodeId,
    packet::sphinx::{SimSphinxPacket, SphinxMessage, SphinxNoOpWireWrapper},
    topology::{TopologyClient, directory::Directory},
};

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Simple*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub type SphinxClient<Ts> = BaseClient<Ts, SphinxProcessingClient, SimSphinxPacket, Vec<u8>>;

impl<Ts> SphinxClient<Ts> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(topology_client: TopologyClient, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let processing_client = SphinxProcessingClient {
            wrapper: SphinxClientWrappingPipeline {
                wire_wrapper: SphinxNoOpWireWrapper,
                directory: directory.clone(),
            },
            unwrapper: SphinxClientUnwrapping,
        };
        BaseClient::with_pipeline(&topology_client, directory, processing_client)
    }
}

#[derive(Clone, Copy)]
pub struct SphinxInputOptions {
    dst: ClientId,
    // In practice, this is the gateway's ID
    // Here since we're not doing gateway, it will get generated randomly and be the first node's ID
    next_hop: NodeId,
}

impl InputOptions<NodeId> for SphinxInputOptions {
    fn reliability(&self) -> bool {
        false
    }

    fn routing_security(&self) -> bool {
        true
    }

    fn obfuscation(&self) -> bool {
        false
    }

    fn next_hop(&self) -> NodeId {
        self.next_hop
    }
}

pub struct SphinxProcessingClient {
    wrapper: SphinxClientWrappingPipeline,
    unwrapper: SphinxClientUnwrapping,
}

impl<Ts: Clone> ProcessingClient<Ts, SimSphinxPacket, Vec<u8>> for SphinxProcessingClient {
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Ts,
    ) -> Vec<AddressedTimedData<Ts, SimSphinxPacket, NodeId>> {
        let input_options = SphinxInputOptions {
            dst,
            next_hop: self.wrapper.random_next_hop(),
        };
        self.wrapper.process(input, input_options, timestamp)
    }

    fn unwrap(&mut self, input: Vec<u8>, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>> {
        self.unwrapper.unwrap(input, timestamp)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

pub struct SphinxClientWrappingPipeline {
    wire_wrapper: SphinxNoOpWireWrapper,
    directory: Arc<Directory>,
}

impl SphinxClientWrappingPipeline {
    pub fn random_next_hop(&self) -> NodeId {
        // SAFETY : Directory can't be empty of nodes in the sim
        #[allow(clippy::unwrap_used)]
        *self.directory.node_ids().choose(&mut OsRng).unwrap() // see comments in SphinxInputOptions as to why we are doing this
    }
}

impl<Ts: Clone> Chunking<Ts, SphinxInputOptions, NodeId> for SphinxClientWrappingPipeline {
    fn chunked(
        &self,
        input: Vec<u8>,
        _: SphinxInputOptions,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<TimedPayload<Ts>> {
        // SW TODO Framing for chunking and reconstruction on the other side
        input
            .chunks(chunk_size)
            .map(|chunk| TimedData {
                data: chunk.to_vec(),
                timestamp: timestamp.clone(),
            })
            .collect()
    }
}

impl NoOpReliability for SphinxClientWrappingPipeline {}
impl NoOpObfuscation for SphinxClientWrappingPipeline {}

impl<Ts: Clone> RoutingSecurity<Ts, SphinxInputOptions, NodeId> for SphinxClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = nym_sphinx::params::PacketSize::RegularPacket.header_size()
        + nym_sphinx::params::PacketSize::RegularPacket.payload_overhead();
    fn nb_frames(&self) -> usize {
        1
    }
    fn encrypt(
        &self,
        input: TimedPayload<Ts>,
        input_options: SphinxInputOptions,
    ) -> TimedPayload<Ts> {
        let mut route_ids = vec![input_options.next_hop];
        for _ in 0..2 {
            route_ids.push(self.random_next_hop()); // I don't care if we go through the same one multiple time
        }

        let route = route_ids
            .into_iter()
            .map(|id| {
                // SAFETY : We just took a random route from the directory, nodes must exists in said directory
                #[allow(clippy::unwrap_used)]
                self.directory.node(id).unwrap().into()
            })
            .collect::<Vec<_>>();
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([input_options.dst; 32]),
            [input_options.dst; 16],
        );

        let delays = (0..route.len())
            .map(|_| Delay::new_from_millis(OsRng.gen_range(0..=10)))
            .collect::<Vec<_>>();

        // SAFETY : Shut up clippy
        #[allow(clippy::unwrap_used)]
        let packet = SphinxPacket::new(input.data, &route, &destination, &delays).unwrap();
        TimedData {
            timestamp: input.timestamp,
            data: packet.to_bytes(),
        }
    }
}

impl<Ts: Clone> Framing<Ts, SphinxPacket> for SphinxClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SphinxPacket>> {
        self.wire_wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket, NodeId>
    for SphinxClientWrappingPipeline
{
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Transport<Ts, _, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SphinxPacket>,
        next_hop: NodeId,
    ) -> AddressedTimedData<Ts, SimSphinxPacket, NodeId> {
        self.wire_wrapper.to_transport_packet(frame, next_hop)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, NodeId>
    for SphinxClientWrappingPipeline
{
    fn packet_size(&self) -> usize {
        <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, _, _, _>>::packet_size(
            &self.wire_wrapper,
        )
    }
}

impl<Ts: Clone>
    ClientWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, SphinxInputOptions, NodeId>
    for SphinxClientWrappingPipeline
{
}
// ─────────────────────────────────────────────────────────────────────────────

// Since the client does not unwrap the last layer, we get the message directly
pub struct SphinxClientUnwrapping;

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
        payload: TimedPayload<Ts>,
        _kind: SphinxMessage,
    ) -> Option<Vec<u8>> {
        // SW TODO reconstruction
        Payload::from_bytes(&payload.data)
            .inspect_err(|e| tracing::warn!("Somehow received a packet that was too short : {e}"))
            .ok()?
            .recover_plaintext()
            .inspect_err(|e| tracing::warn!("Impossible to recover plaintext : {e}"))
            .ok()
    }
}
