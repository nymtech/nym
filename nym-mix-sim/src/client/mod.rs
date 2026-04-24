// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Simulated mix-network client.
//!
//! A [`Client`] owns two UDP sockets:
//!
//! * **`mix_socket`** — sends packets to mix-network nodes and receives
//!   reply packets from them.
//! * **`app_socket`** — receives injection requests from user applications
//!   (e.g. the standalone `client` binary).  Not registered in the
//!   [`Directory`].
//!
//! ## Tick phases
//!
//! ```text
//! tick_app_incoming ──── app_socket ──▶ DynProcessingPipeline ──▶ outgoing_queue
//! tick_outgoing     ──── outgoing_queue ──▶ mix_socket ──▶ Node N
//! tick_mix_incoming ──── mix_socket ◀── Node N   (TODO: UnwrappingPipeline)
//! ```
//!
//! ## App-socket message format
//!
//! ```text
//! ┌─────────────────────┬─────────────────────┐
//! │  dst_node_id (1 B)  │  raw payload bytes  │
//! └─────────────────────┴─────────────────────┘
//! ```
//!
//! The first byte carries the destination [`NodeId`]; the rest is the
//! plaintext payload passed verbatim to [`DynProcessingPipeline::process`].

use std::{io::ErrorKind, net::UdpSocket, sync::Arc};

use nym_lp_data::{
    TimedData,
    clients::{traits::DynProcessingPipeline, types::StreamOptions},
};

use crate::{
    packet::WirePacketFormat,
    topology::{
        TopologyClient,
        directory::{Directory, NodeId},
    },
};

pub use crate::topology::ClientId;

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type (must match the driver's `Ts`).
/// `Pkt` is the packet type.
///
/// The frame type is fixed to `Vec<u8>` for the pipeline trait bounds, which
/// is sufficient for the current simulation.
pub struct Client<Ts, Fr, Pkt> {
    id: ClientId,

    /// Shared routing table, set via [`Client::set_directory`] after all
    /// node sockets are bound.
    directory: Arc<Directory>,

    /// Sends packets to mix-network nodes; also receives reply packets.
    mix_socket: UdpSocket,

    /// Receives injection requests from user applications.
    app_socket: UdpSocket,

    /// Outgoing packet queue, populated by [`tick_app_incoming`] and drained
    /// by [`tick_outgoing`].
    ///
    /// Each entry is `(first_hop_node_id, timed_packet)`.  The timestamp
    /// embedded in [`TimedData`] determines which tick the packet departs.
    ///
    /// [`tick_app_incoming`]: Client::tick_app_incoming
    /// [`tick_outgoing`]: Client::tick_outgoing
    outgoing_queue: Vec<(NodeId, TimedData<Ts, Pkt>)>,

    /// Outgoing pipeline: wraps plaintext application payloads into
    /// mix-network packets.
    ///
    /// Uses [`DynProcessingPipeline`] from `nym_lp_data::clients::traits`.
    /// The frame type `Fr` is erased to `Vec<u8>` for storage.
    processing_pipeline: Box<dyn DynProcessingPipeline<Ts, Fr, Pkt> + Send>,
    // TODO: unwrapping_pipeline: Box<dyn UnwrappingPipeline<Ts, Fr, Pkt, MessageKind = Vec<u8>> + Send>
    //       (nym_lp_data::mixnodes::traits::UnwrappingPipeline)
    //       Unwraps / decrypts packets received from the mix network and
    //       recovers the original plaintext.
}

impl<Ts, Fr, Pkt> Client<Ts, Fr, Pkt> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(
        topology: TopologyClient,
        processing_pipeline: impl DynProcessingPipeline<Ts, Fr, Pkt> + Send + 'static,
    ) -> anyhow::Result<Self> {
        let mix_socket = UdpSocket::bind(topology.mixnet_address)?;
        mix_socket.set_nonblocking(true)?;

        let app_socket = UdpSocket::bind(topology.app_address)?;
        app_socket.set_nonblocking(true)?;

        Ok(Self {
            id: topology.client_id,
            directory: Default::default(),
            mix_socket,
            app_socket,
            outgoing_queue: Vec::new(),
            processing_pipeline: Box::new(processing_pipeline),
        })
    }

    pub fn id(&self) -> ClientId {
        self.id
    }

    /// Attach the shared [`Directory`].  Must be called before the first tick.
    pub fn set_directory(&mut self, directory: Arc<Directory>) {
        self.directory = directory;
    }
}

