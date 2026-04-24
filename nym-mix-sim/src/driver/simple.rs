// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimpleMixDriver`] — concrete driver using the simple (non-Sphinx) packet pipeline.

use std::sync::Arc;

use anyhow::Context;

use crate::{
    client::{MixSimClient, simple::SimpleClient},
    driver::MixSimDriver,
    node::{MixSimNode, simple::SimpleNode},
    topology::{Topology, directory::Directory},
};

/// Concrete [`MixSimDriver`] instantiation that uses [`SimplePacket`]s and a
/// pass-through processing pipeline.
///
/// Each mix node runs a [`SimpleMixnodePipeline`] that forwards packets
/// unchanged to the next node in the topology; each client uses a
/// [`SimpleClientWrappingPipeline`] with no Sphinx layering, reliability
/// encoding, or obfuscation.
pub struct SimpleMixDriver(MixSimDriver<u32>);

impl SimpleMixDriver {
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
            let node = SimpleNode::new(top_node, directory.clone())?;
            nodes.push(Box::new(node));
        }

        let mut clients: Vec<Box<dyn MixSimClient<u32> + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SimpleClient::new(top_client, directory.clone())?;
            clients.push(Box::new(client));
        }

        Ok(SimpleMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(self, manual_mode: bool, display_state: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(manual_mode, display_state, 0, tick_duration_ms).await
    }
}
