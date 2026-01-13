// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::NodeStatusCacheData;
use crate::node_performance::provider::PerformanceRetrievalFailure;
use crate::support::caching::cache::{SharedCache, UninitialisedCache};
use crate::support::caching::Cache;
use nym_api_requests::models::NodeAnnotation;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::RwLockReadGuard;
use tracing::error;

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
    inner: SharedCache<NodeStatusCacheData>,
}

impl NodeStatusCache {
    /// Creates a new cache with no data.
    pub(crate) fn new() -> NodeStatusCache {
        NodeStatusCache {
            inner: SharedCache::new_with_value(HashMap::new().into()),
        }
    }

    pub async fn cache_timestamp(&self) -> OffsetDateTime {
        let Ok(cache) = self.inner.get().await else {
            return OffsetDateTime::UNIX_EPOCH;
        };

        cache.timestamp()
    }

    /// Updates the cache with the latest data.
    async fn update(&self, node_annotations: HashMap<NodeId, NodeAnnotation>) {
        if self
            .inner
            .try_overwrite_old_value(node_annotations, "node-status")
            .await
            .is_err()
        {
            error!("failed to update node status cache!")
        }
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
    ) -> Result<RwLockReadGuard<'_, HashMap<NodeId, NodeAnnotation>>, UninitialisedCache> {
        self.get(|c| &c.node_annotations).await
    }
}
