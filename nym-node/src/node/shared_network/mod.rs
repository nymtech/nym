// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_topology::{EpochRewardedSet, NymTopology, NymTopologyMetadata};
use nym_validator_client::nym_nodes::SemiSkimmedNodeV1;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) mod refresher;
pub(crate) mod topology_provider;

#[derive(Clone)]
pub(crate) struct CachedNetwork {
    inner: Arc<RwLock<CachedNetworkInner>>,
}

impl CachedNetwork {
    fn new_empty() -> Self {
        CachedNetwork {
            inner: Arc::new(RwLock::new(CachedNetworkInner {
                rewarded_set: Default::default(),
                topology_metadata: Default::default(),
                network_nodes: vec![],
            })),
        }
    }

    async fn network_topology(&self, min_mix_performance: u8) -> NymTopology {
        let network_guard = self.inner.read().await;

        NymTopology::new(
            network_guard.topology_metadata,
            network_guard.rewarded_set.clone(),
            Vec::new(),
        )
        .with_additional_nodes(
            network_guard
                .network_nodes
                .iter()
                .map(|node| &node.basic)
                .filter(|node| {
                    if node.supported_roles.mixnode {
                        node.performance.round_to_integer() >= min_mix_performance
                    } else {
                        true
                    }
                }),
        )
    }
}

struct CachedNetworkInner {
    rewarded_set: EpochRewardedSet,
    topology_metadata: NymTopologyMetadata,
    network_nodes: Vec<SemiSkimmedNodeV1>,
}
