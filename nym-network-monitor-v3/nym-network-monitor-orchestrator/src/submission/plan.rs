// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! What an epoch's submission carries, decided from the epoch's stored rows alone.

use crate::aggregation::{NodeEpochFigures, epoch_figures};
use crate::storage::models::{
    DescribedNodeRoles, MixnetEpochAggregate, MixnetEpochConfigScore, TestKind,
};
use anyhow::bail;
use nym_contracts_common::{NaiveFloat, Percent};
use nym_validator_client::nyxd::nym_performance_contract_common::{
    EpochId, Measurements, NetworkMonitorSubmissionMetadata, NodeSubmission,
};
use std::cmp::Ordering;
use std::collections::HashSet;

/// One test kind's share of its eligible nodes with a reachable aggregate, and whether the guard
/// omitted the test kind for it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct TestKindCoverage {
    pub(crate) coverage: f64,
    pub(crate) omitted: bool,
}

impl TestKindCoverage {
    /// The share of the nodes the scheduler tests with `kind` whose aggregate of it scores above
    /// zero, omitted below `threshold`.
    ///
    /// A run that failed scores zero, so a fault on the measuring side reads as the whole network
    /// down; omitting the test kind turns that into missing data, which the contract renormalises
    /// over, instead of zeros.
    fn measure(
        kind: TestKind,
        aggregates: &[MixnetEpochAggregate],
        roles: &[DescribedNodeRoles],
        threshold: f64,
    ) -> Self {
        let reachable: HashSet<i64> = aggregates
            .iter()
            .filter(|row| row.test_kind == kind && row.score > 0.0)
            .map(|row| row.node_id)
            .collect();
        let eligible: Vec<i64> = roles
            .iter()
            .filter(|node| node.is_tested_by(kind))
            .map(|node| node.node_id)
            .collect();
        let reached = eligible
            .iter()
            .filter(|node_id| reachable.contains(node_id))
            .count();

        // nothing to measure is no coverage. 0 / 0 would be NaN, which is never below the threshold,
        // so the test kind would silently be kept
        let coverage = if eligible.is_empty() {
            0.0
        } else {
            reached as f64 / eligible.len() as f64
        };

        TestKindCoverage {
            coverage,
            omitted: coverage < threshold,
        }
    }
}

/// Each test kind's coverage for one epoch.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Coverage {
    pub(crate) mixnode_liveness: TestKindCoverage,
    pub(crate) gateway_liveness: TestKindCoverage,
    pub(crate) stress: TestKindCoverage,
}

impl Coverage {
    fn measure(
        aggregates: &[MixnetEpochAggregate],
        roles: &[DescribedNodeRoles],
        threshold: f64,
    ) -> Self {
        let measure = |kind| TestKindCoverage::measure(kind, aggregates, roles, threshold);
        Coverage {
            mixnode_liveness: measure(TestKind::MixnodeLiveness),
            gateway_liveness: measure(TestKind::GatewayLiveness),
            stress: measure(TestKind::MixnodeStress),
        }
    }

    fn for_kind(&self, kind: TestKind) -> TestKindCoverage {
        match kind {
            TestKind::MixnodeLiveness => self.mixnode_liveness,
            TestKind::GatewayLiveness => self.gateway_liveness,
            TestKind::MixnodeStress => self.stress,
        }
    }
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
    ///
    /// The rows of every test kind the guard omits are dropped before the fold, so a dual-role
    /// node's liveness then comes from its other liveness test kind alone.
    pub(crate) fn build(
        config_scores: &[MixnetEpochConfigScore],
        mut aggregates: Vec<MixnetEpochAggregate>,
        roles: &[DescribedNodeRoles],
        coverage_threshold: f64,
    ) -> Self {
        let coverage = Coverage::measure(&aggregates, roles, coverage_threshold);
        aggregates.retain(|row| !coverage.for_kind(row.test_kind).omitted);

        let nodes = epoch_figures(config_scores, &aggregates)
            .into_iter()
            .filter_map(node_submission)
            .collect();

        EpochPlan { nodes, coverage }
    }
}

