// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::node::{Node, NodeId, NodeInputSender, TopologyNode};

pub struct Directory<Pkt> {
    nodes: HashMap<NodeId, DirectoryNode<Pkt>>,
}

impl<Pkt> Default for Directory<Pkt> {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
}

impl<Pkt> Directory<Pkt> {
    pub fn build_from_nodes<Ts>(node_list: &Vec<Node<Ts, Pkt>>) -> Self {
        let mut nodes = HashMap::new();
        for node in node_list {
            nodes.insert(node.id(), node.get_directory_node());
        }
        Self { nodes }
    }

    pub fn node(&self, id: NodeId) -> Option<&DirectoryNode<Pkt>> {
        self.nodes.get(&id)
    }
}

pub struct DirectoryNode<Pkt> {
    pub node_detail: TopologyNode,
    pub input_channel: NodeInputSender<Pkt>,
}
