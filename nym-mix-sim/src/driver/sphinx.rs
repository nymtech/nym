// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sphinx-based driver variants.
//!
//! Two flavours are provided:
//!
//! * [`SphinxMixDriver`] — wall-clock ([`Instant`]) timestamps; automatic mode only.
//! * [`DiscreteSphinxMixDriver`] — discrete `u32` tick counter (1 tick = 1 ms);
//!   supports both automatic and manual stepping modes.

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
    /// Load a topology JSON file and initialise the driver with Sphinx pipelines.
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
            let client = SphinxClient::new(top_client, directory.clone(), Instant::now())?;
            clients.push(Box::new(client));
        }

        Ok(SphinxMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    ///
    /// `manual_mode` is ignored: [`Instant`]-based drivers cannot be stepped
    /// manually because wall-clock time cannot be advanced by keypress.
    pub async fn run(self, _manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(false, tick_duration_ms).await
    }
}

/// Concrete [`MixSimDriver`] instantiation that uses full Sphinx encryption with a
/// discrete tick counter.
///
/// Each tick corresponds to 1 ms of simulated time, enabling deterministic
/// stepping and delay arithmetic without requiring wall-clock time.  This is
/// the default driver and the only Sphinx variant that supports manual mode.
pub struct DiscreteSphinxMixDriver(MixSimDriver<u32>);

impl DiscreteSphinxMixDriver {
    const START_TICK: u32 = 0;

    /// Load a topology JSON file and initialise the driver with Sphinx pipelines.
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
            let client = SphinxClient::new(top_client, directory.clone(), Self::START_TICK)?;
            clients.push(Box::new(client));
        }

        Ok(DiscreteSphinxMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(self, manual_mode: bool, display_state: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0
            .run(manual_mode, display_state, Self::START_TICK, tick_duration_ms)
            .await
    }
}
