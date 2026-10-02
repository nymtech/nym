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

use rand::rngs::OsRng;

use crate::{
    client::{MixSimClient, sphinx::SphinxClient},
    driver::MixSimDriver,
    node::{MixSimNode, sphinx::SphinxNode},
    topology::{Topology, directory::Directory},
};

/// Concrete [`MixSimDriver`] instantiation that uses [`SphinxPacket`](nym_sphinx::SphinxPacket)s.
pub struct SphinxMixDriver(MixSimDriver);

impl SphinxMixDriver {
    /// Load a topology JSON file and initialise the driver with Sphinx pipelines.
    pub fn new(topology: String) -> anyhow::Result<Self> {
        let topology = Topology::load(&topology)?;

        let directory: Arc<Directory> = Arc::new((&topology).into());

        let mut nodes: Vec<Box<dyn MixSimNode + Send>> = Vec::with_capacity(topology.nodes.len());
        for top_node in topology.nodes {
            let node = SphinxNode::new(top_node, directory.clone())?;
            nodes.push(Box::new(node));
        }

        let mut clients: Vec<Box<dyn MixSimClient + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SphinxClient::new(top_client, directory.clone(), Instant::now(), OsRng)?;
            clients.push(Box::new(client));
        }

        Ok(SphinxMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    ///
    /// `manual_mode` is ignored: [`Instant`]-based drivers cannot be stepped
    /// manually because wall-clock time cannot be advanced by keypress.
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
