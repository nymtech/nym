// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use crate::{
    node::NodeId,
    packet::WirePacketFormat,
    topology::{TopologyClient, directory::Directory},
};

pub mod simple;

/// Compact identifier for a simulated client.
pub type ClientId = NodeId;

/// Shared UDP transport layer for simulated clients.
///
/// Encapsulates both sockets, the routing directory, and the client id so that
/// multiple concrete client types can reuse `send_to_node`, `recv_from_mix`,
/// and `recv_from_app` without duplicating that logic.  Packet types are
/// method-level generics so `ClientSocket` itself has no type parameters.
pub struct BaseClient {
    id: ClientId,
    mix_socket: UdpSocket,
    mix_socket_address: SocketAddr,
    app_socket: UdpSocket,
    directory: Arc<Directory>,
}

impl BaseClient {
    /// Bind both UDP sockets to the given addresses.
    pub fn new(topology_client: TopologyClient, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let mix_socket = UdpSocket::bind(topology_client.mixnet_address)?;
        mix_socket.set_nonblocking(true)?;

        let app_socket = UdpSocket::bind(topology_client.app_address)?;
        app_socket.set_nonblocking(true)?;

        Ok(Self {
            id: topology_client.client_id,
            mix_socket,
            mix_socket_address: topology_client.mixnet_address,
            app_socket,
            directory,
        })
    }

    pub fn set_directory(&mut self, directory: Arc<Directory>) {
        self.directory = directory;
    }

    pub fn mixnet_address(&self) -> SocketAddr {
        self.mix_socket_address
    }

    /// Send `packet` to the mix node identified by `node_id` via `mix_socket`.
    ///
    /// Resolves `node_id` against the shared [`Directory`], serialises via
    /// [`WirePacketFormat::to_bytes`], and dispatches with a single `sendto`.
    /// Errors are logged but not propagated.
    pub fn send_to_node<Pkt: WirePacketFormat>(&self, node_id: NodeId, packet: Pkt) {
        if let Some(node) = self.directory.node(node_id) {
            if let Err(e) = self.mix_socket.send_to(&packet.to_bytes(), node.addr) {
                tracing::error!("[Client {}] Failed to send to node {node_id}: {e}", self.id);
            } else {
                tracing::info!(
                    "[Client {}] Sent packet to node {node_id} @ {}",
                    self.id,
                    node.addr
                );
            }
        } else {
            tracing::error!("[Client {}] Node {node_id} not found in directory", self.id);
        }
    }

    /// Attempt to receive one packet from the mix socket and deserialise it.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_from_mix<Pkt: WirePacketFormat>(&self) -> Option<anyhow::Result<Pkt>> {
        let mut buf = [0u8; 1500];
        let (nb, src) = match self.mix_socket.recv_from(&mut buf) {
            Ok(r) => r,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("[Client {}] mix_socket recv error: {e}", self.id);
                return None;
            }
        };
        tracing::info!(
            "[Client {}] Received {nb} byte(s) from mix node {src}",
            self.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb]))
    }

    /// Attempt to receive one raw datagram from the app socket.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_from_app(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let mut buf = [0u8; 1500];
        let nb = match self.app_socket.recv(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("[Client {}] app_socket recv error: {e}", self.id);
                return None;
            }
        };
        Some(Ok(buf[..nb].to_vec()))
    }
}

/// Driver-facing interface for a simulated client.
///
/// Erases `Fr`, `Pkt`, and `Mk` so that [`MixSimDriver`] only needs `Ts`.
/// Implemented by [`SimpleClient<Ts, Fr, Pkt, Mk>`] and any other concrete
/// client types.
///
/// [`MixSimDriver`]: crate::driver::MixSimDriver
pub trait MixSimClient<Ts: Clone + PartialOrd + Debug + Send>: Send {
    fn tick(&mut self, timestamp: Ts);
}
