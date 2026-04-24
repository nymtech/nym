// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    packet::{SimplePacket, WirePacketFormat},
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

    packet_buffer: Vec<Pkt>,
    _ts_marker: std::marker::PhantomData<Ts>,
}

impl<Ts, Pkt> Node<Ts, Pkt> {
    pub fn from_topology_node(node: TopologyNode) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(node.addr)?;
        Ok(Node {
            directory: Default::default(),
            details: node,
            socket,
            packet_buffer: Vec::new(),
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

impl<Ts, Pkt> Node<Ts, Pkt>
where
    Pkt: WirePacketFormat,
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
        let (nb_bytes, src_address) = self
            .socket
            .recv_from(&mut buf)
            .inspect_err(|e| tracing::error!("Error receiving packet : {e}"))
            .ok()?;

        tracing::info!(
            "[Node {}] Received {nb_bytes} bytes from {src_address}",
            self.details.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb_bytes]))
    }
}

impl<Pkt> Node<u32, Pkt>
where
    Pkt: WirePacketFormat,
{
    pub fn tick_incoming(&mut self, _: u32) {
        while let Some(maybe_packet) = self.recv_packet() {
            match maybe_packet {
                Ok(packet) => self.packet_buffer.push(packet),
                Err(e) => tracing::error!(
                    "[Node {}] Failed to deserialize packet : {e}",
                    self.details.id
                ),
            }
        }
    }
    pub fn tick_outgoing(&mut self, _: u32) {
        while let Some(packet) = self.packet_buffer.pop() {
            self.send_to_node(self.id() + 1, packet);
        }
    }
}
