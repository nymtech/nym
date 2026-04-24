// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, mpsc};

use serde::{Deserialize, Serialize};

use crate::topology::{Directory, DirectoryNode};

pub type NodeId = u8;
pub type NodeInputSender<Pkt> = mpsc::Sender<Pkt>;
pub type NodeInput<Pkt> = mpsc::Receiver<Pkt>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: NodeId,
    pub reliability: u8,
}

impl TopologyNode {
    pub fn new(id: NodeId, reliability: u8) -> Self {
        Self { id, reliability }
    }
}
pub struct Node<Ts, Pkt> {
    directory: Arc<Directory<Pkt>>,
    topology_node: TopologyNode,
    input: NodeInput<Pkt>,

    input_sender: NodeInputSender<Pkt>,
    _ts_marker: std::marker::PhantomData<Ts>,
}

impl<Ts, Pkt> Node<Ts, Pkt> {
    pub fn from_topology_node(node: TopologyNode) -> Self {
        let (input_sender, input) = mpsc::channel();
        Node {
            directory: Default::default(),
            topology_node: node,
            input,
            input_sender,
            _ts_marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> NodeId {
        self.topology_node.id
    }

    pub fn set_directory(&mut self, directory: Arc<Directory<Pkt>>) {
        self.directory = directory
    }

    pub fn get_directory_node(&self) -> DirectoryNode<Pkt> {
        DirectoryNode {
            node_detail: self.topology_node.clone(),
            input_channel: self.input_sender.clone(),
        }
    }
}
