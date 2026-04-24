// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, net::SocketAddr};

use crate::node::{Node, NodeId, TopologyNode};

#[derive(Default, Debug)]
pub struct Directory {
    nodes: HashMap<NodeId, DirectoryNode>,
}

impl Directory {
    pub fn build_from_nodes<Ts, Pkt>(node_list: &Vec<Node<Ts, Pkt>>) -> Self {
        let mut nodes = HashMap::new();
        for node in node_list {
            nodes.insert(node.id(), node.get_directory_node());
        }
        Self { nodes }
    }

    pub fn node(&self, id: NodeId) -> Option<&DirectoryNode> {
        self.nodes.get(&id)
    }
}

#[derive(Debug)]
pub struct DirectoryNode {
    pub node_detail: TopologyNode,
    pub addr: SocketAddr,
}
