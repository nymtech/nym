// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::query_helpers::query_for_described_data;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::refresher::{CacheItemProvider, CacheRefresher};
use crate::support::config;
use crate::support::config::DEFAULT_NODE_DESCRIBE_BATCH_SIZE;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::{DescribedNodeType, NymNodeData, NymNodeDescription};
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_node_requests::api::client::{NymNodeApiClientError, NymNodeApiClientExt};
use nym_topology::node::{RoutingNode, RoutingNodeError};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info};

mod query_helpers;

#[derive(Debug, Error)]
pub enum NodeDescribeCacheError {
    #[error("contract cache hasn't been initialised")]
    UninitialisedContractCache {
        #[from]
        source: UninitialisedCache,
    },

    #[error("node {node_id} has provided malformed host information ({host}: {source}")]
    MalformedHost {
        host: String,

        node_id: NodeId,

        #[source]
        source: NymNodeApiClientError,
    },

    #[error("node {node_id} with host '{host}' doesn't seem to expose its declared http port nor any of the standard API ports, i.e.: 80, 443 or {}", DEFAULT_NYM_NODE_HTTP_PORT)]
    NoHttpPortsAvailable { host: String, node_id: NodeId },

    #[error("failed to query node {node_id}: {source}")]
    ApiFailure {
        node_id: NodeId,

        #[source]
        source: NymNodeApiClientError,
    },

    // TODO: perhaps include more details here like whether key/signature/payload was malformed
    #[error("could not verify signed host information for node {node_id}")]
    MissignedHostInformation { node_id: NodeId },

    #[error("node {node_id} is announcing an illegal ip address")]
    IllegalIpAddress { node_id: NodeId },
}

// this exists because I've been moving things around quite a lot and now the place that holds the type
// doesn't have relevant dependencies for proper impl
pub(crate) trait NodeDescriptionTopologyExt {
    fn try_to_topology_node(&self) -> Result<RoutingNode, RoutingNodeError>;
}

impl NodeDescriptionTopologyExt for NymNodeDescription {
    fn try_to_topology_node(&self) -> Result<RoutingNode, RoutingNodeError> {
        // for the purposes of routing, performance is completely ignored,
        // so add dummy value and piggyback on existing conversion
        (&self.to_skimmed_node(Default::default(), Default::default())).try_into()
    }
}

#[derive(Debug, Clone)]
pub struct DescribedNodes {
    nodes: HashMap<NodeId, NymNodeDescription>,
}

impl DescribedNodes {
    pub fn force_update(&mut self, node: NymNodeDescription) {
        self.nodes.insert(node.node_id, node);
    }

    pub fn get_description(&self, node_id: &NodeId) -> Option<&NymNodeData> {
        self.nodes.get(node_id).map(|n| &n.description)
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NymNodeDescription> {
        self.nodes.get(node_id)
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &NymNodeDescription> {
        self.nodes.values()
    }

    pub fn all_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescription> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeType::NymNode)
    }

    pub fn mixing_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescription> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeType::NymNode)
            .filter(|n| n.description.declared_role.mixnode)
    }

    pub fn entry_capable_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescription> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeType::NymNode)
            .filter(|n| n.description.declared_role.entry)
    }

    pub fn exit_capable_nym_nodes(&self) -> impl Iterator<Item = &NymNodeDescription> {
        self.nodes
            .values()
            .filter(|n| n.contract_node_type == DescribedNodeType::NymNode)
            .filter(|n| n.description.declared_role.can_operate_exit_gateway())
    }
}

pub struct NodeDescriptionProvider {
    contract_cache: NymContractCache,

    allow_all_ips: bool,
    batch_size: usize,
}

impl NodeDescriptionProvider {
    pub(crate) fn new(
        contract_cache: NymContractCache,
        allow_all_ips: bool,
    ) -> NodeDescriptionProvider {
        NodeDescriptionProvider {
            contract_cache,
            allow_all_ips,
            batch_size: DEFAULT_NODE_DESCRIBE_BATCH_SIZE,
        }
    }

    #[must_use]
    pub(crate) fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

async fn try_get_client(
    host: &str,
    node_id: NodeId,
    custom_port: Option<u16>,
) -> Result<nym_node_requests::api::Client, NodeDescribeCacheError> {
    // first try the standard port in case the operator didn't put the node behind the proxy,
    // then default https (443)
    // finally default http (80)
    let mut addresses_to_try = vec![
        format!("http://{host}:{DEFAULT_NYM_NODE_HTTP_PORT}"), // 'standard' nym-node
        format!("https://{host}"),                             // node behind https proxy (443)
        format!("http://{host}"),                              // node behind http proxy (80)
    ];

    // note: I removed 'standard' legacy mixnode port because it should now be automatically pulled via
    // the 'custom_port' since it should have been present in the contract.

    if let Some(port) = custom_port {
        addresses_to_try.insert(0, format!("http://{host}:{port}"));
    }

    for address in addresses_to_try {
        // if provided host was malformed, no point in continuing
        let client = match nym_node_requests::api::Client::builder(address).and_then(|b| {
            b.with_timeout(Duration::from_secs(5))
                .with_user_agent("nym-api-describe-cache")
                .build()
        }) {
            Ok(client) => client,
            Err(err) => {
                return Err(NodeDescribeCacheError::MalformedHost {
                    host: host.to_string(),
                    node_id,
                    source: err,
                });
            }
        };

        if let Ok(health) = client.get_health().await {
            if health.status.is_up() {
                return Ok(client);
            }
        }
    }

    Err(NodeDescribeCacheError::NoHttpPortsAvailable {
        host: host.to_string(),
        node_id,
    })
}

async fn try_get_description(
    data: RefreshData,
    allow_all_ips: bool,
) -> Result<NymNodeDescription, NodeDescribeCacheError> {
    let client = try_get_client(&data.host, data.node_id, data.port).await?;

    let map_query_err = |err| NodeDescribeCacheError::ApiFailure {
        node_id: data.node_id,
        source: err,
    };

    let host_info = client.get_host_information().await.map_err(map_query_err)?;

    if !host_info.verify_host_information() {
        return Err(NodeDescribeCacheError::MissignedHostInformation {
            node_id: data.node_id,
        });
    }

    if !allow_all_ips && !host_info.data.check_ips() {
        return Err(NodeDescribeCacheError::IllegalIpAddress {
            node_id: data.node_id,
        });
    }

    let node_info = query_for_described_data(&client, data.node_id).await?;
    let description = node_info.into_node_description(host_info.data);

    Ok(NymNodeDescription {
        node_id: data.node_id,
        contract_node_type: data.node_type,
        description,
    })
}

#[derive(Debug)]
pub(crate) struct RefreshData {
    host: String,
    node_id: NodeId,
    node_type: DescribedNodeType,

