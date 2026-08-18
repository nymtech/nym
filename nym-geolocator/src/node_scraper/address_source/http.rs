// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node_scraper::address_source::{AddressSource, NodeAddresses};
use crate::nyx::nodes::MinimalNymNode;
use anyhow::{Context, bail};
use async_trait::async_trait;
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_validator_client::client::NodeId;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, info};

/// Addresses as scraped from each node's own http endpoint.
pub(crate) struct HttpAddressSource {
    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    number_of_concurrent_node_queries: usize,

    /// Timeout for querying a single node for its detailed information (ip addresses).
    node_info_query_timeout: Duration,
}

impl HttpAddressSource {
    pub(crate) fn new(config: Config) -> Self {
        HttpAddressSource {
            number_of_concurrent_node_queries: config.number_of_concurrent_node_queries,
            node_info_query_timeout: config.node_info_query_timeout,
        }
    }
}

#[async_trait]
impl AddressSource for HttpAddressSource {
    async fn discover(&self, nodes: &HashMap<NodeId, MinimalNymNode>) -> Vec<NodeAddresses> {
        let start = Instant::now();

        let discovered: Vec<_> = stream::iter(nodes.values().cloned())
            .map(|node| get_node_addresses(node, self.node_info_query_timeout))
            .buffer_unordered(self.number_of_concurrent_node_queries)
            .filter_map(async |node| node)
            .collect()
            .await;

        info!(
            "refreshing node details took {}",
            humantime::format_duration(start.elapsed())
        );

        discovered
    }
}

async fn get_node_addresses_inner(bond: MinimalNymNode) -> anyhow::Result<NodeAddresses> {
    let node_id = bond.node_id;

    let client = NymNodeApiClientRetriever::new(bin_info!())
        .with_expected_identity(Some(bond.identity.to_base58_string()))
        .with_verify_host_information()
        .with_custom_port(bond.custom_http_port)
        .get_client(&bond.host, node_id)
        .await?;

    let host_information = client
        .host_information
        .context("failed to retrieve node host information")?
        .data;

    // announcing the addresses explicitly is the node's job, so one that announces only a
    // hostname is misconfigured and is skipped rather than resolved here: neither this service
    // nor a client should be performing dns lookups on a node's behalf
    if host_information.ip_address.is_empty() {
        bail!("node announced no ip addresses")
    }

    Ok(NodeAddresses {
        node_id,
        addresses: host_information.ip_address,
    })
}

async fn get_node_addresses(bond: MinimalNymNode, timeout: Duration) -> Option<NodeAddresses> {
    let node_id = bond.node_id;

    match tokio::time::timeout(timeout, get_node_addresses_inner(bond)).await {
        Err(_timeout) => {
            debug!("timed out while attempting to retrieve addresses of node {node_id}");
            None
        }
        Ok(Err(err)) => {
            debug!("failed to retrieve addresses of node {node_id}: {err}");
            None
        }
        Ok(Ok(addresses)) => Some(addresses),
    }
}
