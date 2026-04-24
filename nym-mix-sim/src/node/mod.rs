// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Individual mix-node model.
//!
//! A [`Node`] represents one mix node in the simulated network.  Each node owns
//! a non-blocking UDP socket and two internal packet buffers:
//!
//! * **`packets_to_process`** — packets received this tick that have not yet
//!   been mixed.
//! * **`processed_packets`** — packets that have been mixed and are waiting to
//!   be forwarded.
//!
//! The three tick methods advance the node through one simulation step:
//!
//! ```text
//! tick_incoming  →  packets_to_process
//!                       ↓  tick_processing (MixnodeProcessingPipeline)
//!                   processed_packets
//!                       ↓  tick_outgoing
//!                   (sent to next-hop via UDP)
//! ```
//!
//! Nodes share a reference-counted [`Directory`] so they can resolve target
//! addresses without locking.

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use nym_lp_data::{TimedData, mixnodes::traits::DynMixnodeProcessingPipeline};

use crate::{
    packet::WirePacketFormat,
    topology::{
        TopologyNode,
        directory::{Directory, DirectoryNode},
    },
};

/// Compact identifier for a mix node in the simulation topology.
///
/// `u8` keeps the IDs small (max 255 nodes) and is large enough for any
/// realistic simulated topology.
pub type NodeId = u8;

/// A running mix-node instance inside the simulation.
///
/// `Ts` is the timestamp / tick-context type.
/// `Pkt` is the packet type.
///
/// The struct is generic so that different packet formats (e.g. Sphinx-encrypted
/// packets) and richer tick contexts can be plugged in without changing node
/// internals.
pub struct Node<Ts, Pkt> {
    /// Shared routing table.  Set after construction via [`Node::set_directory`]
    /// once all nodes' sockets are bound and the [`Directory`] can be built.
    directory: Arc<Directory>,

    /// Static configuration for this node (id, reliability, listen address).
    id: NodeId,
    _reliability: u8,
    socket_address: SocketAddr,

    /// Non-blocking UDP socket bound to `details.addr`.
    /// Non-blocking mode is essential: [`Node::recv_packet`] must return
    /// immediately with `None` when no datagram is waiting, so that
    /// [`tick_incoming`] can drain the socket without blocking the entire
    /// simulation tick.
    ///
    /// [`tick_incoming`]: Node::tick_incoming
    socket: UdpSocket,

    // Internal Buffers
    /// Packets received during the current tick's [`tick_incoming`] phase that
    /// are waiting to be mixed.
    ///
    /// Drained by [`tick_processing`].
    ///
    /// [`tick_incoming`]: Node::tick_incoming
    /// [`tick_processing`]: Node::tick_processing
    packets_to_process: Vec<TimedData<Ts, Pkt>>,

    /// Packets that have been processed (mixed) and are waiting to be
    /// forwarded to the next hop.
    ///
    /// Drained by [`tick_outgoing`].
    processed_packets: Vec<(NodeId, TimedData<Ts, Pkt>)>,

    processing_pipeline: Box<dyn DynMixnodeProcessingPipeline<Ts, Pkt, NodeId> + Send>,
}

impl<Ts, Pkt> Node<Ts, Pkt> {
    /// Create a [`Node`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.addr`.
    ///
    /// The [`Directory`] is initialised to its default (empty) value and must
    /// be set later with [`Node::set_directory`] before the node attempts to
    /// send any packets.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound (e.g. address already
    /// in use) or if `set_nonblocking` fails.
    pub fn new(
        topology_node: TopologyNode,
        pipeline: impl DynMixnodeProcessingPipeline<Ts, Pkt, NodeId> + Send + 'static,
    ) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(topology_node.socket_address)?;
        socket.set_nonblocking(true)?;
        Ok(Node {
            directory: Default::default(),
            id: topology_node.node_id,
            _reliability: topology_node.reliability,
            socket_address: topology_node.socket_address,
            socket,
            packets_to_process: Vec::new(),
            processed_packets: Vec::new(),
            processing_pipeline: Box::new(pipeline),
        })
    }

    // /// Return this node's [`NodeId`].
    // pub fn id(&self) -> NodeId {
    //     self.id
    // }

    /// Attach the shared [`Directory`] to this node.
    ///
    /// Must be called before the first tick; otherwise [`send_to_node`] will
    /// fail to resolve any destination.
    ///
    /// [`send_to_node`]: Node::send_to_node
    pub fn set_directory(&mut self, directory: Arc<Directory>) {
        self.directory = directory
    }

    /// Build a [`DirectoryNode`] view of this node suitable for insertion into
    /// a [`Directory`].
    ///
    /// Called by [`Directory::build_from_nodes`] during driver initialisation.
    pub fn directory_node(&self) -> DirectoryNode {
        DirectoryNode {
            id: self.id,
            addr: self.socket_address,
        }
    }
}

