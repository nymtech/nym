// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node_scraper::nodes::{KnownNode, KnownNodes, MinimalNodeDetails, NodeUpdate};
use crate::nyx::nodes::{BondedNymNodes, MinimalNymNode};
use anyhow::Context;
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::Instant;
use tracing::{debug, info};

pub(crate) mod nodes;

pub(crate) struct NodeScraper {
    bonded_nym_nodes: BondedNymNodes,

    known_nodes: KnownNodes,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    number_of_concurrent_node_queries: usize,

    /// Timeout for querying a single node for its detailed information (ip addresses) (e.g. `10s`).
    node_info_query_timeout: Duration,
}

async fn get_node_details_inner(bond: MinimalNymNode) -> anyhow::Result<MinimalNodeDetails> {
    let node_id = bond.node_id;

    let client = NymNodeApiClientRetriever::new(bin_info!())
        .with_expected_identity(Some(bond.identity.to_base58_string()))
        .with_verify_host_information()
        .with_custom_port(bond.custom_http_port)
        .get_client(&bond.host, node_id)
        .await?;

    Ok(MinimalNodeDetails {
        node_id,
        host_information: client
            .host_information
            .context("failed to retrieve node host information")?
            .data,
    })
}

async fn get_node_details(bond: MinimalNymNode, timeout: Duration) -> Option<MinimalNodeDetails> {
    let node_id = bond.node_id;
    let self_described = match tokio::time::timeout(timeout, get_node_details_inner(bond)).await {
        Err(_timeout) => {
            debug!(
                "timed out while attempting to retrieve self-described node details for node {node_id}"
            );
            return None;
        }
        Ok(Err(err)) => {
            debug!("failed to retrieve self-described node details for node {node_id}: {err}");
            return None;
        }
        Ok(Ok(info)) => info,
    };

    Some(self_described)
}

pub(crate) async fn refresh_details_of_bonded_nodes(
    bonded: HashMap<NodeId, MinimalNymNode>,
    number_of_concurrent_node_queries: usize,
    node_info_query_timeout: Duration,
) -> Vec<MinimalNodeDetails> {
    let start = Instant::now();

    let refreshed_nodes: Vec<_> = stream::iter(bonded)
        .map(|(_, node)| get_node_details(node, node_info_query_timeout))
        .buffer_unordered(number_of_concurrent_node_queries)
        .filter_map(async |n| n)
        .collect()
        .await;

    info!(
        "refreshing node details took {}",
        humantime::format_duration(start.elapsed())
    );

    refreshed_nodes
}

impl NodeScraper {
    pub(crate) fn new(
        config: Config,
        bonded_nym_nodes: BondedNymNodes,
        known_nodes: KnownNodes,
    ) -> Self {
        NodeScraper {
            bonded_nym_nodes,
            known_nodes,
            number_of_concurrent_node_queries: config.number_of_concurrent_node_queries,
            node_info_query_timeout: config.node_info_query_timeout,
        }
    }

    async fn refresh_details_of_bonded_nodes(
        &self,
        bonded: HashMap<NodeId, MinimalNymNode>,
    ) -> Vec<MinimalNodeDetails> {
        refresh_details_of_bonded_nodes(
            bonded,
            self.number_of_concurrent_node_queries,
            self.node_info_query_timeout,
        )
        .await
    }

    pub(crate) fn node_ips(&self, node_id: NodeId) -> Vec<IpAddr> {
        self.known_nodes
            .get(&node_id)
            .map(|node| node.last_known_data.host_information.ip_address.clone())
            .unwrap_or_default()
    }

    /// The ids currently bonded, as of the last chain refresh.
    pub(crate) async fn bonded_ids(&self) -> HashSet<NodeId> {
        self.bonded_nym_nodes.known_ids().await
    }

    pub(crate) async fn get_updated_nodes(&mut self) -> Vec<NodeUpdate> {
        let bonded = self.bonded_nym_nodes.read().await;
        let new_details = self.refresh_details_of_bonded_nodes(bonded.clone()).await;

        // forget nodes that have left the bonded set, so a node that unbonds and later rebonds
        // is treated as new rather than compared against addresses from its previous life
        let bonded = self.bonded_ids().await;
        self.known_nodes.retain(&bonded);

        let mut node_changes = Vec::new();
        for new in new_details {
            if let Some(existing) = self.known_nodes.get_mut(&new.node_id) {
                if existing.ip_changed(&new) {
                    // ip address(es) of the node changed -> we have to refresh its data
                    node_changes.push(NodeUpdate::ip_changed(new.clone()));
                }
                existing.last_refreshed_at = OffsetDateTime::now_utc();
                existing.last_known_data = new;
            } else {
                // brand new node -> we **might** have to refresh its data
                node_changes.push(NodeUpdate::new_node(new.clone()));
                self.known_nodes.insert(new.node_id, KnownNode::new(new));
            }
        }

        node_changes
    }
}
