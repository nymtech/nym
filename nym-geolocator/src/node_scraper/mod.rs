// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_scraper::address_source::{AddressSource, NodeAddresses};
use crate::node_scraper::nodes::NodeUpdate;
use crate::node_scraper::tracker::AddressTracker;
use crate::nyx::nodes::{BondedNymNodes, MinimalNymNode};
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use tracing::warn;

pub(crate) mod address_source;
pub(crate) mod nodes;
pub(crate) mod tracker;

#[derive(Clone)]
pub(crate) struct NodeScraper {
    bonded_nym_nodes: BondedNymNodes,

    address_tracker: AddressTracker,
}

/// Discover the addresses of every given node, dropping the ones that announced more than they
/// are permitted to.
///
/// The cap is applied here rather than inside a source because it is a policy about unverified
/// input, not about http: the directory contract's addresses are self-published in exactly the
/// same sense, so every source has to be bounded and none of them should have to remember to do
/// it. Rejected rather than truncated, as keeping the first few would let a node choose which of
/// its addresses gets geolocated simply by ordering them.
pub(crate) async fn discover_node_addresses(
    source: &dyn AddressSource,
    bonded: &HashMap<NodeId, MinimalNymNode>,
    max_addresses_per_node: usize,
) -> Vec<NodeAddresses> {
    source
        .discover(bonded)
        .await
        .into_iter()
        .filter(|node| {
            let announced = node.addresses.len();
            if announced > max_addresses_per_node {
                warn!(
                    "node {} announced {announced} addresses, more than the permitted {max_addresses_per_node}",
                    node.node_id
                );
                return false;
            }
            true
        })
        .collect()
}

impl NodeScraper {
    pub(crate) fn new(bonded_nym_nodes: BondedNymNodes, address_tracker: AddressTracker) -> Self {
        NodeScraper {
            bonded_nym_nodes,
            address_tracker,
        }
    }

    /// Every node whose addresses have been successfully discovered - the set that can actually be
    /// measured, as opposed to the set that is merely bonded.
    pub(crate) async fn known_node_ids(&self) -> Vec<NodeId> {
        self.address_tracker.node_ids().await
    }

    pub(crate) async fn node_ips(&self, node_id: NodeId) -> Vec<IpAddr> {
        self.address_tracker.node_ips(node_id).await
    }

    /// Record that these nodes were measured against exactly these addresses and submitted, so
    /// that they stop being reported as due. Nothing else advances that state: a measurement that
    /// failed anywhere between the lookup and the chain leaves the node outstanding.
    pub(crate) async fn mark_measured(&self, measured: Vec<(NodeId, Vec<IpAddr>)>) {
        self.address_tracker.mark_measured(measured).await
    }

    /// The ids currently bonded, as of the last chain refresh.
    pub(crate) async fn bonded_ids(&self) -> HashSet<NodeId> {
        self.bonded_nym_nodes.known_ids().await
    }

    pub(crate) async fn get_updated_nodes(&self) -> Vec<NodeUpdate> {
        // forget nodes that have left the bonded set, so a node that unbonds and later rebonds
        // is treated as new rather than compared against addresses from its previous life
        let bonded_ids = self.bonded_ids().await;
        self.address_tracker.retain(&bonded_ids).await;

        // cloned so the guard is released before the discovery, which takes minutes
        let bonded = self.bonded_nym_nodes.read().await.clone();
        self.address_tracker.refresh_all(&bonded).await
    }

    /// The bond of a single node, as of the last chain refresh, if it is currently bonded.
    ///
    /// Also carries the identity key a node-signed request is verified against.
    pub(crate) async fn bonded_node(&self, node_id: NodeId) -> Option<MinimalNymNode> {
        self.bonded_nym_nodes.read().await.get(&node_id).cloned()
    }

    /// Refresh a single node on demand, for a caller that measures it itself.
    ///
    /// Answers `None` for a node that is not bonded, since the contract deletes a node's entries
    /// when it unbonds and has no way to delete them a second time.
    pub(crate) async fn refresh_node(&self, node_id: NodeId) -> Option<NodeAddresses> {
        let bond = self.bonded_node(node_id).await?;
        self.address_tracker.refresh_node(bond).await
    }
}
