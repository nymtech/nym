// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymTopologyNew;
pub use async_trait::async_trait;

// hehe, wasm
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait TopologyProvider: Send {
    async fn get_new_topology(&mut self) -> Option<NymTopologyNew>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait TopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopologyNew>;
}

pub struct HardcodedTopologyProvider {
    topology: NymTopologyNew,
}

impl HardcodedTopologyProvider {
    #[cfg(feature = "serde")]
    pub fn new_from_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        NymTopologyNew::new_from_file(path).map(Self::new)
    }

    pub fn new(topology: NymTopologyNew) -> Self {
        HardcodedTopologyProvider { topology }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for HardcodedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopologyNew> {
        Some(self.topology.clone())
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for HardcodedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopologyNew> {
        Some(self.topology.clone())
    }
}
