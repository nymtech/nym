// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{LivenessScore, RoutingScore, StressTestingScore};
use nym_mixnet_contract_common::{EpochId, NodeId};
use std::collections::HashMap;
use thiserror::Error;
use tracing::debug;

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
