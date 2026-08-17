// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node_scraper::address_source::{AddressSource, NodeAddresses};
use crate::node_scraper::discover_node_addresses;
use crate::nyx::nodes::MinimalNymNode;
use nym_validator_client::client::NodeId;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) struct KnownNode {
    pub(crate) last_known_data: NodeAddresses,
}

pub(crate) enum NodeUpdate {
    IpChanged(NodeAddresses),
    NewNode(NodeAddresses),
}

impl KnownNode {
    pub(crate) fn new(last_known_data: NodeAddresses) -> KnownNode {
        KnownNode { last_known_data }
    }

    pub(crate) fn ip_changed(&self, new: &NodeAddresses) -> bool {
        // ensure consistent ordering
        let mut old = self.last_known_data.addresses.clone();
        let mut new = new.addresses.clone();
        old.sort_unstable();
        new.sort_unstable();

        old != new
    }
}

/// The addresses last seen for each node, against which the change detection compares.
///
/// Shared, since both the refresh cycle and the http handlers record into it, and held behind the
/// lock rather than being locked from the outside so that discovery - which takes minutes for the
/// whole network - always runs unguarded and cannot stall a request.
#[derive(Clone)]
pub(crate) struct KnownNodes {
    nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
}

impl KnownNodes {
    /// Record the addresses of every bonded node as the starting baseline.
    ///
    /// A cold start has no baseline, so every node would otherwise look new and be measured at
    /// once. This first discovery *is* the baseline; anything that moved while the service was
    /// down is caught by the regular sweep instead.
    pub(crate) async fn build_new(
        config: Config,
        source: &dyn AddressSource,
        bonded: &HashMap<NodeId, MinimalNymNode>,
    ) -> KnownNodes {
        let nodes = discover_node_addresses(source, bonded, config.max_addresses_per_node)
            .await
            .into_iter()
            .map(|node| (node.node_id, KnownNode::new(node)))
            .collect();

        KnownNodes {
            nodes: Arc::new(RwLock::new(nodes)),
        }
    }

    pub(crate) async fn len(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Ids of every node whose addresses have been discovered.
    pub(crate) async fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.read().await.keys().copied().collect()
    }

    pub(crate) async fn node_ips(&self, node_id: NodeId) -> Vec<IpAddr> {
        self.nodes
            .read()
            .await
            .get(&node_id)
            .map(|node| node.last_known_data.addresses.clone())
            .unwrap_or_default()
    }

    pub(crate) async fn retain(&self, nodes: &HashSet<NodeId>) {
        self.nodes
            .write()
            .await
            .retain(|node_id, _| nodes.contains(node_id));
    }

    /// Record what was discovered and report how it differs from the previous baseline.
    pub(crate) async fn reconcile(&self, discovered: Vec<NodeAddresses>) -> Vec<NodeUpdate> {
        let mut guard = self.nodes.write().await;

        let mut node_changes = Vec::new();
        for new in discovered {
            if let Some(existing) = guard.get_mut(&new.node_id) {
                if existing.ip_changed(&new) {
                    // ip address(es) of the node changed -> we have to refresh its data
                    node_changes.push(NodeUpdate::IpChanged(new.clone()));
                }
                existing.last_known_data = new;
            } else {
                // brand new node -> we **might** have to refresh its data
                node_changes.push(NodeUpdate::NewNode(new.clone()));
                guard.insert(new.node_id, KnownNode::new(new));
            }
        }

        node_changes
    }

    /// Record what was discovered without reporting anything as changed.
    pub(crate) async fn set_baseline(&self, discovered: Vec<NodeAddresses>) {
        let mut guard = self.nodes.write().await;
        for new in discovered {
            guard.insert(new.node_id, KnownNode::new(new));
        }
    }
}