impl<Ts: Debug, Pkt: Debug> Node<Ts, Pkt> {
    /// Print a bordered summary of this node's current buffer state to stdout.
    ///
    /// Displays the node ID, listen address, and — for each internal buffer —
    /// either "(empty)" or an indexed list of packet debug representations.
    /// Intended to be called by [`MixSimDriver::display_state`] which wraps all
    /// nodes' output inside a tick-labelled border.
    pub fn display_state(&self) {
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

impl<Ts, Pkt> Node<Ts, Pkt>
where
    Ts: Clone + PartialOrd,
    Pkt: WirePacketFormat,
{
    /// Send `packet` to the node identified by `node_id`.
    ///
    /// Resolves `node_id` against the shared [`Directory`], serialises the
    /// packet via [`WirePacketFormat::to_bytes`], and dispatches it with a
    /// single `sendto(2)` syscall on this node's UDP socket.
    ///
    /// Errors (unknown node, send failure) are logged but not propagated — the
    /// simulation continues even if individual sends fail.
    pub fn send_to_node(&self, node_id: NodeId, packet: Pkt) {
        if let Some(node) = self.directory.node(node_id) {
            if let Err(e) = self.socket.send_to(&packet.to_bytes(), node.addr) {
                tracing::error!(
                    "[Node {}] Failed to send data to node {node_id} : {e}",
                    self.id
                );
            } else {
                tracing::info!(
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
                tracing::info!(
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

    /// Attempt to receive one UDP datagram from this node's socket.
    ///
    /// Uses a 1 500-byte scratch buffer (standard Ethernet MTU) and delegates
    /// deserialisation to [`WirePacketFormat::try_from_bytes`].
    ///
    /// Returns:
    /// * `None` — socket would block (no datagram available).
    /// * `Some(Ok(pkt))` — a valid packet was received and deserialised.
    /// * `Some(Err(e))` — a datagram arrived but could not be deserialised;
    ///
    /// Any other socket error is logged and treated as if no packet arrived
    /// (returns `None`).
    ///
    pub fn recv_packet(&self) -> Option<anyhow::Result<Pkt>> {
        let mut buf = [0; 1500];
        let (nb_bytes, src_address) = match self.socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("Error receiving packet : {e}");
                return None;
            }
        };

        tracing::info!(
            "[Node {}] Received {nb_bytes} bytes from {src_address}",
            self.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb_bytes]))
    }

    /// **Phase 1 of a simulation tick**: drain the UDP socket into the
    /// `packets_to_process` buffer.
    ///
    /// Calls [`recv_packet`] in a loop until the socket would block, pushing
    /// each successfully deserialised packet onto `packets_to_process`.
    /// Deserialisation failures are logged and the offending datagram is
    /// discarded.
    ///
    pub fn tick_incoming(&mut self, timestamp: Ts) {
        while let Some(maybe_packet) = self.recv_packet() {
            match maybe_packet {
                Ok(packet) => self
                    .packets_to_process
                    .push(TimedData::new(timestamp.clone(), packet)),
                Err(e) => tracing::error!("[Node {}] Failed to deserialize packet : {e}", self.id),
            }
        }
    }

    /// **Phase 2 of a simulation tick**: apply the mix operation to every
    /// buffered packet.
    ///
    /// Pops packets from `packets_to_process` one at a time, passes each to
    /// [`MixnodeProcessingPipeline::process`], and extends `processed_packets`
    /// with the results (a packet may produce zero or more outputs).
    ///
    pub fn tick_processing(&mut self, timestamp: Ts) {
        while let Some(packet) = self.packets_to_process.pop() {
            match self.processing_pipeline.process(packet, timestamp.clone()) {
                Ok(processed_packets) => self.processed_packets.extend(processed_packets),
                Err(e) => tracing::error!("[Node {}] Failed to process packet : {e}", self.id),
            }
        }
    }

    /// **Phase 3 of a simulation tick**: forward all processed packets to their
    /// next hop.
    ///
    /// Extracts packets from `processed_packets` whose timestamp is ≤
    /// `timestamp` (i.e. due for delivery this tick) and calls
    /// [`send_to_node`] for each one.  Packets scheduled for a future tick
    /// remain in the buffer.  If the resolved node ID is not present in the
    /// [`Directory`] the send is logged as an error and the packet is dropped.
    ///
    pub fn tick_outgoing(&mut self, timestamp: Ts) {
        let to_send = self
            .processed_packets
            .extract_if(.., |(_, pkt)| pkt.timestamp <= timestamp)
            .map(|(next_hop, pkt)| (next_hop, pkt.data))
            .collect::<Vec<_>>();
        for (next_hop, pkt) in to_send {
            self.send_to_node(next_hop, pkt);
        }
    }
}
