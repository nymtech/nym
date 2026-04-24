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

use std::fmt::Debug;
use std::sync::Arc;

use nym_lp_data::{
    TimedData,
    clients::{
        traits::{ClientUnwrappingPipeline, ClientWrappingPipeline},
        types::StreamOptions,
    },
};

use crate::{
    client::{BaseClient, MixSimClient},
    node::NodeId,
    packet::{SimpleClientUnwrapping, SimpleClientWrappingPipeline, SimplePacket},
    topology::{TopologyClient, directory::Directory},
};

/// A simulated client that injects packets into the mix network.
///
/// `Ts` is the timestamp / tick-context type.  Packet type, frame type, and
/// message marker are fixed to the `Simple*` concrete types.
///
/// UDP transport and routing are handled by the embedded [`BaseClient`]; this
/// struct adds the outgoing queue and the wrapping/unwrapping pipelines.
pub struct SimpleClient<Ts> {
    pub(crate) socket: BaseClient,
    outgoing_queue: Vec<(NodeId, TimedData<Ts, SimplePacket>)>,
    processing_pipeline: SimpleClientWrappingPipeline,
    unwrapping_pipeline: SimpleClientUnwrapping,
}

impl<Ts> SimpleClient<Ts> {
    /// Bind both UDP sockets and return a new client.
    ///
    /// # Errors
    ///
    /// Returns an error if either socket fails to bind or set non-blocking.
    pub fn new(topology: TopologyClient, directory: Arc<Directory>) -> anyhow::Result<Self> {
        Ok(Self {
            socket: BaseClient::new(topology, directory)?,
            outgoing_queue: Vec::new(),
            processing_pipeline: SimpleClientWrappingPipeline::default(),
            unwrapping_pipeline: SimpleClientUnwrapping::default(),
        })
    }
}

impl<Ts> MixSimClient<Ts> for SimpleClient<Ts>
where
    Ts: Clone + PartialOrd + Debug + Send,
{
    fn tick(&mut self, timestamp: Ts) {
        self.tick_app_incoming(timestamp.clone());
        self.tick_outgoing(timestamp.clone());
        self.tick_mix_incoming(timestamp);
    }
}

impl<Ts> SimpleClient<Ts>
where
    Ts: Clone + PartialOrd + Debug + Send,
{
    /// **Phase 1 — app incoming**: drain the app socket, run each payload
    /// through the processing pipeline, and enqueue the resulting packets.
    fn tick_app_incoming(&mut self, timestamp: Ts) {
        while let Some(result) = self.socket.recv_from_app() {
            let bytes = match result {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("[Client {}] app_socket recv error: {e}", self.socket.id);
                    continue;
                }
            };

            if bytes.len() < 2 {
                tracing::warn!(
                    "[Client {}] app message too short ({} bytes), dropping",
                    self.socket.id,
                    bytes.len()
                );
                continue;
            }

            let _dst: NodeId = bytes[0];
            let payload = bytes[1..].to_vec();

            tracing::info!(
                "[Client {}] App input: {} byte(s) → client {_dst}",
                self.socket.id,
                payload.len()
            );

            let packets = self.processing_pipeline.process(
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
            self.socket.send_to_node(node_id, pkt);
        }
    }

    /// **Phase 3 — mix incoming**: drain the mix socket and pass each packet
    /// through the unwrapping pipeline.
    fn tick_mix_incoming(&mut self, timestamp: Ts) {
        while let Some(result) = self.socket.recv_from_mix() {
            match result {
                Ok(pkt) => match self.unwrapping_pipeline.unwrap(pkt, timestamp.clone()) {
                    Ok(Some(content)) => {
                        tracing::info!(
                            "[Client {}] Received: {:?}",
                            self.socket.id,
                            String::from_utf8_lossy(&content)
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "[Client {}] Error unwrapping packet : {e}",
                            self.socket.id
                        );
                    }
                    Ok(None) => {}
                },
                Err(e) => {
                    tracing::error!(
                        "[Client {}] Failed to deserialize mix packet: {e}",
                        self.socket.id
                    );
                }
            }
        }
    }
}
