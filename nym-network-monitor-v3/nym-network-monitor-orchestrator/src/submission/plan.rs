// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! What an epoch's submission carries, decided from the epoch's stored rows alone.

use crate::storage::models::{DescribedNodeRoles, MixnetEpochAggregate, MixnetEpochConfigScore};
use nym_validator_client::nyxd::nym_performance_contract_common::{
    EpochId, NetworkMonitorSubmissionMetadata, NodeSubmission,
};

/// One test kind's share of its eligible nodes with a reachable aggregate, and whether the guard
/// omitted the test kind for it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct TestKindCoverage {
    pub(crate) coverage: f64,
    pub(crate) omitted: bool,
}

/// Each test kind's coverage for one epoch.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Coverage {
    pub(crate) mixnode_liveness: TestKindCoverage,
    pub(crate) gateway_liveness: TestKindCoverage,
    pub(crate) stress: TestKindCoverage,
}

/// What one epoch's submission carries, before the contract's cursor is applied.
#[derive(Debug, Clone)]
pub(crate) struct EpochPlan {
    /// The nodes to submit, in ascending node id.
    pub(crate) nodes: Vec<NodeSubmission>,

    pub(crate) coverage: Coverage,
}

impl EpochPlan {
    /// Builds the plan from the epoch's stored rows and the described nodes' roles.
    pub(crate) fn build(
        config_scores: &[MixnetEpochConfigScore],
        aggregates: Vec<MixnetEpochAggregate>,
        roles: &[DescribedNodeRoles],
        coverage_threshold: f64,
    ) -> Self {
        todo!()
    }
}

/// The nodes planned for `epoch` that the contract's cursor has not passed yet.
pub(crate) fn unsent(
    nodes: Vec<NodeSubmission>,
    epoch: EpochId,
    cursor: NetworkMonitorSubmissionMetadata,
) -> anyhow::Result<Vec<NodeSubmission>> {
    todo!()
}
