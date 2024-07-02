// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::refresher::{CacheItemProvider, CacheRefresher};
use crate::support::config;
use crate::support::config::DEFAULT_NODE_DESCRIBE_BATCH_SIZE;
use futures::{stream, StreamExt};
use nym_api_requests::models::{
    IpPacketRouterDetails, NetworkRequesterDetails, NymNodeDescription, NymNodeRole,
};
use nym_config::defaults::{mainnet, DEFAULT_NYM_NODE_HTTP_PORT};
use nym_contracts_common::IdentityKey;
use nym_node_requests::api::client::{NymNodeApiClientError, NymNodeApiClientExt};
use std::collections::HashMap;
use thiserror::Error;
use time::OffsetDateTime;

// type alias for ease of use
pub type DescribedNodes = HashMap<IdentityKey, NymNodeDescription>;

#[derive(Debug, Error)]
pub enum NodeDescribeCacheError {
    #[error("contract cache hasn't been initialised")]
    UninitialisedContractCache {
        #[from]
        source: UninitialisedCache,
    },

    #[error("gateway {gateway} has provided malformed host information ({host}: {source}")]
    MalformedHost {
        host: String,

        gateway: IdentityKey,

        #[source]
        source: NymNodeApiClientError,
    },

    #[error("gateway '{gateway}' with host '{host}' doesn't seem to expose any of the standard API ports, i.e.: 80, 443 or {}", DEFAULT_NYM_NODE_HTTP_PORT)]
    NoHttpPortsAvailable { host: String, gateway: IdentityKey },

    #[error("failed to query gateway '{gateway}': {source}")]
    ApiFailure {
        gateway: IdentityKey,

        #[source]
        source: NymNodeApiClientError,
    },

    // TODO: perhaps include more details here like whether key/signature/payload was malformed
    #[error("could not verify signed host information for gateway '{gateway}'")]
    MissignedHostInformation { gateway: IdentityKey },
}

pub struct NodeDescriptionProvider {
    // for now we only care about gateways, nothing more
    // network_gateways: SharedCache<Vec<GatewayBond>>,
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
    identity_key: &IdentityKey,
) -> Result<nym_node_requests::api::Client, NodeDescribeCacheError> {
    // first try the standard port in case the operator didn't put the node behind the proxy,
    // then default https (443)
    // finally default http (80)
    let addresses_to_try = vec![
        format!("http://{host}:{DEFAULT_NYM_NODE_HTTP_PORT}"),
        format!("https://{host}"),
        format!("http://{host}"),
    ];

    for address in addresses_to_try {
        // if provided host was malformed, no point in continuing
        let client = match nym_node_requests::api::Client::new_url(address, None) {
            Ok(client) => client,
            Err(err) => {
                return Err(NodeDescribeCacheError::MalformedHost {
                    host: host.to_string(),
                    gateway: identity_key.clone(),
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
        gateway: identity_key.to_string(),
    })
}

async fn try_get_description(
    data: RefreshData,
) -> Result<(IdentityKey, NymNodeDescription), NodeDescribeCacheError> {
    let client = try_get_client(&data.host(), &data.identity_key()).await?;

    let host_info =
        client
            .get_host_information()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: data.identity_key().to_string(),
                source: err,
            })?;

    if !host_info.verify_host_information() {
        return Err(NodeDescribeCacheError::MissignedHostInformation {
            gateway: data.identity_key().clone(),
        });
    }

    let build_info =
        client
            .get_build_information()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: data.identity_key().clone(),
                source: err,
            })?;

    // this can be an old node that hasn't yet exposed this
    let auxiliary_details = client.get_auxiliary_details().await.inspect_err(|err| {
        debug!("could not obtain auxiliary details of node {}: {err} is it running an old version?", data.identity_key());
    }).unwrap_or_default();

    let websockets =
        client
            .get_mixnet_websockets()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: data.identity_key().clone(),
                source: err,
            })?;

    let network_requester =
        if let Ok(nr) = client.get_network_requester().await {
            let exit_policy = client.get_exit_policy().await.map_err(|err| {
                NodeDescribeCacheError::ApiFailure {
                    gateway: data.identity_key().clone(),
                    source: err,
                }
            })?;
            let uses_nym_exit_policy = exit_policy.upstream_source == mainnet::EXIT_POLICY_URL;

            Some(NetworkRequesterDetails {
                address: nr.address,
                uses_exit_policy: exit_policy.enabled && uses_nym_exit_policy,
            })
        } else {
            None
        };

    let ip_packet_router = if let Ok(ipr) = client.get_ip_packet_router().await {
        Some(IpPacketRouterDetails {
            address: ipr.address,
        })
    } else {
        None
    };

    let description = NymNodeDescription {
        host_information: host_info.data.into(),
        last_polled: OffsetDateTime::now_utc().into(),
        build_information: build_info,
        network_requester,
        ip_packet_router,
        mixnet_websockets: websockets.into(),
        auxiliary_details,
        role: data.role(),
    };

    Ok((data.identity_key().clone(), description))
}

