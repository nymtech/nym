// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_noise::config::NoiseNetworkView;
use nym_topology::{EpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata, RoutingNode};
use nym_validator_client::nym_nodes::SemiSkimmedNodeV3;
use std::sync::Arc;

use crate::node::{lp::directory::LpNodes, routing_filter::network_filter::NetworkRoutingFilter};

pub(crate) mod refresher;
pub(crate) mod topology_provider;

#[derive(Clone)]
pub(crate) struct CachedNetwork {
    topology_builder: Arc<ArcSwap<NymTopologyBuilder>>,

    routing_filter: NetworkRoutingFilter,
    noise_view: NoiseNetworkView,
    lp_nodes: LpNodes,
    full_topology: CachedFullTopology,
}

impl CachedNetwork {
    fn new_empty(
        routing_filter: NetworkRoutingFilter,
        noise_view: NoiseNetworkView,
        lp_nodes: LpNodes,
    ) -> Self {
        CachedNetwork {
            topology_builder: Arc::new(ArcSwap::from_pointee(NymTopologyBuilder::default())),
            routing_filter,
            noise_view,
            lp_nodes,
            full_topology: CachedFullTopology::new_empty(),
        }
    }

    fn network_topology(&self, min_mix_performance: u8) -> NymTopology {
        let builder_guard = self.topology_builder.load();
        builder_guard.build(min_mix_performance)
    }
}

#[derive(Default)]
struct NymTopologyBuilder {
    rewarded_set: EpochRewardedSet,
    topology_metadata: NymTopologyMetadata,
    network_nodes: Vec<SemiSkimmedNodeV3>,
}

impl NymTopologyBuilder {
    fn build(&self, min_mix_performance: u8) -> NymTopology {
        NymTopology::new(
            self.topology_metadata,
            self.rewarded_set.clone(),
            Vec::new(),
        )
        .with_additional_nodes(self.network_nodes.iter().map(|node| &node.basic).filter(
            |node| {
                if node.supported_roles.mixnode {
                    node.performance.round_to_integer() >= min_mix_performance
                } else {
                    true
                }
            },
        ))
    }
}

#[derive(Clone)]
pub(crate) struct CachedFullTopology {
    inner: Arc<ArcSwap<NymTopology>>,
}

impl CachedFullTopology {
    pub(crate) fn new_empty() -> Self {
        CachedFullTopology {
            inner: Arc::new(ArcSwap::from_pointee(NymTopology::default())),
        }
    }

    pub fn from_topology(topology: NymTopology) -> Self {
        CachedFullTopology {
            inner: Arc::new(ArcSwap::from_pointee(topology)),
        }
    }

    pub(crate) fn find_node(&self, node_id: NodeId) -> Option<RoutingNode> {
        self.inner.load().find_node(node_id).cloned()
    }

    fn store(&self, topology: NymTopology) {
        self.inner.store(Arc::new(topology));
    }
}
