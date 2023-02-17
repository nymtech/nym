// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymTopology;
use async_trait::async_trait;

// hehe, wasm
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait TopologyProvider: Send {
    async fn get_new_topology(&mut self) -> Option<NymTopology>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait TopologyProvider {
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

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for HardcodedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        Some(self.topology.clone())
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for HardcodedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        Some(self.topology.clone())
    }
}