impl<Ts, Fr, Pkt> Client<Ts, Fr, Pkt>
where
    Ts: Clone + PartialOrd,
    Pkt: WirePacketFormat,
{
    pub fn tick(&mut self, timestamp: Ts) {
        self.tick_app_incoming(timestamp.clone());
        self.tick_outgoing(timestamp.clone());
        self.tick_outgoing(timestamp);
    }
    /// **Phase 1 — app incoming**: drain the app socket, log each payload,
    /// run it through the [`DynProcessingPipeline`], and enqueue the
    /// resulting packets for sending this tick.
    ///
    /// Message format: `[dst_node_id: u8][payload bytes…]`
    pub fn tick_app_incoming(&mut self, timestamp: Ts) {
        let mut buf = [0u8; 1500];
        loop {
            let nb = match self.app_socket.recv(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    tracing::error!("[Client {}] app_socket recv error: {e}", self.id);
                    break;
                }
            };

            if nb < 2 {
                tracing::warn!(
                    "[Client {}] app message too short ({nb} bytes), dropping",
                    self.id
                );
                continue;
            }

            let dst: NodeId = buf[0];
            let payload = buf[1..nb].to_vec();

            tracing::info!(
                "[Client {}] App input: {} byte(s) → node {dst}",
                self.id,
                payload.len()
            );

            let packets = self.processing_pipeline.process(
                payload,
                StreamOptions::default(),
                timestamp.clone(),
            );

            for td in packets {
                self.outgoing_queue.push((dst, td));
            }
        }
    }

    /// **Phase 2 — outgoing**: send all queued packets whose scheduled
    /// timestamp is ≤ `timestamp` to their first-hop node via `mix_socket`.
    pub fn tick_outgoing(&mut self, timestamp: Ts) {
        let to_send = self
            .outgoing_queue
            .extract_if(.., |(_, td)| td.timestamp <= timestamp)
            .collect::<Vec<_>>();

        for (node_id, td) in to_send {
            if let Some(node) = self.directory.node(node_id) {
                if let Err(e) = self.mix_socket.send_to(&td.data.to_bytes(), node.addr) {
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
    }

    /// **Phase 3 — mix incoming**: drain the mix socket, log each received
    /// packet, and pass it through the unwrapping pipeline.
    ///
    /// The unwrapping pipeline is not yet implemented; see the TODO field in
    /// the struct.
    pub fn tick_mix_incoming(&mut self, _timestamp: Ts) {
        let mut buf = [0u8; 1500];
        loop {
            let (nb, src) = match self.mix_socket.recv_from(&mut buf) {
                Ok(r) => r,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    tracing::error!("[Client {}] mix_socket recv error: {e}", self.id);
                    break;
                }
            };

            match Pkt::try_from_bytes(&buf[..nb]) {
                Ok(pkt) => {
                    tracing::info!(
                        "[Client {}] Received {nb} byte(s) from mix node {src}: {pkt:?}",
                        self.id
                    );
                    // TODO: pass pkt through unwrapping_pipeline, then log the
                    // recovered plaintext:
                    //   if let Some(content) = self.unwrapping_pipeline.process(pkt, timestamp) {
                    //       tracing::info!("[Client {}] Unwrapped: {:?}", self.id, content);
                    //   }
                }
                Err(e) => {
                    tracing::error!(
                        "[Client {}] Failed to deserialize mix packet from {src}: {e}",
                        self.id
                    );
                }
            }
        }
    }
}
