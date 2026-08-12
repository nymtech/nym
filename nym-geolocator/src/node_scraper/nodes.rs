// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node_scraper::refresh_details_of_bonded_nodes;
use crate::nyx::nodes::MinimalNymNode;
use nym_node_requests::api::v1::node::models::HostInformation;
use nym_validator_client::client::NodeId;
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct MinimalNodeDetails {
    pub(crate) node_id: NodeId,
    pub(crate) host_information: HostInformation,
}

pub(crate) struct KnownNode {
    pub(crate) last_refreshed_at: OffsetDateTime,
    pub(crate) last_known_data: MinimalNodeDetails,
}

pub(crate) enum NodeUpdate {
    IpChanged(MinimalNodeDetails),
    NewNode(MinimalNodeDetails),
}

impl NodeUpdate {
    pub(crate) fn ip_changed(node: MinimalNodeDetails) -> Self {
        NodeUpdate::IpChanged(node)
    }

    pub(crate) fn new_node(node: MinimalNodeDetails) -> Self {
        NodeUpdate::NewNode(node)
    }
}

impl KnownNode {
    pub(crate) fn new(last_known_data: MinimalNodeDetails) -> KnownNode {
        KnownNode {
            last_refreshed_at: OffsetDateTime::now_utc(),
            last_known_data,
        }
    }

    pub(crate) fn ip_changed(&self, new: &MinimalNodeDetails) -> bool {
        // ensure consistent ordering
        let mut old = self.last_known_data.host_information.ip_address.clone();
        let mut new = new.host_information.ip_address.clone();
        old.sort_unstable();
        new.sort_unstable();

        old != new
    }
}

pub(crate) struct KnownNodes {
    nodes: HashMap<NodeId, KnownNode>,
}

impl KnownNodes {
    pub(crate) async fn build_new(
        config: Config,
        bonded: &HashMap<NodeId, MinimalNymNode>,
    ) -> KnownNodes {
        let nodes = refresh_details_of_bonded_nodes(
            bonded.clone(),
            config.number_of_concurrent_node_queries,
            config.node_info_query_timeout,
        )
        .await
        .into_iter()
        .map(|node| (node.node_id, KnownNode::new(node)))
        .collect();

        KnownNodes { nodes }
    }

    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn get(&self, node_id: &NodeId) -> Option<&KnownNode> {
        self.nodes.get(node_id)
    }

    pub(crate) fn get_mut(&mut self, node_id: &NodeId) -> Option<&mut KnownNode> {
        self.nodes.get_mut(node_id)
    }

    pub(crate) fn insert(&mut self, node_id: NodeId, node: KnownNode) {
        self.nodes.insert(node_id, node);
    }

    pub(crate) fn retain(&mut self, nodes: &HashSet<NodeId>) {
        self.nodes.retain(|node_id, _| nodes.contains(node_id));
    }
}
