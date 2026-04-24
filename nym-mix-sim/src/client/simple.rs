// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Simulated mix-network client.
//!
//! A [`SimpleClient`] owns a [`BaseClient`] (which manages both UDP sockets
//! and the routing directory) plus the mix and unwrapping pipelines.
//!
//! ## Tick phases
//!
//! ```text
//! tick_app_incoming ──── app_socket ──▶ processing_pipeline ──▶ outgoing_queue
//! tick_outgoing     ──── outgoing_queue ──▶ mix_socket ──▶ Node N
//! tick_mix_incoming ──── mix_socket ◀── Node N ──▶ unwrapping_pipeline
//! ```
//!
//! ## App-socket message format
//!
//! ```text
//! ┌─────────────────────┬─────────────────────┐
//! │  dst_node_id (1 B)  │  raw payload bytes  │
//! └─────────────────────┴─────────────────────┘
//! ```

use std::sync::Arc;

use nym_lp_data::{
    TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability, NoOpRoutingSecurity},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline},
        types::StreamOptions,
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};

use crate::{
    client::{BaseClient, ProcessingClient},
    packet::simple::{
        SimpleFrame, SimpleMessage, SimplePacket, SimpleWireUnwrapper, SimpleWireWrapper,
    },
    topology::{TopologyClient, directory::Directory},
};

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Simple*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub type SimpleClient<Ts> = BaseClient<Ts, SimpleProcessingClient, SimplePacket>;

impl<Ts> SimpleClient<Ts> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(topology_client: TopologyClient, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let processing_client = SimpleProcessingClient {
            wrapper: SimpleClientWrappingPipeline::default(),
            unwrapper: SimpleClientUnwrapping::default(),
        };
        BaseClient::with_pipeline(&topology_client, directory, processing_client)
    }
}

pub struct SimpleProcessingClient {
    wrapper: SimpleClientWrappingPipeline,
    unwrapper: SimpleClientUnwrapping,
}

impl<Ts: Clone> ProcessingClient<Ts, SimplePacket> for SimpleProcessingClient {
    fn process(
        &mut self,
        input: Vec<u8>,
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, SimplePacket>> {
        self.wrapper.process(input, processing_options, timestamp)
    }

    fn unwrap(&mut self, input: SimplePacket, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>> {
        self.unwrapper.unwrap(input, timestamp)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete pipelines

/// Stub client processing pipeline for [`SimplePacket`].
///
/// A no-op pass-through: returns the payload as a single packet with no
/// Sphinx layering, chunking, reliability encoding, or obfuscation.
///
/// All required sub-traits of [`ClientWrappingPipeline`] are implemented here;
/// [`ClientWrappingPipeline`] is then provided automatically via the blanket
/// impl in `nym_lp_data`.
pub struct SimpleClientWrappingPipeline(SimpleWireWrapper);

impl Default for SimpleClientWrappingPipeline {
    fn default() -> Self {
        Self(SimpleWireWrapper)
    }
}

impl<Ts: Clone> Chunking<Ts> for SimpleClientWrappingPipeline {
    fn chunked(
        &self,
        mut input: Vec<u8>,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<TimedPayload<Ts>> {
        // Padding with 10000...
        input.push(1);
        if !input.len().is_multiple_of(chunk_size) {
            let padding = vec![0; chunk_size - input.len() % chunk_size];
            input.extend_from_slice(&padding);
        }

        input
            .chunks(chunk_size)
            .map(|chunk| TimedData {
                data: chunk.to_vec(),
                timestamp: timestamp.clone(),
            })
            .collect()
    }
}

impl NoOpReliability for SimpleClientWrappingPipeline {}
impl NoOpObfuscation for SimpleClientWrappingPipeline {}
impl NoOpRoutingSecurity for SimpleClientWrappingPipeline {}

impl<Ts: Clone> Framing<Ts, SimpleFrame> for SimpleClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SimpleFrame>> {
        self.0.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SimpleFrame, SimplePacket> for SimpleClientWrappingPipeline {
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Transport<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> TimedData<Ts, SimplePacket> {
        self.0.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimpleFrame, SimplePacket>
    for SimpleClientWrappingPipeline
{
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<Ts, SimpleFrame, SimplePacket>>::packet_size(
            &self.0,
        )
    }
}

impl<Ts: Clone> ClientWrappingPipeline<Ts, SimpleFrame, SimplePacket>
    for SimpleClientWrappingPipeline
{
}
// ─────────────────────────────────────────────────────────────────────────────

pub struct SimpleClientUnwrapping(SimpleWireUnwrapper);

impl Default for SimpleClientUnwrapping {
    fn default() -> Self {
        Self(SimpleWireUnwrapper)
    }
}

impl<Ts> FramingUnwrap<Ts, SimpleFrame, SimpleMessage> for SimpleClientUnwrapping {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> Option<(TimedPayload<Ts>, SimpleMessage)> {
        self.0.frame_to_message(frame)
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimpleFrame, SimplePacket> for SimpleClientUnwrapping {
    fn packet_to_frame(
        &self,
        packet: SimplePacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SimpleFrame>> {
        self.0.packet_to_frame(packet, timestamp)
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleClientUnwrapping
{
}

impl<Ts: Clone> ClientUnwrappingPipeline<Ts, SimpleFrame, SimplePacket, SimpleMessage>
    for SimpleClientUnwrapping
{
    fn process_unwrapped(
        &mut self,
        payload: TimedPayload<Ts>,
        _kind: SimpleMessage,
    ) -> Option<Vec<u8>> {
        let mut data = payload.data;
        if let Some(pos) = data.iter().rposition(|&b| b == 1) {
            data.truncate(pos);
        }
        Some(data)
    }
}