struct RefreshData {
    host: String,
    identity_key: IdentityKey,
    role: NymNodeRole,
}

impl RefreshData {
    pub fn new(host: String, identity_key: IdentityKey, role: NymNodeRole) -> Self {
        RefreshData {
            host,
            identity_key,
            role,
        }
    }

    pub fn host(&self) -> String {
        self.host.clone()
    }

    pub fn identity_key(&self) -> IdentityKey {
        self.identity_key.clone()
    }

    pub fn role(&self) -> NymNodeRole {
        self.role.clone()
    }
}

#[async_trait]
impl CacheItemProvider for NodeDescriptionProvider {
    type Item = HashMap<IdentityKey, NymNodeDescription>;
    type Error = NodeDescribeCacheError;

    async fn wait_until_ready(&self) {
        self.contract_cache.wait_for_initial_values().await
    }

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error> {
        let mut host_id_pairs = self
            .contract_cache
            .gateways_all()
            .await
            .into_iter()
            .map(|bond| {
                RefreshData::new(
                    bond.gateway.host,
                    bond.gateway.identity_key,
                    NymNodeRole::Gateway,
                )
            })
            .collect::<Vec<RefreshData>>();

        let nodes = self
            .contract_cache
            .mixnodes_all()
            .await
            .into_iter()
            .map(|node| {
                RefreshData::new(
                    node.bond_information.mix_node.host,
                    node.bond_information.mix_node.identity_key,
                    NymNodeRole::Mixnode,
                )
            });

        host_id_pairs.extend(nodes);
        // let guard = self.network_gateways.get().await?;
        // let gateways = &*guard;

        if host_id_pairs.is_empty() {
            return Ok(HashMap::new());
        }

        // TODO: somehow bypass the 'higher-ranked lifetime error' and remove that redundant clone
        let node_description = stream::iter(
            host_id_pairs
                // .deref()
                // .clone()
                .into_iter()
                .map(try_get_description),
        )
        .buffer_unordered(self.batch_size)
        .filter_map(|res| async move {
            match res {
                Ok((identity, description)) => Some((identity, description)),
                Err(err) => {
                    debug!("failed to obtain gateway self-described data: {err}");
                    None
                }
            }
        })
        .collect::<HashMap<_, _>>()
        .await;

        Ok(node_description)
    }
}

// currently dead code : (
#[allow(dead_code)]
pub(crate) fn new_refresher(
    config: &config::TopologyCacher,
    contract_cache: NymContractCache,
    // hehe. we can't do that yet
    // network_gateways: SharedCache<Vec<GatewayBond>>,
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
    // hehe. we can't do that yet
    // network_gateways: SharedCache<Vec<GatewayBond>>,
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
