// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_noise::config::NoiseNetworkView;
use nym_topology::lp::{LpRoutingNode, LpTopology};
use nym_topology::{EpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata};
use nym_validator_client::nym_api::SemiSkimmedNodeV1;
use nym_validator_client::nym_nodes::SkimmedNodeV2;
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

    fn lp_network_topology(&self, min_mix_performance: u8) -> LpTopology {
        let builder_guard = self.topology_builder.load();
        builder_guard.build_lp(min_mix_performance)
    }
}

/// The two listings this node fetched, and what both views are built from.
///
/// They describe different networks on purpose. `all_nodes` is the node-to-node listing and omits
/// nothing, which is what the forwarding side needs - an address this node refuses to route to is
/// an address its embedded clients must not be handed. `lp_nodes` is the smaller one: only those
/// that published LP details, and so only those an LP route can name.
#[derive(Default)]
struct NymTopologyBuilder {
    rewarded_set: EpochRewardedSet,
    topology_metadata: NymTopologyMetadata,
    all_nodes: Vec<SemiSkimmedNodeV1>,
    lp_nodes: Vec<SkimmedNodeV2>,
}

impl NymTopologyBuilder {
    fn build(&self, min_mix_performance: u8) -> NymTopology {
        NymTopology::new(
            self.topology_metadata,
            self.rewarded_set.clone(),
            Vec::new(),
        )
        .with_additional_nodes(self.all_nodes.iter().map(|node| &node.basic).filter(
            |node| {
                if node.supported_roles.mixnode {
                    node.performance.round_to_integer() >= min_mix_performance
                } else {
                    true
                }
            },
        ))
    }

    fn build_lp(&self, min_mix_performance: u8) -> LpTopology {
        LpTopology::from_nodes(
            self.topology_metadata,
            self.rewarded_set.clone(),
            self.lp_nodes
                .iter()
                .filter(|node| {
                    if node.supported_roles.mixnode {
                        node.performance.round_to_integer() >= min_mix_performance
                    } else {
                        true
                    }
                })
                // the same signature check that admits a node to `LpNodes`, so the directory the
                // control plane dials from and the topology the data plane routes against agree.
                // Without it a node could be routed to over LP and then never dialled, and the
                // frames queued for it would sit until they went stale
                .filter(|node| node.lp.verify(&node.ed25519_identity_pubkey)),
        )
    }
}

#[derive(Clone)]
pub(crate) struct CachedFullTopology {
    inner: Arc<ArcSwap<LpTopology>>,
}

impl CachedFullTopology {
    pub(crate) fn new_empty() -> Self {
        CachedFullTopology {
            inner: Arc::new(ArcSwap::from_pointee(LpTopology::default())),
        }
    }

    pub(crate) fn find_node(&self, node_id: NodeId) -> Option<LpRoutingNode> {
        self.inner.load().find_node(node_id).cloned()
    }

    fn store(&self, topology: LpTopology) {
        self.inner.store(Arc::new(topology));
    }
}
