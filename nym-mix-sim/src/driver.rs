// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use anyhow::Context;
use tracing::debug;

use crate::{
    node::{Node, TopologyNode},
    topology::Directory,
};

pub struct MixSimDriver<Ts, Pkt> {
    nodes: Vec<Node<Ts, Pkt>>,
}

impl<Ts, Pkt> MixSimDriver<Ts, Pkt> {
    pub fn new(topology_file_path: String) -> anyhow::Result<Self> {
        debug!(
            "Bootstrapping nodes from topology file: {}",
            topology_file_path
        );
        // 1. Read topology from file
        let topology_data =
            std::fs::read_to_string(&topology_file_path).context("Failed to read topolgy file")?;
        let topology: Vec<TopologyNode> =
            serde_json::from_str(&topology_data).context("Topology file malformed")?;

        // 2. Init nodes
        let mut nodes = topology
            .into_iter()
            .map(|node| Node::<Ts, Pkt>::from_topology_node(node))
            .collect::<Vec<_>>();

        // 3. Build Directory
        let directory = Arc::new(Directory::build_from_nodes(&nodes));

        // 4. Give Directory to nodes
        for node in &mut nodes {
            node.set_directory(directory.clone());
        }

        Ok(Self { nodes })
    }
}
