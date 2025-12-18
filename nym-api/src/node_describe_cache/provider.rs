// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_describe_cache::refresh::RefreshData;
use crate::node_describe_cache::NodeDescribeCacheError;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::{CacheItemProvider, CacheRefresher};
use crate::support::config;
use crate::support::config::DEFAULT_NODE_DESCRIBE_BATCH_SIZE;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info};

pub struct NodeDescriptionProvider {
    contract_cache: MixnetContractCache,

    allow_all_ips: bool,
    batch_size: usize,
}

impl NodeDescriptionProvider {
    pub(crate) fn new(
        contract_cache: MixnetContractCache,
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

#[async_trait]
impl CacheItemProvider for NodeDescriptionProvider {
    type Item = DescribedNodes;
    type Error = NodeDescribeCacheError;

    async fn wait_until_ready(&self) {
        self.contract_cache.naive_wait_for_initial_values().await
    }

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let mut nodes_to_query: Vec<RefreshData> = Vec::new();

        match self.contract_cache.all_cached_nym_nodes().await {
            None => error!("failed to obtain nym-nodes information from the cache"),
            Some(nym_nodes) => {
                for node in &**nym_nodes {
                    if let Ok(data) = node.try_into() {
                        nodes_to_query.push(data);
                    }
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

        let mut addresses_cache = HashMap::new();
        for node in nodes.values() {
            for ip in &node.description.host_information.ip_address {
                addresses_cache.insert(*ip, node.node_id);
            }
        }

        info!("refreshed self described data for {} nodes", nodes.len());
        info!("with {} unique ip addresses", addresses_cache.len());

        Ok(Some(DescribedNodes {
            nodes,
            addresses_cache,
        }))
    }
}

pub(crate) fn new_provider_with_initial_value(
    config: &config::DescribeCache,
    contract_cache: MixnetContractCache,
    initial: SharedCache<DescribedNodes>,
    on_disk_file: PathBuf,
) -> CacheRefresher<DescribedNodes, NodeDescribeCacheError> {
    CacheRefresher::new_with_initial_value(
        Box::new(
            NodeDescriptionProvider::new(contract_cache, config.debug.allow_illegal_ips)
                .with_batch_size(config.debug.batch_size),
        ),
        config.debug.caching_interval,
        initial,
    )
    .named("node-self-described-data-refresher")
    .with_persistent_cache(on_disk_file)
}
