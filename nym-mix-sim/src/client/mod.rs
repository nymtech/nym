// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, io::ErrorKind, net::UdpSocket, sync::Arc};

use nym_lp_data::{TimedData, clients::types::StreamOptions};

use crate::{
    node::NodeId,
    packet::WirePacketFormat,
    topology::{TopologyClient, directory::Directory},
};

pub mod simple;
pub mod sphinx;

/// Compact identifier for a simulated client.
pub type ClientId = NodeId;

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

pub trait ProcessingClient<Ts, SndPkt, RcvPkt = SndPkt>: Send {
    fn process(
        &mut self,
        input: Vec<u8>,
        processing_options: StreamOptions,
        timestamp: Ts,
    ) -> Vec<TimedData<Ts, SndPkt>>;

    fn unwrap(&mut self, input: RcvPkt, timestamp: Ts) -> anyhow::Result<Option<Vec<u8>>>;
}

/// Shared UDP transport layer for simulated clients.
///
/// Encapsulates both sockets, the routing directory, and the client id so that
/// multiple concrete client types can reuse `send_to_node`, `recv_from_mix`,
/// and `recv_from_app` without duplicating that logic.  Packet types are
/// method-level generics so `BaseClient` itself has no type parameters.
pub struct BaseClient<Ts, Pc, SndPkt, RcvPkt = SndPkt> {
    id: ClientId,
    mix_socket: UdpSocket,
    app_socket: UdpSocket,
    directory: Arc<Directory>,

    outgoing_queue: Vec<(NodeId, TimedData<Ts, SndPkt>)>,

    processing_client: Pc,

    _marker: std::marker::PhantomData<RcvPkt>,
}

impl<Ts, Pc, SndPkt, RcvPkt> BaseClient<Ts, Pc, SndPkt, RcvPkt> {
    /// Bind both UDP sockets to the given addresses.
    pub(crate) fn with_pipeline(
        topology_client: &TopologyClient,
        directory: Arc<Directory>,
        processing_client: Pc,
    ) -> anyhow::Result<Self> {
        let mix_socket = UdpSocket::bind(topology_client.mixnet_address)?;
        mix_socket.set_nonblocking(true)?;

        let app_socket = UdpSocket::bind(topology_client.app_address)?;
        app_socket.set_nonblocking(true)?;

        Ok(Self {
            id: topology_client.client_id,
            mix_socket,
            app_socket,
            directory,
            outgoing_queue: Vec::new(),
            processing_client,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<Ts, Pc, SndPkt, RcvPkt> BaseClient<Ts, Pc, SndPkt, RcvPkt>
where
    SndPkt: WirePacketFormat,
    RcvPkt: WirePacketFormat,
{
    /// Send `packet` to the mix node identified by `node_id` via `mix_socket`.
    ///
    /// Resolves `node_id` against the shared [`Directory`], serialises via
    /// [`WirePacketFormat::to_bytes`], and dispatches with a single `sendto`.
    /// Errors are logged but not propagated.
    pub fn send_to_node(&self, node_id: NodeId, packet: SndPkt) {
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
    pub fn recv_from_mix(&self) -> Option<anyhow::Result<RcvPkt>> {
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
        Some(RcvPkt::try_from_bytes(&buf[..nb]))
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

impl<Ts, Pc, SndPkt, RcvPkt> MixSimClient<Ts> for BaseClient<Ts, Pc, SndPkt, RcvPkt>
where
    Ts: Clone + PartialOrd + Debug + Send,
    SndPkt: WirePacketFormat + Debug + Send,
    RcvPkt: WirePacketFormat + Debug + Send,
    Pc: ProcessingClient<Ts, SndPkt, RcvPkt>,
{
    fn tick(&mut self, timestamp: Ts) {
        self.tick_app_incoming(timestamp.clone());
        self.tick_outgoing(timestamp.clone());
        self.tick_mix_incoming(timestamp);
    }
}

impl<Ts, Pc, SndPkt, RcvPkt> BaseClient<Ts, Pc, SndPkt, RcvPkt>
where
    Ts: Clone + PartialOrd + Debug + Send,
    SndPkt: WirePacketFormat + Debug + Send,
    RcvPkt: WirePacketFormat + Debug + Send,
    Pc: ProcessingClient<Ts, SndPkt, RcvPkt>,
{
    /// **Phase 1 — app incoming**: drain the app socket, run each payload
    /// through the processing pipeline, and enqueue the resulting packets.
    fn tick_app_incoming(&mut self, timestamp: Ts) {
        while let Some(result) = self.recv_from_app() {
            let bytes = match result {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("[Client {}] app_socket recv error: {e}", self.id);
                    continue;
                }
            };

            // SW This bit should be in simple client probably,
            if bytes.len() < 2 {
                tracing::warn!(
                    "[Client {}] app message too short ({} bytes), dropping",
                    self.id,
                    bytes.len()
                );
                continue;
            }

            let _dst: NodeId = bytes[0];
            let payload = bytes[1..].to_vec();

            tracing::info!(
                "[Client {}] App input: {} byte(s) → client {_dst}",
                self.id,
                payload.len()
            );

            // SW actually, pipeline should tell me where to send it next
            let packets = self.processing_client.process(
                payload,
                StreamOptions::default(),
                timestamp.clone(),
            );

            for td in packets {
                self.outgoing_queue.push((0, td));
            }
        }
    }

    /// **Phase 2 — outgoing**: send all queued packets whose scheduled
    /// timestamp is ≤ `timestamp` to their first-hop node.
    fn tick_outgoing(&mut self, timestamp: Ts) {
        let to_send = self
            .outgoing_queue
            .extract_if(.., |(_, td)| td.timestamp <= timestamp)
            .map(|(node_id, td)| (node_id, td.data))
            .collect::<Vec<_>>();
        for (node_id, pkt) in to_send {
            self.send_to_node(node_id, pkt);
        }
    }

    /// **Phase 3 — mix incoming**: drain the mix socket and pass each packet
    /// through the unwrapping pipeline.
    fn tick_mix_incoming(&mut self, timestamp: Ts) {
        while let Some(result) = self.recv_from_mix() {
            match result {
                Ok(pkt) => match self.processing_client.unwrap(pkt, timestamp.clone()) {
                    Ok(Some(content)) => {
                        tracing::info!(
                            "[Client {}] Received: {:?}",
                            self.id,
                            String::from_utf8_lossy(&content)
                        );
                    }
                    Err(e) => {
                        tracing::error!("[Client {}] Error unwrapping packet : {e}", self.id);
                    }
                    Ok(None) => {}
                },
                Err(e) => {
                    tracing::error!("[Client {}] Failed to deserialize mix packet: {e}", self.id);
                }
            }
        }
    }
}
