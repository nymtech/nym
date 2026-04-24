// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Individual mix-node model.
//!
//! A [`SimpleNode`] represents one mix node in the simulated network.  Each
//! node owns a [`NodeSocket`] (which manages the UDP socket and routing
//! directory) and two internal packet buffers:
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

use std::{fmt::Debug, sync::Arc};

use nym_lp_data::{TimedData, mixnodes::traits::DynMixnodeProcessingPipeline};

use crate::{
    node::{BaseNode, MixSimNode, NodeId},
    packet::{SimpleMixnodePipeline, SimplePacket},
    topology::{TopologyNode, directory::Directory},
};

/// A running mix-node instance inside the simulation.
///
/// `Ts` is the timestamp / tick-context type; `Fr` is the intermediate frame
/// type; `Pkt` is the transport packet type; `Mk` is the message marker type.
///
/// UDP transport and routing are handled by the embedded [`NodeSocket`]; this
/// struct adds the packet buffers and the mix-processing pipeline on top.
pub struct SimpleNode<Ts> {
    pub(crate) socket: BaseNode,

    packets_to_process: Vec<TimedData<Ts, SimplePacket>>,
    processed_packets: Vec<(NodeId, TimedData<Ts, SimplePacket>)>,
    processing_pipeline: SimpleMixnodePipeline,
}

impl<Ts> SimpleNode<Ts> {
    /// Create a [`SimpleNode`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.socket_address`.
    ///
    /// The [`Directory`] is initialised to its default (empty) value and must
    /// be set later with [`MixSimNode::set_directory`] before the node attempts
    /// to send any packets.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        Ok(SimpleNode {
            socket: BaseNode::new(topology_node.clone(), directory)?,
            packets_to_process: Vec::new(),
            processed_packets: Vec::new(),
            processing_pipeline: SimpleMixnodePipeline::new(topology_node.node_id),
        })
    }
}

impl<Ts> MixSimNode<Ts> for SimpleNode<Ts>
where
    Ts: Clone + PartialOrd + Debug + Send,
{
    fn tick_incoming(&mut self, timestamp: Ts) {
        while let Some(maybe_packet) = self.socket.recv_packet() {
            match maybe_packet {
                Ok(packet) => self
                    .packets_to_process
                    .push(TimedData::new(timestamp.clone(), packet)),
                Err(e) => tracing::error!(
                    "[Node {}] Failed to deserialize packet : {e}",
                    self.socket.id
                ),
            }
        }
    }

    fn tick_processing(&mut self, timestamp: Ts) {
        while let Some(packet) = self.packets_to_process.pop() {
            match self.processing_pipeline.process(packet, timestamp.clone()) {
                Ok(processed_packets) => self.processed_packets.extend(processed_packets),
                Err(e) => {
                    tracing::error!("[Node {}] Failed to process packet : {e}", self.socket.id)
                }
            }
        }
    }

    fn tick_outgoing(&mut self, timestamp: Ts) {
        let to_send = self
            .processed_packets
            .extract_if(.., |(_, pkt)| pkt.timestamp <= timestamp)
            .map(|(next_hop, pkt)| (next_hop, pkt.data))
            .collect::<Vec<_>>();
        for (next_hop, pkt) in to_send {
            self.socket.send_to_node(next_hop, pkt);
        }
    }

    fn display_state(&self) {
        println!(
            "│  Node {:2} @ {}",
            self.socket.id, self.socket.socket_address
        );
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
