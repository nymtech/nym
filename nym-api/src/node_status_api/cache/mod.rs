// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::NodeStatusCacheData;
use self::inclusion_probabilities::InclusionProbabilities;
use crate::support::caching::Cache;
use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation};
use nym_contracts_common::IdentityKey;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::RwLockReadGuard;
use tokio::{sync::RwLock, time};
use tracing::error;

const CACHE_TIMEOUT_MS: u64 = 100;

pub mod data;
mod inclusion_probabilities;
mod node_sets;
pub mod refresher;

#[derive(Debug, Error)]
enum NodeStatusCacheError {
    #[error("failed to simulate selection probabilities for mixnodes, not updating cache")]
    SimulationFailed,

    #[error("the current interval information is not available at the moment")]
    SourceDataMissing,
}

/// A node status cache suitable for caching values computed in one sweep, such as active set
/// inclusion probabilities that are computed for all mixnodes at the same time.
///
/// The cache can be triggered to update on contract cache changes, and/or periodically on a timer.
#[derive(Clone)]
pub struct NodeStatusCache {
    inner: Arc<RwLock<NodeStatusCacheData>>,
}

impl NodeStatusCache {
    /// Creates a new cache with no data.
    pub(crate) fn new() -> NodeStatusCache {
        NodeStatusCache {
            inner: Arc::new(RwLock::new(NodeStatusCacheData::new())),
        }
    }

    /// Updates the cache with the latest data.
    async fn update(
        &self,
        legacy_gateway_mapping: HashMap<IdentityKey, NodeId>,
        node_annotations: HashMap<NodeId, NodeAnnotation>,
        mixnodes: HashMap<NodeId, MixNodeBondAnnotated>,
        gateways: HashMap<NodeId, GatewayBondAnnotated>,
        inclusion_probabilities: InclusionProbabilities,
    ) {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache.mixnodes_annotated.unchecked_update(mixnodes);
                cache
                    .legacy_gateway_mapping
                    .unchecked_update(legacy_gateway_mapping);
                cache.node_annotations.unchecked_update(node_annotations);
                cache.gateways_annotated.unchecked_update(gateways);
                cache
                    .inclusion_probabilities
                    .unchecked_update(inclusion_probabilities);
            }
            Err(e) => error!("{e}"),
        }
    }

    /// Returns a copy of the current cache data.
    async fn get_owned<T>(
        &self,
        fn_arg: impl FnOnce(RwLockReadGuard<'_, NodeStatusCacheData>) -> Cache<T>,
    ) -> Option<Cache<T>> {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(fn_arg(cache)),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    async fn get<'a, T: 'a>(
        &'a self,
        fn_arg: impl FnOnce(&NodeStatusCacheData) -> &Cache<T>,
    ) -> Option<RwLockReadGuard<'a, Cache<T>>> {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.read()).await {
            Ok(cache) => Some(RwLockReadGuard::map(cache, |item| fn_arg(item))),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    pub(crate) async fn node_annotations(
        &self,
    ) -> Option<RwLockReadGuard<Cache<HashMap<NodeId, NodeAnnotation>>>> {
        self.get(|c| &c.node_annotations).await
    }

    pub(crate) async fn map_identity_to_node_id(&self, identity: &str) -> Option<NodeId> {
        self.inner
            .read()
            .await
            .legacy_gateway_mapping
            .get(identity)
            .copied()
    }

    pub(crate) async fn annotated_legacy_mixnodes(
        &self,
    ) -> Option<RwLockReadGuard<Cache<HashMap<NodeId, MixNodeBondAnnotated>>>> {
        self.get(|c| &c.mixnodes_annotated).await
    }

    pub(crate) async fn mixnodes_annotated_full(&self) -> Option<Vec<MixNodeBondAnnotated>> {
        let mixnodes = self.get(|c| &c.mixnodes_annotated).await?;

        // just clone everything and return the vec to work with the existing code
        Some(mixnodes.values().cloned().collect())
    }

    pub(crate) async fn mixnodes_annotated_filtered(&self) -> Option<Vec<MixNodeBondAnnotated>> {
        let full = self.mixnodes_annotated_full().await?;
        Some(full.iter().filter(|m| !m.blacklisted).cloned().collect())
    }

    pub(crate) async fn mixnode_annotated(&self, mix_id: NodeId) -> Option<MixNodeBondAnnotated> {
        let mixnodes = self.get(|c| &c.mixnodes_annotated).await?;
        mixnodes.get(&mix_id).cloned()
    }

    pub(crate) async fn annotated_legacy_gateways(
        &self,
    ) -> Option<RwLockReadGuard<Cache<HashMap<NodeId, GatewayBondAnnotated>>>> {
        self.get(|c| &c.gateways_annotated).await
    }

    pub(crate) async fn gateways_annotated_full(&self) -> Option<Vec<GatewayBondAnnotated>> {
        let gateways = self.get(|c| &c.gateways_annotated).await?;

        // just clone everything and return the vec to work with the existing code
        Some(gateways.values().cloned().collect())
    }

    pub(crate) async fn gateways_annotated_filtered(&self) -> Option<Vec<GatewayBondAnnotated>> {
        let full = self.gateways_annotated_full().await?;
        Some(full.iter().filter(|m| !m.blacklisted).cloned().collect())
    }

    pub(crate) async fn gateway_annotated(&self, node_id: NodeId) -> Option<GatewayBondAnnotated> {
        let gateways = self.get(|c| &c.gateways_annotated).await?;
        gateways.get(&node_id).cloned()
    }

    pub(crate) async fn inclusion_probabilities(&self) -> Option<Cache<InclusionProbabilities>> {
        self.get_owned(|c| c.inclusion_probabilities.clone_cache())
            .await
    }
}
