// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{NymTopology, NymTopologyMetadata};
pub use async_trait::async_trait;
use nym_api_requests::nym_nodes::NodesResponseMetadata;

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
    #[cfg(feature = "persistence")]
    pub fn new_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        NymTopology::new_from_file(path).map(Self::new)
    }

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

// helper trait to convert between nym-api response and the topology metadata
// (we don't want to be importing any of those in the other crates)
pub trait ToTopologyMetadata {
    fn to_topology_metadata(&self) -> NymTopologyMetadata;
}

impl ToTopologyMetadata for NodesResponseMetadata {
    fn to_topology_metadata(&self) -> NymTopologyMetadata {
        NymTopologyMetadata::new(self.rotation_id, self.absolute_epoch_id, self.refreshed_at)
    }
}
