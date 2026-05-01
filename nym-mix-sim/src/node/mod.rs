// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use nym_lp_data::AddressedTimedData;

use crate::{packet::WirePacketFormat, topology::directory::Directory};

pub mod simple;
pub mod sphinx;

/// Compact identifier for a mix node in the simulation topology.
///
/// `u8` keeps the IDs small (max 255 nodes) and is large enough for any
/// realistic simulated topology.
pub type NodeId = u8;

/// Driver-facing interface for a mix node.
///
/// Erases `Pkt` and `Pn` so that [`MixSimDriver`] only needs `Ts`.
/// Implemented by [`BaseNode<Ts, Pkt, Pn>`] for any compatible `Pkt` and
/// `Pn`.
///
/// [`MixSimDriver`]: crate::driver::MixSimDriver
pub trait MixSimNode<Ts: Clone + PartialOrd + Debug + Send>: Send {
    /// **Phase 1** — drain the UDP socket into the inbound buffer
    fn tick_incoming(&mut self);

    /// **Phase 2** — pass every buffered packet through the mix pipeline and
    /// move the results into the outbound queue.
    fn tick_processing(&mut self, timestamp: Ts);

    /// **Phase 3** — forward all outbound packets whose scheduled timestamp is
    /// ≤ `timestamp` to their next-hop address.
    fn tick_outgoing(&mut self, timestamp: Ts);

    /// Pretty-print the node's current buffer state to stdout (used in manual mode).
    fn display_state(&self);
}

/// Minimal pipeline interface used by [`BaseNode`].
///
/// Hides the `Frame` and message-marker type parameters of
/// [`MixnodeProcessingPipeline`] so that [`BaseNode`] only needs
/// `<Ts, Pkt, Pipeline>` rather than five generics.
///
/// Implement [`MixnodeProcessingPipeline`] on your concrete type and then add
/// a trivial delegation impl of this trait; the two-line body just calls
/// through to [`MixnodeProcessingPipeline::process`].
///
/// [`MixnodeProcessingPipeline`]: nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline
/// [`MixnodeProcessingPipeline::process`]: nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline::process
pub trait ProcessingNode<Ts, Pkt>: Send {
    fn process(
        &mut self,
        input: Pkt,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<AddressedTimedData<Ts, Pkt, NodeId>>>;
}

/// Full mix-node state: UDP transport, routing directory, packet buffers, and
/// processing pipeline.
///
/// `Ts` is the timestamp / tick-context type.  `Pkt` is the wire packet type
/// (e.g. [`SimplePacket`] or [`SimMixPacket`]).  `Pn` is any type that
/// implements [`ProcessingNode<Ts, Pkt>`].
///
/// Concrete node variants (`SimpleNode`, `SphinxNode`, …) are type aliases
/// over this struct and only need to supply a `new()` constructor that wires
/// up the right pipeline.
///
/// [`SimplePacket`]: crate::packet::simple::SimplePacket
/// [`SimMixPacket`]: crate::packet::sphinx::SimMixPacket
pub struct BaseNode<Ts, Pkt, Pn> {
    pub(crate) id: NodeId,
    _reliability: u8, // Unused yet, can be used later for testing the reliability layer
    pub(crate) socket_address: SocketAddr,
    socket: UdpSocket,
    directory: Arc<Directory>,

    packets_to_process: Vec<Pkt>,
    processed_packets: Vec<AddressedTimedData<Ts, Pkt, NodeId>>,

