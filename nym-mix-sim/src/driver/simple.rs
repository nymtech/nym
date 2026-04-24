// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::MixSimDriver,
    packet::{
        SimpleClientWrappingPipeline, SimpleClientUnwrapping, SimpleFrame, SimplePacket,
        SimplePassThroughPipeline,
    },
    topology::{TopologyClient, TopologyNode},
};

/// Concrete [`MixSimDriver`] instantiation that uses [`SimplePacket`]s and a
/// pass-through processing pipeline.
///
/// Each mix node runs a
/// [`SimplePassThroughPipeline`] that forwards packets unchanged to the next
/// node in the topology; each client uses a [`SimpleClientPipeline`] with no
/// Sphinx layering, reliability encoding, or obfuscation.
pub struct SimpleMixDriver(MixSimDriver<u32, SimpleFrame, SimplePacket>);

impl SimpleMixDriver {
    /// Load a topology JSON file and initialise the driver with simple pipelines.
    pub fn new(topology: String) -> anyhow::Result<Self> {
        let mixnode_pipeline =
            |top_node: &TopologyNode| SimplePassThroughPipeline::new(top_node.node_id);
        let client_processing_pipeline = |_: &TopologyClient| SimpleClientWrappingPipeline;

        let client_unwrapping_pipeline = |_: &TopologyClient| SimpleClientUnwrapping;

        let driver = MixSimDriver::<u32, SimpleFrame, SimplePacket>::new(
            topology,
            mixnode_pipeline,
            client_processing_pipeline,
            client_unwrapping_pipeline,
        )?;
        Ok(SimpleMixDriver(driver))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(manual_mode, tick_duration_ms).await
    }
}
