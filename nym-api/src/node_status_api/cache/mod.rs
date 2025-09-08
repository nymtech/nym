// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::NodeStatusCacheData;
use crate::node_performance::provider::PerformanceRetrievalFailure;
use crate::support::caching::cache::UninitialisedCache;
use crate::support::caching::Cache;
use nym_api_requests::models::NodeAnnotation;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::RwLockReadGuard;
use tokio::{sync::RwLock, time};
use tracing::error;

const CACHE_TIMEOUT_MS: u64 = 100;

mod config_score;
pub mod data;
pub mod refresher;

#[derive(Debug, Error)]
enum NodeStatusCacheError {
    #[error("the current interval information is not available at the moment")]
    SourceDataMissing,

    #[error("the self-described cache data is not available")]
    UnavailableDescribedCache,

    #[error(transparent)]
    PerformanceRetrievalFailure(#[from] PerformanceRetrievalFailure),
}

impl From<UninitialisedCache> for NodeStatusCacheError {
    fn from(_: UninitialisedCache) -> Self {
        NodeStatusCacheError::SourceDataMissing
    }
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
    async fn update(&self, node_annotations: HashMap<NodeId, NodeAnnotation>) {
        match time::timeout(Duration::from_millis(CACHE_TIMEOUT_MS), self.inner.write()).await {
            Ok(mut cache) => {
                cache.node_annotations.unchecked_update(node_annotations);
            }
            Err(e) => error!("{e}"),
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
    ) -> Option<RwLockReadGuard<'_, Cache<HashMap<NodeId, NodeAnnotation>>>> {
        self.get(|c| &c.node_annotations).await
    }
}
