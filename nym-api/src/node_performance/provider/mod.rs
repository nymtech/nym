// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::provider::contract_provider::ContractPerformanceProvider;
use async_trait::async_trait;
use legacy_storage_provider::LegacyStoragePerformanceProvider;
use nym_api_requests::models::{LivenessScore, RoutingScore, StressTestingScore};
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
    /// No stress data for any node, as distinct from a failure to retrieve it. Used by a provider
    /// that structurally has none: every node then reads as `unreachable()`, which is inert
    /// because a provider with no stress data is only ever selected with the component disabled.
    pub(crate) fn empty() -> Self {
        NodesStressTestingScores {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get_or_log(&self, node_id: NodeId) -> StressTestingScore {
        match self.inner.get(&node_id) {
            Some(Ok(score)) => *score,
            Some(Err(err)) => {
                debug!("{err}");
                StressTestingScore::unreachable()
            }
            None => StressTestingScore::unreachable(),
        }
    }

    /// Number of nodes for which the orchestrator has produced at least one reachable sample
    /// in the configured window. Used by the refresher to gate whether stress-testing data is
    /// applied at all: if the orchestrator is down or has not yet submitted anything, this
    /// returns 0 and the refresher falls back to routing × config score only.
    ///
    /// Note: nodes that were tested but found unreachable (`was_reachable=false`) intentionally
    /// do **not** count here. Counting them would let a single recently-rebooted orchestrator
    /// pass the threshold while every node it touched still scored 0.
    pub(crate) fn available_count(&self) -> usize {
        self.inner
            .iter()
            .filter(|(_, v)| match v {
                Ok(score) => score.was_reachable,
                Err(_) => false,
            })
            .count()
    }
}

pub(crate) struct NodesLivenessScores {
    inner: HashMap<NodeId, Result<LivenessScore, PerformanceRetrievalFailure>>,
}

impl NodesLivenessScores {
    /// No liveness data for any node, as distinct from a failure to retrieve it. See
    /// [`NodesStressTestingScores::empty`].
    pub(crate) fn empty() -> Self {
        NodesLivenessScores {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get_or_log(&self, node_id: NodeId) -> LivenessScore {
        match self.inner.get(&node_id) {
            Some(Ok(score)) => *score,
            Some(Err(err)) => {
                debug!("{err}");
                LivenessScore::unreachable()
            }
            None => LivenessScore::unreachable(),
        }
    }

    /// Number of nodes for which the orchestrators have produced at least one reachable liveness
    /// sample in the configured window, on the same reasoning as
    /// [`NodesStressTestingScores::available_count`]: a node that was probed and found unreachable
    /// must not count towards availability, or one freshly-restarted orchestrator could clear the
    /// threshold while every node it touched still scored zero.
    pub(crate) fn available_count(&self) -> usize {
        self.inner
            .iter()
            .filter(|(_, v)| match v {
                Ok(score) => score.was_reachable,
                Err(_) => false,
            })
            .count()
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

    /// Obtain a liveness score of a particular node for given epoch
    #[allow(unused)]
    async fn get_node_liveness_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<LivenessScore, PerformanceRetrievalFailure>;

    /// An optimisation for obtaining node scores of multiple nodes at once
    async fn get_batch_node_liveness_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesLivenessScores, PerformanceRetrievalFailure>;
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

    /// Reports ABSENCE rather than failure, matching [`Self::get_batch_node_routing_scores`].
    ///
    /// This provider structurally has no stress data, which is not an error condition: the cache
    /// refresher fetches every component unconditionally, so returning `Err` here aborted the
    /// whole refresh before it could write any annotations, leaving them frozen for as long as
    /// this provider was selected. Empty is also safe, because config validation forbids enabling
    /// the stress component alongside contract data, so nothing folds these zeroes into a score.
    async fn get_batch_node_stress_testing_scores(
        &self,
        _: &[NodeId],
        _: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure> {
        Ok(NodesStressTestingScores::empty())
    }

    async fn get_node_liveness_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<LivenessScore, PerformanceRetrievalFailure> {
        error!("liveness data not available in contract data");
        Err(PerformanceRetrievalFailure {
            node_id,
            epoch_id,
            error: "liveness data not available in contract data".to_string(),
        })
    }

    /// Reports absence rather than failure, on the same reasoning as the stress equivalent above.
    async fn get_batch_node_liveness_scores(
        &self,
        _: &[NodeId],
        _: EpochId,
    ) -> Result<NodesLivenessScores, PerformanceRetrievalFailure> {
        Ok(NodesLivenessScores::empty())
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

        let epoch_timestamp = self.epoch_id_timestamp(epoch_id).await?.unix_timestamp();
        for &node_id in node_ids {
            scores.insert(
                node_id,
                self.get_node_routing_score_with_unix_timestamp(node_id, epoch_id, epoch_timestamp)
                    .await,
            );
        }

        Ok(NodesRoutingScores { inner: scores })
    }

    #[allow(unused)]
    async fn get_node_stress_testing_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<StressTestingScore, PerformanceRetrievalFailure> {
        self.node_stress_testing_score(node_id, epoch_id).await
    }

    async fn get_batch_node_stress_testing_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesStressTestingScores, PerformanceRetrievalFailure> {
        self.get_node_stress_testing_scores(node_ids, epoch_id)
            .await
    }

    #[allow(unused)]
    async fn get_node_liveness_score(
        &self,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<LivenessScore, PerformanceRetrievalFailure> {
        self.node_liveness_score(node_id, epoch_id).await
    }

    async fn get_batch_node_liveness_scores(
        &self,
        node_ids: &[NodeId],
        epoch_id: EpochId,
    ) -> Result<NodesLivenessScores, PerformanceRetrievalFailure> {
        self.get_node_liveness_scores(node_ids, epoch_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::caching::cache::SharedCache;
    use crate::support::config;

    /// The refresher fetches EVERY component unconditionally, so a provider that structurally has
    /// no data for one of them must report absence rather than failure. When these returned `Err`,
    /// the `?` in `NodeStatusCacheRefresher::refresh` aborted the whole refresh before it wrote a
    /// single annotation, and since its caller discards the error the only symptom was a node
    /// status cache that silently stopped updating for as long as this provider was selected.
    #[tokio::test]
    async fn the_contract_provider_reports_absent_stress_and_liveness_rather_than_failing() {
        let provider = ContractPerformanceProvider::new(
            &config::PerformanceProvider::default(),
            SharedCache::new(),
        );
        let node_ids = [1, 2, 3];

        let stress = provider
            .get_batch_node_stress_testing_scores(&node_ids, 0)
            .await
            .expect("absent stress data must not fail the refresh");
        let liveness = provider
            .get_batch_node_liveness_scores(&node_ids, 0)
            .await
            .expect("absent liveness data must not fail the refresh");

        // absent, so every node reads as unreachable - which is inert, because config validation
        // forbids enabling either component alongside contract data
        assert_eq!(stress.available_count(), 0);
        assert_eq!(liveness.available_count(), 0);
        for node_id in node_ids {
            assert!(!stress.get_or_log(node_id).was_reachable);
            assert!(!liveness.get_or_log(node_id).was_reachable);
        }
    }
}
