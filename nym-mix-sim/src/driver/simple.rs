// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    driver::MixSimDriver,
    packet::{SimpleClientPipeline, SimpleFrame, SimplePacket, SimplePassThroughPipeline},
    topology::{TopologyClient, TopologyNode},
};

// Driver using `SimplePacket`s
pub struct SimpleMixDriver(MixSimDriver<u32, SimpleFrame, SimplePacket>);

impl SimpleMixDriver {
    pub fn new(topology: String) -> anyhow::Result<Self> {
        let mixnode_pipeline =
            |top_node: &TopologyNode| SimplePassThroughPipeline::new(top_node.node_id);
        let client_processing_pipeline = |_: &TopologyClient| SimpleClientPipeline;

        let driver = MixSimDriver::<u32, SimpleFrame, SimplePacket>::new(
            topology,
            mixnode_pipeline,
            client_processing_pipeline,
        )?;
        Ok(SimpleMixDriver(driver))
    }

    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        self.0.run(manual_mode, tick_duration_ms).await
    }
}
