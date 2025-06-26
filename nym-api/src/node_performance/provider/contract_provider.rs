// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::node_performance::provider::{NodesRoutingScores, PerformanceRetrievalFailure};
use crate::support::caching::cache::SharedCache;
use crate::support::config;
use nym_api_requests::models::RoutingScore;
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use tracing::warn;

pub(crate) struct ContractPerformanceProvider {
    cached: SharedCache<PerformanceContractCacheData>,
    max_epochs_fallback: u32,
}

impl ContractPerformanceProvider {
    pub(crate) fn new(
        config: &config::PerformanceProvider,
        contract_cache: SharedCache<PerformanceContractCacheData>,
    ) -> Self {
        ContractPerformanceProvider {
            cached: contract_cache,
            max_epochs_fallback: config.debug.max_performance_fallback_epochs,
        }
    }

    fn node_routing_score_with_fallback(
        &self,
        contract_cache: &PerformanceContractCacheData,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        let err = match contract_cache.node_routing_score(node_id, epoch_id) {
            Ok(res) => return Ok(res),
            Err(err) => err,
        };

        warn!("failed to retrieve performance score of node {node_id} for epoch {epoch_id}. falling back to at most {} past epochs", self.max_epochs_fallback);

        let threshold = epoch_id.saturating_sub(self.max_epochs_fallback);
        let start = epoch_id.saturating_sub(1);
        for epoch_id in start..threshold {
            if let Ok(res) = contract_cache.node_routing_score(node_id, epoch_id) {
                return Ok(res);
            }
        }

        Err(err)
    }

    pub(crate) async fn node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        let contract_cache = self.cached.get().await.map_err(|_| {
            PerformanceRetrievalFailure::new(
                node_id,
                epoch_id,
                "performance contract cache has not been initialised yet",
            )
        })?;

        self.node_routing_score_with_fallback(&contract_cache, node_id, epoch_id)
    }

    pub(crate) async fn node_routing_scores(
        &self,
        node_ids: Vec<NodeId>,
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure> {
        let Some(first) = node_ids.first() else {
            return Ok(NodesRoutingScores::empty());
        };

        let contract_cache = self.cached.get().await.map_err(|_| {
            PerformanceRetrievalFailure::new(
                *first,
                epoch_id,
                "performance contract cache has not been initialised yet",
            )
        })?;

        let mut scores = HashMap::new();
        for node_id in node_ids {
            let score = self.node_routing_score_with_fallback(&contract_cache, node_id, epoch_id);
            scores.insert(node_id, score);
        }

        Ok(NodesRoutingScores { inner: scores })
    }
}
