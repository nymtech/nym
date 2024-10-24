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
use nym_api_requests::models::{DescribedNodeType, NymNodeData, NymNodeDescription};
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_mixnet_contract_common::{LegacyMixLayer, NodeId};
use nym_node_requests::api::client::{NymNodeApiClientError, NymNodeApiClientExt};
use nym_topology::gateway::GatewayConversionError;
use nym_topology::mix::MixnodeConversionError;
use nym_topology::{gateway, mix, NetworkAddress};
use std::collections::HashMap;
use std::net::SocketAddr;
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
}

// this exists because I've been moving things around quite a lot and now the place that holds the type
// doesn't have relevant dependencies for proper impl
pub(crate) trait NodeDescriptionTopologyExt {
    fn try_to_topology_mix_node(
        &self,
        layer: LegacyMixLayer,
    ) -> Result<mix::LegacyNode, MixnodeConversionError>;

    fn try_to_topology_gateway(&self) -> Result<gateway::LegacyNode, GatewayConversionError>;
}

impl NodeDescriptionTopologyExt for NymNodeDescription {
    // TODO: this might have to be moved around
    fn try_to_topology_mix_node(
        &self,
        layer: LegacyMixLayer,
    ) -> Result<mix::LegacyNode, MixnodeConversionError> {
        let keys = &self.description.host_information.keys;
        let ips = &self.description.host_information.ip_address;
        if ips.is_empty() {
            return Err(MixnodeConversionError::NoIpAddressesProvided {
                mixnode: keys.ed25519.to_base58_string(),
            });
        }

        let host = match &self.description.host_information.hostname {
            None => NetworkAddress::IpAddr(ips[0]),
            Some(hostname) => NetworkAddress::Hostname(hostname.clone()),
        };

        // get ip from the self-reported values so we wouldn't need to do any hostname resolution
        // (which doesn't really work in wasm)
        let mix_host = SocketAddr::new(ips[0], self.description.mix_port());

        Ok(mix::LegacyNode {
            mix_id: self.node_id,
            host,
            mix_host,
            identity_key: keys.ed25519,
            sphinx_key: keys.x25519,
            layer,
            version: self
                .description
                .build_information
                .build_version
                .as_str()
                .into(),
        })
    }

    fn try_to_topology_gateway(&self) -> Result<gateway::LegacyNode, GatewayConversionError> {
        let keys = &self.description.host_information.keys;

        let ips = &self.description.host_information.ip_address;
        if ips.is_empty() {
            return Err(GatewayConversionError::NoIpAddressesProvided {
                gateway: keys.ed25519.to_base58_string(),
            });
        }

        let host = match &self.description.host_information.hostname {
            None => NetworkAddress::IpAddr(ips[0]),
            Some(hostname) => NetworkAddress::Hostname(hostname.clone()),
        };

        // get ip from the self-reported values so we wouldn't need to do any hostname resolution
        // (which doesn't really work in wasm)
        let mix_host = SocketAddr::new(ips[0], self.description.mix_port());

        Ok(gateway::LegacyNode {
            node_id: self.node_id,
            host,
            mix_host,
            clients_ws_port: self.description.mixnet_websockets.ws_port,
            clients_wss_port: self.description.mixnet_websockets.wss_port,
            identity_key: self.description.host_information.keys.ed25519,
            sphinx_key: self.description.host_information.keys.x25519,
            version: self
                .description
                .build_information
                .build_version
                .as_str()
                .into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DescribedNodes {
    nodes: HashMap<NodeId, NymNodeDescription>,
}

impl DescribedNodes {
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

    batch_size: usize,
}

impl NodeDescriptionProvider {
    pub(crate) fn new(contract_cache: NymContractCache) -> NodeDescriptionProvider {
        NodeDescriptionProvider {
            contract_cache,
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

    let node_info = query_for_described_data(&client, data.node_id).await?;
    let description = node_info.into_node_description(host_info.data);

    Ok(NymNodeDescription {
        node_id: data.node_id,
        contract_node_type: data.node_type,
        description,
    })
}

#[derive(Debug)]
struct RefreshData {
    host: String,
    node_id: NodeId,
    node_type: DescribedNodeType,

    port: Option<u16>,
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

    async fn try_refresh(self) -> Option<NymNodeDescription> {
        match try_get_description(self).await {
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

        let mut nodes_to_query = Vec::new();

        match self.contract_cache.all_cached_legacy_mixnodes().await {
            None => error!("failed to obtain mixnodes information from the cache"),
            Some(legacy_mixnodes) => {
                for node in &**legacy_mixnodes {
                    nodes_to_query.push(RefreshData::new(
                        &node.bond_information.mix_node.host,
                        DescribedNodeType::LegacyMixnode,
                        node.mix_id(),
                        Some(node.bond_information.mix_node.http_api_port),
                    ))
                }
            }
        }

        match self.contract_cache.all_cached_legacy_gateways().await {
            None => error!("failed to obtain gateways information from the cache"),
            Some(legacy_gateways) => {
                for node in &**legacy_gateways {
                    nodes_to_query.push(RefreshData::new(
                        &node.bond.gateway.host,
                        DescribedNodeType::LegacyGateway,
                        node.node_id,
                        None,
                    ))
                }
            }
        }

        match self.contract_cache.all_cached_nym_nodes().await {
            None => error!("failed to obtain nym-nodes information from the cache"),
            Some(nym_nodes) => {
                for node in &**nym_nodes {
                    nodes_to_query.push(RefreshData::new(
                        &node.bond_information.node.host,
                        DescribedNodeType::NymNode,
                        node.node_id(),
                        node.bond_information.node.custom_http_port,
                    ))
                }
            }
        }

        let nodes = stream::iter(nodes_to_query.into_iter().map(|n| n.try_refresh()))
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
            NodeDescriptionProvider::new(contract_cache)
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
            NodeDescriptionProvider::new(contract_cache)
                .with_batch_size(config.debug.node_describe_batch_size),
        ),
        config.debug.node_describe_caching_interval,
        initial,
    )
}
