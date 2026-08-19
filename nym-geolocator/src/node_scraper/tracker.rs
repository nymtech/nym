// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node_scraper::address_source::{AddressSource, NodeAddresses};
use crate::node_scraper::discover_node_addresses;
use crate::node_scraper::nodes::{KnownNodes, NodeUpdate};
use crate::nyx::nodes::MinimalNymNode;
use nym_validator_client::client::NodeId;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;

/// The address source paired with the baseline of what it last returned.
///
/// Keeping the two together, with the source private, is what stops a discovery from bypassing
/// either the baseline update or the per-node address cap: both are properties of any discovery
/// rather than of a particular caller. The startup baseline is the one discovery that predates the
/// tracker, so it applies the cap through the same helper.
#[derive(Clone)]
pub(crate) struct AddressTracker {
    source: Arc<dyn AddressSource>,

    known_nodes: KnownNodes,

    /// Maximum number of addresses a node may announce before its details are rejected.
    max_addresses_per_node: usize,
}

impl AddressTracker {
    pub(crate) fn new(
        config: Config,
        source: Arc<dyn AddressSource>,
        known_nodes: KnownNodes,
    ) -> AddressTracker {
        AddressTracker {
            source,
            known_nodes,
            max_addresses_per_node: config.max_addresses_per_node,
        }
    }

    async fn discover(&self, nodes: &HashMap<NodeId, MinimalNymNode>) -> Vec<NodeAddresses> {
        discover_node_addresses(self.source.as_ref(), nodes, self.max_addresses_per_node).await
    }

    /// Refresh every given node, reporting those whose addresses have changed since the last time.
    pub(crate) async fn refresh_all(
        &self,
        nodes: &HashMap<NodeId, MinimalNymNode>,
    ) -> Vec<NodeUpdate> {
        let discovered = self.discover(nodes).await;
        self.known_nodes.reconcile(discovered).await
    }

    /// Refresh a single node, for a caller that measures it itself.
    ///
    /// The result is recorded rather than reported: the caller is about to measure this node, so
    /// emitting a change here would have the next sweep measure it a second time.
    pub(crate) async fn refresh_node(&self, node: MinimalNymNode) -> Option<NodeAddresses> {
        let node_id = node.node_id;

        let discovered = self.discover(&HashMap::from([(node_id, node)])).await;
        let addresses = discovered.into_iter().next()?;
        self.known_nodes.set_baseline(vec![addresses.clone()]).await;

        Some(addresses)
    }

    /// Every node whose addresses have been successfully discovered - the set that can actually be
    /// measured, as opposed to the set that is merely bonded.
    pub(crate) async fn node_ids(&self) -> Vec<NodeId> {
        self.known_nodes.node_ids().await
    }

    pub(crate) async fn node_ips(&self, node_id: NodeId) -> Vec<IpAddr> {
        self.known_nodes.node_ips(node_id).await
    }

    pub(crate) async fn mark_measured(&self, measured: Vec<(NodeId, Vec<IpAddr>)>) {
        self.known_nodes.mark_measured(measured).await
    }

    pub(crate) async fn retain(&self, bonded: &HashSet<NodeId>) {
        self.known_nodes.retain(bonded).await
    }
}
