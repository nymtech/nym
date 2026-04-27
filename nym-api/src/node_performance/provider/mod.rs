// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::provider::contract_provider::ContractPerformanceProvider;
use async_trait::async_trait;
use legacy_storage_provider::LegacyStoragePerformanceProvider;
use nym_api_requests::models::{RoutingScore, StressTestingScore};
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error};

pub(crate) mod contract_provider;
pub(crate) mod legacy_storage_provider;

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

pub(crate) struct NodesStressTestingScores {
    inner: HashMap<NodeId, Result<StressTestingScore, PerformanceRetrievalFailure>>,
}

impl NodesStressTestingScores {
    pub(crate) fn get_or_log(&self, node_id: NodeId) -> StressTestingScore {
        todo!()
        // match self.inner.get(&node_id) {
        //     Some(Ok(score)) => *score,
        //     Some(Err(err)) => {
        //         debug!("{err}");
        //         RoutingScore::zero()
        //     }
        //     None => RoutingScore::zero(),
        // }
    }

    pub(crate) fn count(&self) -> usize {
        self.inner.len()
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
    async fn get_node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure>;

    /// An optimisation for obtaining node scores of multiple nodes at once
    async fn get_batch_node_routing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure>;

    /// Obtain a stress-testing score of a particular node for given epoch
    #[allow(unused)]
    async fn get_node_stress_testing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure>;

    /// An optimisation for obtaining node scores of multiple nodes at once
    async fn get_batch_node_stress_testing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure>;
}

#[async_trait]
impl NodePerformanceProvider for ContractPerformanceProvider {
    #[allow(unused)]
    async fn get_node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        self.node_routing_score(node_id, epoch_id).await
    }

    async fn get_batch_node_routing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure> {
        self.node_routing_scores(node_ids, epoch_id).await
    }

    async fn get_node_stress_testing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure> {
        error!("stress testing data not available in contract data");
        Err(PerformanceRetrievalFailure {
            node_id,
            epoch_id,
            error: "stress testing data not available in contract data".to_string(),
        })
    }

    async fn get_batch_node_stress_testing_scores(
        &self,
        _: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure> {
        error!("stress testing data not available in contract data");
        Err(PerformanceRetrievalFailure {
            node_id: 0,
            epoch_id,
            error: "stress testing data not available in contract data".to_string(),
        })
    }
}

#[async_trait]
impl NodePerformanceProvider for LegacyStoragePerformanceProvider {
    #[allow(unused)]
    async fn get_node_routing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<RoutingScore, PerformanceRetrievalFailure> {
        self.node_routing_score(node_id, epoch_id).await
    }

    async fn get_batch_node_routing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesRoutingScores, PerformanceRetrievalFailure> {
        let mut scores = HashMap::new();

        let epoch_timestamp = self.epoch_id_unix_timestamp(epoch_id).await?;
        for &node_id in node_ids {
            scores.insert(
                node_id,
                self.get_node_routing_score_with_unix_timestamp(node_id, epoch_id, epoch_timestamp)
                    .await,
            );
        }

        Ok(NodesRoutingScores { inner: scores })
    }

    async fn get_node_stress_testing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure> {
        todo!()
    }

    async fn get_batch_node_stress_testing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure> {
        todo!()
    }
}
