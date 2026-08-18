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

    /// The addresses this node no longer needs measuring against, sorted.
    ///
    /// Advanced only by a submission that reached the chain, never by discovery. That is what
    /// makes a failed measurement retryable: the announced addresses move on, this does not, and
    /// the node keeps being reported as due until a measurement succeeds. Comparing against the
    /// discovery baseline instead would report the change exactly once, and a failure that
    /// followed it would leave the stale location standing until the entry expired.
    ///
    /// `None` means nothing is known, which happens for a node discovered after startup. Those
    /// defer to the on-chain freshness check rather than being measured on sight.
    last_measured: Option<Vec<IpAddr>>,
}

pub(crate) enum NodeUpdate {
    IpChanged(NodeAddresses),
    NewNode(NodeAddresses),
}

impl KnownNode {
    /// A node discovered after startup, which nothing has measured yet.
    pub(crate) fn newly_discovered(last_known_data: NodeAddresses) -> KnownNode {
        KnownNode {
            last_known_data,
            last_measured: None,
        }
    }

    /// A node adopted as the startup baseline, taken to need no measurement on account of its
    /// addresses alone.
    ///
    /// Not a claim that it was measured - it may never have been. It keeps a cold start out of
    /// the address-change path, which is uncapped because churn is small; a fresh agent's
    /// network-wide backlog belongs to the expiration sweep, which is capped.
    fn adopted(last_known_data: NodeAddresses) -> KnownNode {
        let last_measured = Some(sorted(&last_known_data.addresses));
        KnownNode {
            last_known_data,
            last_measured,
        }
    }

    /// What this node should be reported as, if anything, given what it announces now.
    fn pending_update(&self) -> Option<NodeUpdate> {
        match &self.last_measured {
            // measured against exactly what it announces - nothing to do
            Some(measured) if *measured == sorted(&self.last_known_data.addresses) => None,
            Some(_) => Some(NodeUpdate::IpChanged(self.last_known_data.clone())),
            None => Some(NodeUpdate::NewNode(self.last_known_data.clone())),
        }
    }
}

fn sorted(addresses: &[IpAddr]) -> Vec<IpAddr> {
    let mut sorted = addresses.to_vec();
    sorted.sort_unstable();
    sorted
}

/// The addresses last seen for each node, and the ones each was last measured against.
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
            .map(|node| (node.node_id, KnownNode::adopted(node)))
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

    /// Record what was discovered and report every node that still needs measuring.
    ///
    /// Reported against what was last *measured*, not against the previous discovery, so a node
    /// whose measurement failed is reported again on the next pass rather than only once.
    pub(crate) async fn reconcile(&self, discovered: Vec<NodeAddresses>) -> Vec<NodeUpdate> {
        let mut guard = self.nodes.write().await;

        let mut node_changes = Vec::new();
        for new in discovered {
            let node = guard
                .entry(new.node_id)
                .or_insert_with(|| KnownNode::newly_discovered(new.clone()));
            node.last_known_data = new;

            node_changes.extend(node.pending_update());
        }

        node_changes
    }

    /// Record what was discovered without reporting anything as changed.
    ///
    /// Leaves `last_measured` alone: the caller measures this node itself, and a measurement it
    /// does not complete has to stay outstanding rather than being absorbed here.
    pub(crate) async fn set_baseline(&self, discovered: Vec<NodeAddresses>) {
        let mut guard = self.nodes.write().await;
        for new in discovered {
            guard
                .entry(new.node_id)
                .and_modify(|known| known.last_known_data = new.clone())
                .or_insert_with(|| KnownNode::newly_discovered(new));
        }
    }

    /// Record that these nodes were measured against exactly these addresses and submitted.
    pub(crate) async fn mark_measured(&self, measured: Vec<(NodeId, Vec<IpAddr>)>) {
        let mut guard = self.nodes.write().await;
        for (node_id, addresses) in measured {
            if let Some(known) = guard.get_mut(&node_id) {
                known.last_measured = Some(sorted(&addresses));
            }
        }
    }
}
