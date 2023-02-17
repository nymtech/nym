// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymTopology;
use async_trait::async_trait;

#[async_trait]
pub trait TopologyProvider: Send {
    async fn get_new_topology(&mut self) -> Option<NymTopology>;
}

pub struct HardcodedTopologyProvider {
    topology: NymTopology,
}

impl HardcodedTopologyProvider {
    pub fn new(topology: NymTopology) -> Self {
        HardcodedTopologyProvider { topology }
    }
}

#[async_trait]
impl TopologyProvider for HardcodedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        Some(self.topology.clone())
    }
}