    port: Option<u16>,
}

impl<'a> From<&'a LegacyMixNodeDetailsWithLayer> for RefreshData {
    fn from(node: &'a LegacyMixNodeDetailsWithLayer) -> Self {
        RefreshData::new(
            &node.bond_information.mix_node.host,
            DescribedNodeType::LegacyMixnode,
            node.mix_id(),
            Some(node.bond_information.mix_node.http_api_port),
        )
    }
}

impl<'a> From<&'a LegacyGatewayBondWithId> for RefreshData {
    fn from(node: &'a LegacyGatewayBondWithId) -> Self {
        RefreshData::new(
            &node.bond.gateway.host,
            DescribedNodeType::LegacyGateway,
            node.node_id,
            None,
        )
    }
}

impl<'a> From<&'a NymNodeDetails> for RefreshData {
    fn from(node: &'a NymNodeDetails) -> Self {
        RefreshData::new(
            &node.bond_information.node.host,
            DescribedNodeType::NymNode,
            node.node_id(),
            node.bond_information.node.custom_http_port,
        )
    }
}

impl RefreshData {
    pub fn new(
        host: impl Into<String>,
        node_type: DescribedNodeType,
        node_id: NodeId,
        port: Option<u16>,
    ) -> Self {
        RefreshData {
            host: host.into(),
            node_id,
            node_type,
            port,
        }
    }

    pub(crate) fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub(crate) async fn try_refresh(self, allow_all_ips: bool) -> Option<NymNodeDescription> {
        match try_get_description(self, allow_all_ips).await {
            Ok(description) => Some(description),
            Err(err) => {
                debug!("failed to obtain node self-described data: {err}");
                None
            }
        }
    }
}

#[async_trait]
impl CacheItemProvider for NodeDescriptionProvider {
    type Item = DescribedNodes;
    type Error = NodeDescribeCacheError;

    async fn wait_until_ready(&self) {
        self.contract_cache.wait_for_initial_values().await
    }

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error> {
        // we need to query:
        // - legacy mixnodes (because they might already be running nym-nodes, but haven't updated contract info)
        // - legacy gateways (because they might already be running nym-nodes, but haven't updated contract info)
        // - nym-nodes

        let mut nodes_to_query: Vec<RefreshData> = Vec::new();

        match self.contract_cache.all_cached_legacy_mixnodes().await {
            None => error!("failed to obtain mixnodes information from the cache"),
            Some(legacy_mixnodes) => {
                for node in &**legacy_mixnodes {
                    nodes_to_query.push(node.into())
                }
            }
        }

        match self.contract_cache.all_cached_legacy_gateways().await {
            None => error!("failed to obtain gateways information from the cache"),
            Some(legacy_gateways) => {
                for node in &**legacy_gateways {
                    nodes_to_query.push(node.into())
                }
            }
        }

        match self.contract_cache.all_cached_nym_nodes().await {
            None => error!("failed to obtain nym-nodes information from the cache"),
            Some(nym_nodes) => {
                for node in &**nym_nodes {
                    nodes_to_query.push(node.into())
                }
            }
        }

        let nodes = stream::iter(
            nodes_to_query
                .into_iter()
                .map(|n| n.try_refresh(self.allow_all_ips)),
        )
        .buffer_unordered(self.batch_size)
        .filter_map(|x| async move { x.map(|d| (d.node_id, d)) })
        .collect::<HashMap<_, _>>()
        .await;

        info!("refreshed self described data for {} nodes", nodes.len());

        Ok(DescribedNodes { nodes })
    }
}

// currently dead code : (
#[allow(dead_code)]
pub(crate) fn new_refresher(
    config: &config::TopologyCacher,
    contract_cache: NymContractCache,
) -> CacheRefresher<DescribedNodes, NodeDescribeCacheError> {
    CacheRefresher::new(
        Box::new(
            NodeDescriptionProvider::new(
                contract_cache,
                config.debug.node_describe_allow_illegal_ips,
            )
            .with_batch_size(config.debug.node_describe_batch_size),
        ),
        config.debug.node_describe_caching_interval,
    )
}

pub(crate) fn new_refresher_with_initial_value(
    config: &config::TopologyCacher,
    contract_cache: NymContractCache,
    initial: SharedCache<DescribedNodes>,
) -> CacheRefresher<DescribedNodes, NodeDescribeCacheError> {
    CacheRefresher::new_with_initial_value(
        Box::new(
            NodeDescriptionProvider::new(
                contract_cache,
                config.debug.node_describe_allow_illegal_ips,
            )
            .with_batch_size(config.debug.node_describe_batch_size),
        ),
        config.debug.node_describe_caching_interval,
        initial,
    )
}
