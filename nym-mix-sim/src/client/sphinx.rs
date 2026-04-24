// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nym_lp_data::{
    TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline, RoutingSecurity},
        types::StreamOptions,
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::SphinxPacket;

use crate::{
    client::{BaseClient, ProcessingClient},
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
            wrapper: SphinxClientWrappingPipeline::default(),
            unwrapper: SphinxClientUnwrapping,
        };
        BaseClient::with_pipeline(&topology_client, directory, processing_client)
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
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, SimSphinxPacket>> {
        self.wrapper.process(input, processing_options, timestamp)
    }

    fn unwrap(&mut self, input: Vec<u8>, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>> {
        self.unwrapper.unwrap(input, timestamp)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

pub struct SphinxClientWrappingPipeline(SphinxNoOpWireWrapper);

impl Default for SphinxClientWrappingPipeline {
    fn default() -> Self {
        Self(SphinxNoOpWireWrapper)
    }
}

impl<Ts: Clone> Chunking<Ts> for SphinxClientWrappingPipeline {
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
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

impl<Ts: Clone> RoutingSecurity<Ts> for SphinxClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = nym_sphinx::params::PacketSize::RegularPacket.header_size()
        + nym_sphinx::params::PacketSize::RegularPacket.payload_size();
    fn nb_frames(&self) -> usize {
        1
    }
    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        // SW Here be sphinx packet creation
        todo!()
    }
}

impl<Ts: Clone> Framing<Ts, SphinxPacket> for SphinxClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SphinxPacket>> {
        self.0.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket> for SphinxClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Transport<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SphinxPacket>,
    ) -> TimedData<Ts, SimSphinxPacket> {
        self.0.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket>
    for SphinxClientWrappingPipeline
{
    fn packet_size(&self) -> usize {
        <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket>>::packet_size(
            &self.0,
        )
    }
}

impl<Ts: Clone> ClientWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket>
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
        Some(payload.data)
    }
}
