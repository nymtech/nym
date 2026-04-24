// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use crate::{
    packet::WirePacketFormat,
    topology::{TopologyNode, directory::Directory},
};

pub mod simple;

/// Compact identifier for a mix node in the simulation topology.
///
/// `u8` keeps the IDs small (max 255 nodes) and is large enough for any
/// realistic simulated topology.
pub type NodeId = u8;

/// Shared UDP transport layer for mix nodes.
///
/// Encapsulates the socket, address, and routing directory so that multiple
/// concrete node types can reuse `send_to_node` and `recv_packet` without
/// duplicating that logic.  The packet type `Pkt` is a method-level generic
/// rather than a struct-level one, so `NodeSocket` itself has no type
/// parameters.
pub struct BaseNode {
    id: NodeId,
    _reliability: u8,
    socket_address: SocketAddr,
    socket: UdpSocket,
    directory: Arc<Directory>,
}

impl BaseNode {
    /// Bind a non-blocking UDP socket to `socket_address`.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(topology_node.socket_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            id: topology_node.node_id,
            _reliability: topology_node.reliability,
            socket_address: topology_node.socket_address,
            socket,
            directory,
        })
    }

    /// Send `packet` to the node or client identified by `node_id`.
    ///
    /// Resolves `node_id` against the shared [`Directory`], serialises via
    /// [`WirePacketFormat::to_bytes`], and dispatches with a single `sendto`.
    /// Errors are logged but not propagated.
    pub fn send_to_node<Pkt: WirePacketFormat>(&self, node_id: NodeId, packet: Pkt) {
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

    /// Attempt to receive one UDP datagram and deserialise it as `Pkt`.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_packet<Pkt: WirePacketFormat>(&self) -> Option<anyhow::Result<Pkt>> {
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
}

/// Driver-facing interface for a mix node.
///
/// Erases `Fr`, `Pkt`, and `Mk` so that [`MixSimDriver`] only needs `Ts`.
/// Implemented by [`SimpleNode<Ts, Fr, Pkt, Mk>`] and any other concrete node
/// types.
///
/// [`MixSimDriver`]: crate::driver::MixSimDriver
pub trait MixSimNode<Ts: Clone + PartialOrd + Debug + Send>: Send {
    fn tick_incoming(&mut self, timestamp: Ts);
    fn tick_processing(&mut self, timestamp: Ts);
    fn tick_outgoing(&mut self, timestamp: Ts);
    fn display_state(&self);
}
