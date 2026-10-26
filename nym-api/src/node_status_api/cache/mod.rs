// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::NodeStatusCacheData;
use crate::node_performance::provider::PerformanceRetrievalFailure;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::Cache;
use nym_api_requests::models::NodeAnnotationV2;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::RwLockReadGuard;

mod config_score;
mod contract_annotations;
pub mod data;
pub mod refresher;

#[derive(Debug, Error)]
enum NodeStatusCacheError {
    #[error("the current interval information is not available at the moment")]
    SourceDataMissing,

    #[error("the self-described cache data is not available")]
    UnavailableDescribedCache,

    #[error("no performance snapshot has settled yet")]
    NoSettledPerformanceSnapshot,

    #[error("timed out waiting to write the node status cache")]
    WriteTimeout,

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
    inner: SharedCache<NodeStatusCacheData>,
}

impl From<SharedCache<NodeStatusCacheData>> for NodeStatusCache {
    fn from(inner: SharedCache<NodeStatusCacheData>) -> Self {
        NodeStatusCache { inner }
    }
}

impl NodeStatusCache {
    /// The persisted annotations if still fresh, else uninitialised until the first refresh: an
    /// empty map would be served as performance 0 for every node.
    pub(crate) fn new<P: AsRef<Path>>(store_path: P, max_cache_age: Duration) -> NodeStatusCache {
        NodeStatusCache {
            inner: SharedCache::new_with_persistent(store_path, max_cache_age),
        }
    }

    pub async fn cache_timestamp(&self) -> OffsetDateTime {
        let Ok(cache) = self.inner.get().await else {
            return OffsetDateTime::UNIX_EPOCH;
        };

        cache.timestamp()
    }

    /// Updates the cache with the latest data.
    async fn update(
        &self,
        node_annotations: HashMap<NodeId, NodeAnnotationV2>,
    ) -> Result<(), NodeStatusCacheError> {
        self.inner
            .try_overwrite_old_value(node_annotations, "node-status")
            .await
            .map_err(|_| NodeStatusCacheError::WriteTimeout)
    }

    pub(crate) async fn cache(
        &self,
    ) -> Result<RwLockReadGuard<'_, Cache<NodeStatusCacheData>>, UninitialisedCache> {
        self.inner.get().await
    }

    async fn get<'a, T: 'a>(
        &'a self,
        fn_arg: impl FnOnce(&Cache<NodeStatusCacheData>) -> &T,
    ) -> Result<RwLockReadGuard<'a, T>, UninitialisedCache> {
        let guard = self.inner.get().await?;
        Ok(RwLockReadGuard::map(guard, fn_arg))
    }

    pub(crate) async fn node_annotations(
        &self,
    ) -> Result<RwLockReadGuard<'_, HashMap<NodeId, NodeAnnotationV2>>, UninitialisedCache> {
        self.get(|c| &c.node_annotations).await
    }
}
