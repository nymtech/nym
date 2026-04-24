// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Instant};

use anyhow::Context;

use crate::{
    client::{MixSimClient, sphinx::SphinxClient},
    driver::MixSimDriver,
    node::{MixSimNode, sphinx::SphinxNode},
    topology::{Topology, directory::Directory},
};

/// Concrete [`MixSimDriver`] instantiation that uses [`SphinxPacket`]s.
pub struct SphinxMixDriver(MixSimDriver<Instant>);

impl SphinxMixDriver {
    /// Load a topology JSON file and initialise the driver with simple pipelines.
    pub fn new(topology: String) -> anyhow::Result<Self> {
        let topology_data =
            std::fs::read_to_string(&topology).context("Failed to read topology file")?;
        let topology: Topology =
            serde_json::from_str(&topology_data).context("Topology file malformed")?;

        let directory: Arc<Directory> = Arc::new((&topology).into());

        let mut nodes: Vec<Box<dyn MixSimNode<Instant> + Send>> =
            Vec::with_capacity(topology.nodes.len());
        for top_node in topology.nodes {
            let node = SphinxNode::new(top_node, directory.clone())?;
            nodes.push(Box::new(node));
        }

        let mut clients: Vec<Box<dyn MixSimClient<Instant> + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SphinxClient::new(top_client, directory.clone())?;
            clients.push(Box::new(client));
        }

        Ok(SphinxMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(self, _manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(false, tick_duration_ms).await
    }
}

/// Concrete [`MixSimDriver`] instantiation that uses [`SphinxPacket`]s. Use this for manual mode, one tick is one ms
pub struct DiscreteSphinxMixDriver(MixSimDriver<u32>);

impl DiscreteSphinxMixDriver {
    /// Load a topology JSON file and initialise the driver with simple pipelines.
    pub fn new(topology: String) -> anyhow::Result<Self> {
        let topology_data =
            std::fs::read_to_string(&topology).context("Failed to read topology file")?;
        let topology: Topology =
            serde_json::from_str(&topology_data).context("Topology file malformed")?;

        let directory: Arc<Directory> = Arc::new((&topology).into());

        let mut nodes: Vec<Box<dyn MixSimNode<u32> + Send>> =
            Vec::with_capacity(topology.nodes.len());
        for top_node in topology.nodes {
            let node = SphinxNode::new(top_node, directory.clone())?;
            nodes.push(Box::new(node));
        }

        let mut clients: Vec<Box<dyn MixSimClient<u32> + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SphinxClient::new(top_client, directory.clone())?;
            clients.push(Box::new(client));
        }

        Ok(DiscreteSphinxMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(manual_mode, tick_duration_ms).await
    }
}
