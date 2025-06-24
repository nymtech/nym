// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::node_performance::legacy_storage_provider::LegacyStoragePerformanceProvider;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use thiserror::Error;

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

pub(crate) trait NodePerformanceProvider {
    /// Obtain a performance/routing score of a particular node for given epoch
    async fn get_node_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure>;

    // /// An optimisation for obtaining node scores of multiple nodes at once
    // async fn get_batch_node_scores(&self, node_ids: Vec<NodeId>, epoch_id: EpochId) -> Result<HashMap<NodeId, PerformanceRetrievalFailure>, PerformanceRetrievalFailure>;
}

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
}

impl NodePerformanceProvider for LegacyStoragePerformanceProvider {
    async fn get_node_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        self.node_routing_score(node_id, epoch_id).await
    }
}
