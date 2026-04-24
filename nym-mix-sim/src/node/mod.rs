// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    packet::WirePacketFormat,
    topology::{Directory, DirectoryNode},
};

pub type NodeId = u8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: NodeId,
    pub reliability: u8,
    pub addr: SocketAddr,
}

impl TopologyNode {
    pub fn new(id: NodeId, reliability: u8, addr: SocketAddr) -> Self {
        Self {
            id,
            reliability,
            addr,
        }
    }
}

pub struct Node<Ts, Pkt> {
    directory: Arc<Directory>,
    details: TopologyNode,
    socket: UdpSocket,

    // Internal Buffers
    packets_to_process: Vec<Pkt>,
    processed_packets: Vec<Pkt>,

    _ts_marker: std::marker::PhantomData<Ts>,
}

impl<Ts, Pkt> Node<Ts, Pkt> {
    pub fn from_topology_node(node: TopologyNode) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(node.addr)?;
        socket.set_nonblocking(true)?;
        Ok(Node {
            directory: Default::default(),
            details: node,
            socket,
            packets_to_process: Vec::new(),
            processed_packets: Vec::new(),
            _ts_marker: std::marker::PhantomData,
        })
    }

    pub fn id(&self) -> NodeId {
        self.details.id
    }

    pub fn set_directory(&mut self, directory: Arc<Directory>) {
        self.directory = directory
    }

    pub fn get_directory_node(&self) -> DirectoryNode {
        DirectoryNode {
            node_detail: self.details.clone(),
            addr: self.details.addr,
        }
    }
}

impl<Ts: Debug, Pkt: Debug> Node<Ts, Pkt> {
    pub fn display_state(&self) {
        println!("│  Node {:2} @ {}", self.details.id, self.details.addr);
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
    Ts: Clone,
    Pkt: WirePacketFormat<Ts>,
{
    pub fn send_to_node(&self, node_id: NodeId, packet: Pkt) {
        if let Some(node) = self.directory.node(node_id) {
            if let Err(e) = self.socket.send_to(&packet.to_bytes(), node.addr) {
                tracing::error!(
                    "[Node {}] Failed to send data to node {node_id} : {e}",
                    self.details.id
                );
            } else {
                tracing::info!(
                    "[Node {}] Successfully sent a packet to {node_id}",
                    self.details.id
                );
            }
        } else {
            tracing::error!(
                "[Node {}] Trying to send to non-existing node {node_id}",
                self.details.id
            );
        }
    }

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
            self.details.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb_bytes]))
    }

    pub fn tick_incoming(&mut self, _: Ts) {
        while let Some(maybe_packet) = self.recv_packet() {
            match maybe_packet {
                Ok(packet) => self.packets_to_process.push(packet),
                Err(e) => tracing::error!(
                    "[Node {}] Failed to deserialize packet : {e}",
                    self.details.id
                ),
            }
        }
    }

    pub fn tick_processing(&mut self, timestamp: Ts) {
        while let Some(packet) = self.packets_to_process.pop() {
            match packet.process(timestamp.clone()) {
                Ok(packet) => self.processed_packets.push(packet),
                Err(e) => {
                    tracing::error!("[Node {}] Failed to process packet : {e}", self.details.id)
                }
            }
        }
    }

    pub fn tick_outgoing(&mut self, _: Ts) {
        while let Some(packet) = self.processed_packets.pop() {
            self.send_to_node(self.id() + 1, packet);
        }
    }
}
