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
//! ┌────────────────────────┬─────────────────────┐
//! │  dst_client_id (1 B)   │  raw payload bytes  │
//! └────────────────────────┴─────────────────────┘
//! ```

use std::{net::SocketAddr, sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    clients::{
        helpers::{NoOpObfuscation, NoOpReliability, NoOpRoutingSecurity},
        traits::{Chunking, ClientUnwrappingPipeline, ClientWrappingPipeline},
    },
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};

use crate::{
    client::{BaseClient, ClientId, ProcessingClient},
    packet::simple::{SimpleFrame, SimplePacket, SimpleWireUnwrapper, SimpleWireWrapper},
    topology::{TopologyClient, directory::Directory},
};

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Simple*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub type SimpleClient = BaseClient<SimpleProcessingClient, SimplePacket>;

impl SimpleClient {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(topology_client: TopologyClient, directory: Arc<Directory>) -> anyhow::Result<Self> {
        // SAFETY : node 0 always exists, otherwise we don't have any nodes
        #[allow(clippy::unwrap_used)]
        let first_hop_address = directory.node(0).unwrap().addr;
        let processing_client = SimpleProcessingClient {
            first_hop: first_hop_address,
            wrapper: SimpleClientWrappingPipeline::default(),
            unwrapper: SimpleClientUnwrapping::default(),
        };
        BaseClient::with_pipeline(
            topology_client.client_id,
            topology_client.mixnet_address,
            topology_client.app_address,
            processing_client,
        )
    }
}

/// Bridges [`BaseClient`] to the simple wrapping and unwrapping pipelines.
pub struct SimpleProcessingClient {
    first_hop: SocketAddr,
    wrapper: SimpleClientWrappingPipeline,
    unwrapper: SimpleClientUnwrapping,
}

impl ProcessingClient<SimplePacket> for SimpleProcessingClient {
    fn process(
        &mut self,
        input: Vec<u8>,
        _: ClientId,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<SimplePacket>> {
        // SAFETY: this pipeline's transport is `Infallible`
        #[allow(clippy::unwrap_used)]
        self.wrapper
            .process(Some((input, (), self.first_hop)), timestamp)
            .unwrap()
    }

    fn unwrap(
        &mut self,
        input: SimplePacket,
        timestamp: Instant,
    ) -> anyhow::Result<Option<Vec<u8>>> {
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

impl Chunking<()> for SimpleClientWrappingPipeline {
    /// Split `input` into chunks of `chunk_size` bytes, padding the last chunk
    /// with zero bytes if necessary.
    ///
    /// A `0x01` marker byte is appended before padding so the unwrapper can
    /// strip trailing zeros.
    fn chunked(
        &mut self,
        input: AddressedTimedPayload,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<AddressedTimedPayload> {
        let mut input_data = input.data.data;
        input_data.push(1);
        if !input_data.len().is_multiple_of(chunk_size) {
            let padding = vec![0; chunk_size - input_data.len() % chunk_size];
            input_data.extend_from_slice(&padding);
        }

        input_data
            .chunks(chunk_size)
            .map(|chunk| AddressedTimedPayload::new_addressed(timestamp, chunk.to_vec(), input.dst))
            .collect()
    }
}

impl NoOpReliability for SimpleClientWrappingPipeline {}
impl NoOpObfuscation for SimpleClientWrappingPipeline {}
impl NoOpRoutingSecurity for SimpleClientWrappingPipeline {}

// Delegation to SimpleWireWrapper
impl Framing<()> for SimpleClientWrappingPipeline {
    type Frame = SimpleFrame;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Framing<_>>::OVERHEAD_SIZE;
    fn to_frame(
        &mut self,
        payload: AddressedTimedPayload,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<SimpleFrame>> {
        self.0.to_frame(payload, frame_size)
    }
}

// Delegation to SimpleWireWrapper
impl Transport<SimplePacket> for SimpleClientWrappingPipeline {
    type Frame = SimpleFrame;
    type Error = <SimpleWireWrapper as Transport<SimplePacket>>::Error;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Transport<_>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<SimpleFrame>,
    ) -> Result<AddressedTimedData<SimplePacket>, Self::Error> {
        self.0.to_transport_packet(frame)
    }
}

// Delegation to SimpleWireWrapper
impl WireWrappingPipeline<SimplePacket, ()> for SimpleClientWrappingPipeline {
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<_, _>>::packet_size(&self.0)
    }
}

impl ClientWrappingPipeline<SimplePacket, ()> for SimpleClientWrappingPipeline {}
// ─────────────────────────────────────────────────────────────────────────────

/// Unwrapping pipeline for [`SimpleClient`]: strips the frame header and
/// removes padding from the recovered payload.
pub struct SimpleClientUnwrapping(SimpleWireUnwrapper);

impl Default for SimpleClientUnwrapping {
    fn default() -> Self {
        Self(SimpleWireUnwrapper)
    }
}

// Delegation to SimpleWireUnwrapper
impl FramingUnwrap<()> for SimpleClientUnwrapping {
    type Frame = SimpleFrame;
    fn frame_to_message(&mut self, frame: TimedData<SimpleFrame>) -> Option<(TimedPayload, ())> {
        self.0.frame_to_message(frame)
    }
}

// Delegation to SimpleWireUnwrapper
impl TransportUnwrap<SimplePacket> for SimpleClientUnwrapping {
    type Frame = SimpleFrame;
    type Error = anyhow::Error;
    fn packet_to_frame(
        &mut self,
        packet: SimplePacket,
        timestamp: Instant,
    ) -> anyhow::Result<TimedData<SimpleFrame>> {
        self.0.packet_to_frame(packet, timestamp)
    }
}

impl WireUnwrappingPipeline<SimplePacket, ()> for SimpleClientUnwrapping {}

impl ClientUnwrappingPipeline<SimplePacket, ()> for SimpleClientUnwrapping {
    fn process_unwrapped(&mut self, payload: TimedPayload, _: ()) -> Option<Vec<u8>> {
        let mut data = payload.data;
        if let Some(pos) = data.iter().rposition(|&b| b == 1) {
            data.truncate(pos);
        }
        Some(data)
    }
}
