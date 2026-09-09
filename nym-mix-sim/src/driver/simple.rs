// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimpleMixDriver`] — concrete driver using the simple (non-Sphinx) packet pipeline.

use std::sync::Arc;

use crate::{
    client::{MixSimClient, simple::SimpleClient},
    driver::MixSimDriver,
    node::{MixSimNode, simple::SimpleNode},
    sim::env::SimEnv,
    topology::{Topology, directory::Directory},
};

/// Concrete [`MixSimDriver`] instantiation that uses
/// [`SimplePacket`](crate::packet::simple::SimplePacket)s and a pass-through
/// processing pipeline.
///
/// Each mix node runs a [`SimpleProcessingNode`] that forwards packets
/// unchanged to the next node in the topology; each client uses a
/// [`SimpleClientWrappingPipeline`] with no Sphinx layering, reliability
/// encoding, or obfuscation.
///
/// [`SimpleProcessingNode`]: crate::node::simple::SimpleProcessingNode
/// [`SimpleClientWrappingPipeline`]: crate::client::simple::SimpleClientWrappingPipeline
pub struct SimpleMixDriver(MixSimDriver);

impl SimpleMixDriver {
    /// Build the driver over whatever world `env` provides, given a Topology
    pub fn new(topology: Topology, env: &mut dyn SimEnv) -> anyhow::Result<Self> {
        let directory: Arc<Directory> = Arc::new((&topology).into());

        let mut nodes: Vec<Box<dyn MixSimNode + Send>> = Vec::with_capacity(topology.nodes.len());
        for top_node in topology.nodes {
            let node = SimpleNode::new(top_node, directory.clone(), env)?;
            nodes.push(Box::new(node));
        }

        let mut clients: Vec<Box<dyn MixSimClient + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SimpleClient::new(top_client, directory.clone(), env)?;
            clients.push(Box::new(client));
        }

        Ok(SimpleMixDriver(MixSimDriver::new(nodes, clients)))
    }

    pub fn into_inner(self) -> MixSimDriver {
        self.0
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(
        self,
        manual_mode: bool,
        display_state: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        self.0
            .run(manual_mode, display_state, tick_duration_ms)
            .await
    }
}