/// The node's submission: its config score and whichever of liveness and stress it has, or `None`
/// without a config score or without both of the others.
///
/// The contract uses a node's bundle at the epoch whenever one exists, even one it cannot score,
/// so an unscoreable bundle would block the fallback to the node's last scored epoch. This assumes
/// no kind is weighted zero, under which a node with only that kind would be unscoreable too.
fn node_submission(figures: NodeEpochFigures) -> Option<NodeSubmission> {
    let config = percent(figures.config_score?.score)?;
    let liveness = figures
        .aggregates
        .liveness
        .and_then(|liveness| percent(liveness.score));
    let stress = figures
        .aggregates
        .stress
        .and_then(|stress| percent(stress.score));

    if liveness.is_none() && stress.is_none() {
        return None;
    }

    Some(NodeSubmission {
        node_id: figures.node_id,
        measurements: Measurements {
            liveness,
            stress,
            config: Some(config),
        },
    })
}

/// `value` as a percent, clamped to `[0, 1]` against float error. `None` only for a NaN, which no
/// aggregate or config score produces; it is treated as unmeasured rather than as zero.
fn percent(value: f64) -> Option<Percent> {
    Percent::naive_try_from_f64(value.clamp(0.0, 1.0)).ok()
}

/// The nodes planned for `epoch` that the contract's cursor has not passed yet.
///
/// The contract moves the cursor to the last node of every batch it takes, so a cursor at `epoch`
/// has taken every node up to its own, and one at an earlier epoch has taken nothing of this one.
/// A cursor at a later epoch means this reading of the interval is behind the chain, so nothing
/// planned for `epoch` can land.
pub(crate) fn unsent(
    nodes: Vec<NodeSubmission>,
    epoch: EpochId,
    cursor: NetworkMonitorSubmissionMetadata,
) -> anyhow::Result<Vec<NodeSubmission>> {
    match cursor.last_submitted_epoch_id.cmp(&epoch) {
        Ordering::Less => Ok(nodes),
        Ordering::Equal => Ok(nodes
            .into_iter()
            .filter(|node| node.node_id > cursor.last_submitted_node_id)
            .collect()),
        Ordering::Greater => bail!(
            "the submission cursor is already at epoch {}, past epoch {epoch}",
            cursor.last_submitted_epoch_id
        ),
    }
}

