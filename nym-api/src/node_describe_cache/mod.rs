// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::refresher::{CacheItemProvider, CacheRefresher};
use crate::support::config;
use crate::support::config::DEFAULT_NODE_DESCRIBE_BATCH_SIZE;
use futures_util::{stream, StreamExt};
use nym_api_requests::models::NymNodeDescription;
use nym_contracts_common::IdentityKey;
use nym_mixnet_contract_common::Gateway;
use nym_node_requests::api::client::{NymNodeApiClientError, NymNodeApiClientExt};
use nym_node_requests::api::v1::gateway::models::WebSockets;
use std::collections::HashMap;
use thiserror::Error;

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

    #[error("failed to query gateway '{gateway}': {source}")]
    ApiFailure {
        gateway: IdentityKey,

        #[source]
        source: NymNodeApiClientError,
    },
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

async fn get_gateway_websocket_support(
    gateway: Gateway,
) -> Result<(IdentityKey, WebSockets), NodeDescribeCacheError> {
    let client = match nym_node_requests::api::Client::new_url(&gateway.host, None) {
        Ok(client) => client,
        Err(err) => {
            return Err(NodeDescribeCacheError::MalformedHost {
                host: gateway.host,
                gateway: gateway.identity_key,
                source: err,
            });
        }
    };

    let websockets =
        client
            .get_mixnet_websockets()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: gateway.identity_key.clone(),
                source: err,
            })?;

    Ok((gateway.identity_key, websockets))
}

#[async_trait]
impl CacheItemProvider for NodeDescriptionProvider {
    type Item = HashMap<IdentityKey, NymNodeDescription>;
    type Error = NodeDescribeCacheError;

    async fn wait_until_ready(&self) {
        self.contract_cache.wait_for_initial_values().await
    }

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error> {
        let gateways = self.contract_cache.gateways_all().await;

        // let guard = self.network_gateways.get().await?;
        // let gateways = &*guard;

        if gateways.is_empty() {
            return Ok(HashMap::new());
        }

        // TODO: somehow bypass the 'higher-ranked lifetime error' and remove that redundant clone
        let websockets = stream::iter(
            gateways
                // .deref()
                // .clone()
                .into_iter()
                .map(|bond| bond.gateway)
                .map(get_gateway_websocket_support),
        )
        .buffer_unordered(self.batch_size)
        .filter_map(|res| async move {
            match res {
                Ok((identity, mixnet_websockets)) => {
                    Some((identity, NymNodeDescription { mixnet_websockets }))
                }
                Err(err) => {
                    // TODO: reduce it to trace/debug before PR
                    warn!("{err}");
                    None
                }
            }
        })
        .collect::<HashMap<_, _>>()
        .await;

        Ok(websockets)
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
