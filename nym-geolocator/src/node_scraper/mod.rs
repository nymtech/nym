// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_scraper::nodes::MinimalNodeDetails;
use crate::nyx::nodes::{BondedNymNodes, MinimalNymNode};
use anyhow::Context;
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::Instant;
use tracing::{debug, info};

pub(crate) mod nodes;

pub(crate) struct KnownNode {
    last_refreshed_at: OffsetDateTime,
    last_known_data: MinimalNodeDetails,
}

pub(crate) enum NodeUpdate {
    IpChanged(MinimalNodeDetails),
    NewNode(MinimalNodeDetails),
}

impl NodeUpdate {
    fn ip_changed(node: MinimalNodeDetails) -> Self {
        NodeUpdate::IpChanged(node)
    }

    fn new_node(node: MinimalNodeDetails) -> Self {
        NodeUpdate::NewNode(node)
    }
}

impl KnownNode {
    fn new(last_known_data: MinimalNodeDetails) -> KnownNode {
        KnownNode {
            last_refreshed_at: OffsetDateTime::now_utc(),
            last_known_data,
        }
    }

    fn ip_changed(&self, new: &MinimalNodeDetails) -> bool {
        // ensure consistent ordering
        let mut old = self.last_known_data.host_information.ip_address.clone();
        let mut new = new.host_information.ip_address.clone();
        old.sort_unstable();
        new.sort_unstable();

        old != new
    }
}

pub(crate) struct NodeScraper {
    bonded_nym_nodes: BondedNymNodes,

    // TODO: must be built at initialisation
    known_nodes: HashMap<NodeId, KnownNode>,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    number_of_concurrent_node_queries: usize,

    /// Timeout for querying a single node for its detailed information (ip addresses) (e.g. `10s`).
    node_info_query_timeout: Duration,
}

impl NodeScraper {
    async fn get_node_details_inner(
        &self,
        bond: MinimalNymNode,
    ) -> anyhow::Result<MinimalNodeDetails> {
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

    async fn get_node_details(
        &self,
        bond: MinimalNymNode,
        timeout: Duration,
    ) -> Option<MinimalNodeDetails> {
        let node_id = bond.node_id;
        let self_described = match tokio::time::timeout(timeout, self.get_node_details_inner(bond))
            .await
        {
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

    async fn refresh_bonded_nodes(&self) -> Vec<MinimalNodeDetails> {
        let start = Instant::now();

        // 1. retrieve all known nodes from the contract
        let to_refresh = self.bonded_nym_nodes.read().await;
        let num_nodes = to_refresh.len();
        info!("going to look-up {num_nodes} bonded nodes from the contract");

        // 2. retrieve detailed information from the self-described endpoints
        let timeout = self.node_info_query_timeout;
        let refreshed_nodes: Vec<_> = stream::iter(to_refresh.clone())
            .map(|(_, node)| self.get_node_details(node, timeout))
            .buffer_unordered(self.number_of_concurrent_node_queries)
            .filter_map(async |n| n)
            .collect()
            .await;

        info!(
            "refreshing node details took {}",
            humantime::format_duration(start.elapsed())
        );

        refreshed_nodes
    }

    pub(crate) fn node_ips(&self, node_id: NodeId) -> Vec<IpAddr> {
        self.known_nodes
            .get(&node_id)
            .map(|node| node.last_known_data.host_information.ip_address.clone())
            .unwrap_or_default()
    }

    pub(crate) async fn get_updated_nodes(&mut self) -> Vec<NodeUpdate> {
        let new_details = self.refresh_bonded_nodes().await;

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