/// The threshold is shared with the submitter's tests.
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::storage::models::{FIXTURE_EPOCH, aggregate, config_score};
    use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;

    pub(crate) const THRESHOLD: f64 = 0.5;

    fn roles(node_id: i64, mixnode_enabled: bool, gateway_enabled: bool) -> DescribedNodeRoles {
        DescribedNodeRoles {
            node_id,
            mixnode_enabled,
            gateway_enabled,
        }
    }

    fn mixnode(node_id: i64) -> DescribedNodeRoles {
        roles(node_id, true, false)
    }

    fn gateway(node_id: i64) -> DescribedNodeRoles {
        roles(node_id, false, true)
    }

    fn dual_role(node_id: i64) -> DescribedNodeRoles {
        roles(node_id, true, true)
    }

    fn pct(value: f64) -> Percent {
        Percent::naive_try_from_f64(value).unwrap()
    }

    fn plan(
        config_scores: &[MixnetEpochConfigScore],
        aggregates: Vec<MixnetEpochAggregate>,
        roles: &[DescribedNodeRoles],
    ) -> EpochPlan {
        EpochPlan::build(config_scores, aggregates, roles, THRESHOLD)
    }

    fn node_ids(plan: &EpochPlan) -> Vec<NodeId> {
        plan.nodes.iter().map(|node| node.node_id).collect()
    }

    fn measurements(plan: &EpochPlan, node_id: NodeId) -> Measurements {
        plan.nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .unwrap()
            .measurements
    }

    // absent means unmeasured: a run that failed already scores zero, so a zero would claim a
    // measurement nobody made
    #[test]
    fn an_unmeasured_kind_is_omitted_rather_than_zero() {
        let plan = plan(
            &[config_score(1, 1.0)],
            vec![aggregate(1, TestKind::MixnodeStress, 0.8)],
            &[mixnode(1)],
        );

        assert_eq!(
            measurements(&plan, 1),
            Measurements::default()
                .with_config(pct(1.0))
                .with_stress(pct(0.8))
        );
    }

    #[test]
    fn a_measured_zero_is_submitted() {
        // two of the three mixnodes are reachable, which keeps stress above the threshold
        let plan = plan(
            &[
                config_score(1, 1.0),
                config_score(2, 1.0),
                config_score(3, 1.0),
            ],
            vec![
                aggregate(1, TestKind::MixnodeStress, 0.0),
                aggregate(2, TestKind::MixnodeStress, 1.0),
                aggregate(3, TestKind::MixnodeStress, 1.0),
            ],
            &[mixnode(1), mixnode(2), mixnode(3)],
        );

        assert_eq!(measurements(&plan, 1).stress, Some(pct(0.0)));
    }

    // the contract requires a config value to score a node, and would use the unscoreable bundle
    // instead of falling back to the node's last scored epoch
    #[test]
    fn a_node_without_a_config_score_is_not_submitted() {
        let plan = plan(
            &[config_score(2, 1.0)],
            vec![
                aggregate(1, TestKind::MixnodeStress, 1.0),
                aggregate(2, TestKind::MixnodeStress, 1.0),
            ],
            &[mixnode(1), mixnode(2)],
        );

        assert_eq!(node_ids(&plan), vec![2]);
    }

    #[test]
    fn a_config_score_alone_is_not_submitted() {
        let plan = plan(
            &[config_score(1, 1.0), config_score(2, 1.0)],
            vec![aggregate(2, TestKind::MixnodeStress, 1.0)],
            &[mixnode(1), mixnode(2)],
        );

        assert_eq!(node_ids(&plan), vec![2]);
    }

    #[test]
    fn a_test_kind_below_the_threshold_is_omitted_for_every_node() {
        // stress reaches one mixnode in three; liveness reaches all of them, so every node stays
        let plan = plan(
            &[
                config_score(1, 1.0),
                config_score(2, 1.0),
                config_score(3, 1.0),
            ],
            vec![
                aggregate(1, TestKind::MixnodeLiveness, 1.0),
                aggregate(1, TestKind::MixnodeStress, 1.0),
                aggregate(2, TestKind::MixnodeLiveness, 1.0),
                aggregate(2, TestKind::MixnodeStress, 0.0),
                aggregate(3, TestKind::MixnodeLiveness, 1.0),
            ],
            &[mixnode(1), mixnode(2), mixnode(3)],
        );

        assert!(plan.coverage.stress.omitted);
        assert_eq!(node_ids(&plan), vec![1, 2, 3]);
        assert!(
            plan.nodes
                .iter()
                .all(|node| node.measurements.stress.is_none()),
            "an omitted test kind was still submitted for some node"
        );
    }

    // the scheduler never assigns mixnode liveness or stress to a gateway-only node, so it cannot
    // count against them
    #[test]
    fn a_gateway_only_node_is_not_eligible_for_mixnode_liveness_or_stress() {
        let plan = plan(
            &[],
            vec![
                aggregate(2, TestKind::MixnodeLiveness, 1.0),
                aggregate(2, TestKind::MixnodeStress, 1.0),
            ],
            &[gateway(1), mixnode(2)],
        );

        assert_eq!(plan.coverage.mixnode_liveness.coverage, 1.0);
        assert_eq!(plan.coverage.stress.coverage, 1.0);
    }

    #[test]
    fn a_zero_is_eligible_but_not_reachable() {
        let plan = plan(
            &[],
            vec![
                aggregate(1, TestKind::MixnodeStress, 0.0),
                aggregate(2, TestKind::MixnodeStress, 1.0),
            ],
            &[mixnode(1), mixnode(2)],
        );

        assert_eq!(plan.coverage.stress.coverage, 0.5);
    }

    // the two liveness probes are separate failure domains: a broken gateway probe must not drag
    // down the liveness a dual-role node earned through its mixnode runs
    #[test]
    fn an_omitted_gateway_liveness_leaves_dual_role_liveness_to_mixnode_runs_alone() {
        let plan = plan(
            &[
                config_score(1, 1.0),
                config_score(2, 1.0),
                config_score(3, 1.0),
            ],
            vec![
                aggregate(1, TestKind::GatewayLiveness, 0.0),
                aggregate(1, TestKind::MixnodeLiveness, 1.0),
                aggregate(2, TestKind::GatewayLiveness, 0.0),
                aggregate(3, TestKind::GatewayLiveness, 0.0),
            ],
            &[dual_role(1), gateway(2), gateway(3)],
        );

        assert!(plan.coverage.gateway_liveness.omitted);
        // combined with its failed gateway run it would read 0.5
        assert_eq!(measurements(&plan, 1).liveness, Some(pct(1.0)));
        // the gateway-only nodes have nothing left to submit
        assert_eq!(node_ids(&plan), vec![1]);
    }

    #[test]
    fn every_test_kind_omitted_leaves_an_empty_plan() {
        let plan = plan(
            &[config_score(1, 1.0), config_score(2, 1.0)],
            vec![
                aggregate(1, TestKind::MixnodeLiveness, 0.0),
                aggregate(1, TestKind::MixnodeStress, 0.0),
                aggregate(2, TestKind::GatewayLiveness, 0.0),
            ],
            &[mixnode(1), gateway(2)],
        );

        assert!(plan.coverage.mixnode_liveness.omitted);
        assert!(plan.coverage.gateway_liveness.omitted);
        assert!(plan.coverage.stress.omitted);
        assert!(plan.nodes.is_empty());
    }

    // 0 / 0 is NaN, and NaN is never below the threshold, so this pins the empty population as
    // omitted rather than silently kept
    #[test]
    fn an_empty_population_omits_its_test_kind() {
        let plan = plan(
            &[config_score(1, 1.0)],
            vec![aggregate(1, TestKind::MixnodeStress, 1.0)],
            &[mixnode(1)],
        );

        assert_eq!(
            plan.coverage.gateway_liveness,
            TestKindCoverage {
                coverage: 0.0,
                omitted: true,
            }
        );
    }

    // the contract rejects a batch whose node ids do not strictly ascend
    #[test]
    fn the_plan_is_in_ascending_node_order() {
        let plan = plan(
            &[
                config_score(3, 1.0),
                config_score(1, 1.0),
                config_score(2, 1.0),
            ],
            vec![
                aggregate(2, TestKind::MixnodeStress, 1.0),
                aggregate(3, TestKind::MixnodeStress, 1.0),
                aggregate(1, TestKind::MixnodeStress, 1.0),
            ],
            &[mixnode(3), mixnode(1), mixnode(2)],
        );

        assert_eq!(node_ids(&plan), vec![1, 2, 3]);
    }

    // a mean of perfect scores can land a hair above 1.0, which the percent conversion rejects
    #[test]
    fn a_value_a_hair_above_one_is_clamped() {
        let above_one = 1.0 + 1e-12;
        let plan = plan(
            &[config_score(1, above_one)],
            vec![aggregate(1, TestKind::MixnodeStress, above_one)],
            &[mixnode(1)],
        );

        assert_eq!(
            measurements(&plan, 1),
            Measurements::default()
                .with_config(pct(1.0))
                .with_stress(pct(1.0))
        );
    }

    mod unsent {
        use super::*;

        fn planned(node_ids: &[NodeId]) -> Vec<NodeSubmission> {
            node_ids
                .iter()
                .map(|&node_id| NodeSubmission {
                    node_id,
                    measurements: Measurements::default()
                        .with_config(pct(1.0))
                        .with_stress(pct(1.0)),
                })
                .collect()
        }

        fn cursor_at(epoch: EpochId, node_id: NodeId) -> NetworkMonitorSubmissionMetadata {
            NetworkMonitorSubmissionMetadata {
                last_submitted_epoch_id: epoch,
                last_submitted_node_id: node_id,
            }
        }

        fn unsent_ids(cursor: NetworkMonitorSubmissionMetadata) -> Vec<NodeId> {
            unsent(planned(&[1, 2, 3, 4]), FIXTURE_EPOCH, cursor)
                .unwrap()
                .iter()
                .map(|node| node.node_id)
                .collect()
        }

        // the cursor's node already landed, so resending it would fail the whole batch as stale
        #[test]
        fn a_cursor_in_the_same_epoch_drops_the_nodes_at_or_below_it() {
            assert_eq!(unsent_ids(cursor_at(FIXTURE_EPOCH, 2)), vec![3, 4]);
        }

        #[test]
        fn a_cursor_in_an_earlier_epoch_keeps_every_node() {
            assert_eq!(
                unsent_ids(cursor_at(FIXTURE_EPOCH - 1, 9)),
                vec![1, 2, 3, 4]
            );
        }

        // the contract has moved past this epoch, so anything sent for it would be rejected
        #[test]
        fn a_cursor_in_a_later_epoch_is_an_error() {
            assert!(
                unsent(
                    planned(&[1, 2, 3, 4]),
                    FIXTURE_EPOCH,
                    cursor_at(FIXTURE_EPOCH + 1, 0)
                )
                .is_err()
            );
        }
    }
}
