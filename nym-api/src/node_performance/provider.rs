// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::node_performance::legacy_storage_provider::LegacyStoragePerformanceProvider;
use crate::support::caching::cache::SharedCache;
use async_trait::async_trait;
use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Error)]
#[error("failed to retrieve performance score for node {node_id} for epoch {epoch_id}: {error}")]
pub(crate) struct PerformanceRetrievalFailure {
    pub(crate) node_id: NodeId,
    pub(crate) epoch_id: EpochId,
    pub(crate) error: String,
}

impl PerformanceRetrievalFailure {
    pub(crate) fn new(node_id: NodeId, epoch_id: EpochId, error: impl Into<String>) -> Self {
        PerformanceRetrievalFailure {
            node_id,
            epoch_id,
            error: error.into(),
        }
    }
}

pub(crate) struct NodesRoutingScores {
    inner: HashMap<NodeId, Result<RoutingScore, PerformanceRetrievalFailure>>,
}

impl NodesRoutingScores {
    pub(crate) fn empty() -> Self {
        NodesRoutingScores {
            inner: HashMap::new(),
        }
    }
    pub(crate) fn get_or_log(&self, node_id: NodeId) -> RoutingScore {
        match self.inner.get(&node_id) {
            Some(Ok(score)) => *score,
            Some(Err(err)) => {
                debug!("{err}");
                RoutingScore::zero()
            }
            None => RoutingScore::zero(),
        }
    }
}

#[async_trait]
pub(crate) trait NodePerformanceProvider {
    /// Obtain a performance/routing score of a particular node for given epoch
    #[allow(unused)]
    async fn get_node_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure>;

    /// An optimisation for obtaining node scores of multiple nodes at once
    async fn get_batch_node_scores(
        &self,
        node_ids: Vec<NodeId>,
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure>;
}

#[async_trait]
impl NodePerformanceProvider for SharedCache<PerformanceContractCacheData> {
    async fn get_node_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        let contract_cache = self.get().await.map_err(|_| {
            PerformanceRetrievalFailure::new(
                node_id,
                epoch_id,
                "performance contract cache has not been initialised yet",
            )
        })?;

        contract_cache.node_routing_score(node_id, epoch_id)
    }

    async fn get_batch_node_scores(
        &self,
        node_ids: Vec<NodeId>,
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure> {
        let Some(first) = node_ids.first() else {
            return Ok(NodesRoutingScores::empty());
        };

        let contract_cache = self.get().await.map_err(|_| {
            PerformanceRetrievalFailure::new(
                *first,
                epoch_id,
                "performance contract cache has not been initialised yet",
            )
        })?;

        let mut scores = HashMap::new();
        for node_id in node_ids {
            scores.insert(
                node_id,
                contract_cache.node_routing_score(node_id, epoch_id),
            );
        }

        Ok(NodesRoutingScores { inner: scores })
    }
}

#[async_trait]
impl NodePerformanceProvider for LegacyStoragePerformanceProvider {
    #[allow(unused)]
    async fn get_node_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        self.node_routing_score(node_id, epoch_id).await
    }

    async fn get_batch_node_scores(
        &self,
        node_ids: Vec<NodeId>,
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure> {
        let mut scores = HashMap::new();

        let epoch_timestamp = self.epoch_id_unix_timestamp(epoch_id).await?;
        for node_id in node_ids {
            scores.insert(
                node_id,
                self.get_node_routing_score_with_unix_timestamp(node_id, epoch_id, epoch_timestamp)
                    .await,
            );
        }

        Ok(NodesRoutingScores { inner: scores })
    }
}