    processing_node: Pn,
}

impl<Ts, Pkt, Pn> BaseNode<Ts, Pkt, Pn> {
    /// Bind a non-blocking UDP socket to `socket_address` and initialise the
    /// node with the given `pipeline`.
    pub(crate) fn with_pipeline(
        id: NodeId,
        reliability: u8,
        socket_address: SocketAddr,
        directory: Arc<Directory>,
        processing_node: Pn,
    ) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(socket_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            id,
            _reliability: reliability,
            socket_address,
            socket,
            directory,
            packets_to_process: Vec::new(),
            processed_packets: Vec::new(),
            processing_node,
        })
    }

    /// Send `packet` to the node or client identified by `node_id`.
    ///
    /// Resolves `node_id` against the shared [`Directory`], serialises via
    /// [`WirePacketFormat::to_bytes`], and dispatches with a single `sendto`.
    /// Errors are logged but not propagated.
    pub fn send_to_node(&self, node_id: NodeId, packet: Pkt)
    where
        Pkt: WirePacketFormat,
    {
        if let Some(node) = self.directory.node(node_id) {
            if let Err(e) = self.socket.send_to(&packet.to_bytes(), node.addr) {
                tracing::error!(
                    "[Node {}] Failed to send data to node {node_id} : {e}",
                    self.id
                );
            } else {
                tracing::debug!(
                    "[Node {}] Successfully sent a packet to node {node_id}",
                    self.id
                );
            }
        } else if let Some(client) = self.directory.client(node_id) {
            if let Err(e) = self.socket.send_to(&packet.to_bytes(), client) {
                tracing::error!(
                    "[Node {}] Failed to send data to client {node_id} : {e}",
                    self.id
                );
            } else {
                tracing::debug!(
                    "[Node {}] Successfully sent a packet to client {node_id} @ {client}",
                    self.id
                );
            }
        } else {
            tracing::error!(
                "[Node {}] Trying to send to non-existing node/client {node_id}",
                self.id
            );
        }
    }

    /// Attempt to receive one UDP datagram and deserialise it as `Pkt`.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_packet(&self) -> Option<anyhow::Result<Pkt>>
    where
        Pkt: WirePacketFormat,
    {
        let mut buf = [0; 1500];
        let (nb_bytes, src_address) = match self.socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("Error receiving packet : {e}");
                return None;
            }
        };
        tracing::debug!(
            "[Node {}] Received {nb_bytes} bytes from {src_address}",
            self.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb_bytes]))
    }
}

impl<Ts, Pkt, Pn> MixSimNode<Ts> for BaseNode<Ts, Pkt, Pn>
where
    Ts: Clone + PartialOrd + Debug + Send,
    Pkt: WirePacketFormat + Debug + Send,
    Pn: ProcessingNode<Ts, Pkt>,
{
    fn tick_incoming(&mut self) {
        while let Some(maybe_packet) = self.recv_packet() {
            match maybe_packet {
                Ok(packet) => self.packets_to_process.push(packet),
                Err(e) => tracing::error!("[Node {}] Failed to deserialize packet : {e}", self.id),
            }
        }
    }

    fn tick_processing(&mut self, timestamp: Ts) {
        while let Some(packet) = self.packets_to_process.pop() {
            match self.processing_node.process(packet, timestamp.clone()) {
                Ok(processed_packets) => self.processed_packets.extend(processed_packets),
                Err(e) => {
                    tracing::error!("[Node {}] Failed to process packet : {e}", self.id)
                }
            }
        }
    }

    fn tick_outgoing(&mut self, timestamp: Ts) {
        let to_send = self
            .processed_packets
            .extract_if(.., |pkt| pkt.data.timestamp <= timestamp)
            .collect::<Vec<_>>();
        for pkt in to_send {
            self.send_to_node(pkt.dst, pkt.data.data);
        }
    }

    fn display_state(&self) {
        println!("│  Node {:2} @ {}", self.id, self.socket_address);
        if self.packets_to_process.is_empty() {
            println!("│    to_process buffer: (empty)");
        } else {
            println!(
                "│    to_process buffer: {} packet(s)",
                self.packets_to_process.len()
            );
            for (i, pkt) in self.packets_to_process.iter().enumerate() {
                println!("│      [{i}] {pkt:?}");
            }
        }

        if self.processed_packets.is_empty() {
            println!("│    processed buffer: (empty)");
        } else {
            println!(
                "│    processed buffer: {} packet(s)",
                self.processed_packets.len()
            );
            for (i, pkt) in self.processed_packets.iter().enumerate() {
                println!("│      [{i}] {pkt:?}");
            }
        }
    }
}
