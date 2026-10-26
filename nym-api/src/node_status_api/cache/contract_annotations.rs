// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots::data::RetainedEntry;
use nym_api_requests::models::{
    ChainInteractionCapabilities, ConfigScoreV2, DetailedNodePerformanceV2, DisplayRole,
    LivenessScore, NodeAnnotationV2, RoutingScore, StressTestingScore,
};
use nym_contracts_common::{NaiveFloat, Percent};
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;

impl RetainedEntry<PerformanceSnapshotSource> {
    /// Contract mode's annotations, one for every bonded node with a committed entry.
    pub(crate) fn annotations(
        &self,
        rewarded_set: &CachedEpochRewardedSet,
    ) -> HashMap<NodeId, NodeAnnotationV2> {
        let auxiliary = self.auxiliary();

        self.bonded_entries()
            .map(|committed| {
                // an entry older than the epochs whose medians are held keeps default sub-scores
                let medians = auxiliary
                    .node_medians(committed.epoch_id, committed.node_id)
                    .unwrap_or_default();

                // reachable meaning a monitor collected samples of that kind
                let stress_testing_score = StressTestingScore {
                    score: medians
                        .stress
                        .map(|median| median.naive_to_f64())
                        .unwrap_or_default(),
                    was_reachable: medians.stress.is_some(),
                };
                let liveness_score = LivenessScore {
                    score: medians
                        .liveness
                        .map(|median| median.naive_to_f64())
                        .unwrap_or_default(),
                    was_reachable: medians.liveness.is_some(),
                };
                let detailed_performance = DetailedNodePerformanceV2::new(
                    committed
                        .score
                        .map(|score| score.naive_to_f64())
                        .unwrap_or_default(),
                    // the contract computes no routing figure
                    RoutingScore::zero(),
                    config_score(medians.config),
                    stress_testing_score,
                    liveness_score,
                );

                let annotation = NodeAnnotationV2 {
                    current_role: rewarded_set.role(committed.node_id).map(DisplayRole::from),
                    // contract mode does not query them
                    chain_interaction_capabilities: None,
                    detailed_performance,
                };
                (committed.node_id, annotation)
            })
            .collect()
    }
}

/// The contract holds only the config score, so its breakdown is inferred from it: a node scores
/// 0 unless it is described, runs nym-node and accepted the terms, and 1 only on the newest
/// release.
fn config_score(median: Option<Percent>) -> ConfigScoreV2 {
    match median {
        Some(median) if !median.is_zero() => ConfigScoreV2::new(
            median.naive_to_f64(),
            median.is_hundred().then_some(0),
            true,
            true,
            // a full score also means it can transact, but not whether by balance or feegrant
            ChainInteractionCapabilities::default(),
        ),
        _ => ConfigScoreV2::unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_performance::attestation::test_helpers::{
        committed, kind_medians, retained_entry,
    };
    use nym_performance_contract_common::{KindMedians, LatestPerformance};
    use std::collections::BTreeMap;

    /// Node 7's annotation from a snapshot holding `entry`, with `held` as node 7's medians at
    /// epochs 99 and 100.
    fn annotate(entry: LatestPerformance, held: KindMedians) -> NodeAnnotationV2 {
        let medians = BTreeMap::from([
            (99, BTreeMap::from([(7, held)])),
            (100, BTreeMap::from([(7, held)])),
        ]);
        let entry = retained_entry(1000, vec![entry], &[7], medians);
        entry
            .annotations(&CachedEpochRewardedSet::default())
            .remove(&7)
            .unwrap()
    }

    fn measured() -> KindMedians {
        kind_medians(Some(90), Some(80), Some(95))
    }

    #[test]
    fn performance_is_the_committed_score() {
        let annotation = annotate(committed(7, 100, Some(85)), measured());
        assert_eq!(annotation.detailed_performance.performance_score, 0.85);
    }

    #[test]
    fn stress_and_liveness_come_from_the_medians() {
        let performance = annotate(committed(7, 100, Some(85)), measured()).detailed_performance;

        assert_eq!(performance.stress_testing_score.score, 0.8);
        assert!(performance.stress_testing_score.was_reachable);
        assert_eq!(performance.liveness_score.score, 0.9);
        assert!(performance.liveness_score.was_reachable);
        assert_eq!(performance.routing_score.score, 0.0);
    }

    #[test]
    fn a_kind_without_a_median_is_zero_and_unreachable() {
        let gateway = kind_medians(Some(90), None, Some(95));
        let performance = annotate(committed(7, 100, Some(85)), gateway).detailed_performance;

        assert_eq!(performance.stress_testing_score.score, 0.0);
        assert!(!performance.stress_testing_score.was_reachable);
    }

    #[test]
    fn a_positive_config_score_marks_the_node_described_on_nym_node_with_terms_accepted() {
        let config = annotate(committed(7, 100, Some(85)), measured())
            .detailed_performance
            .config_score;

        assert_eq!(config.score, 0.95);
        assert!(config.self_described_api_available);
        assert!(config.runs_nym_node_binary);
        assert!(config.accepted_terms_and_conditions);
        assert_eq!(config.versions_behind, None);
    }

    #[test]
    fn a_full_config_score_means_no_versions_behind() {
        let full = kind_medians(Some(90), Some(80), Some(100));
        let config = annotate(committed(7, 100, Some(85)), full)
            .detailed_performance
            .config_score;

        assert_eq!(config.score, 1.0);
        assert_eq!(config.versions_behind, Some(0));
    }

    #[test]
    fn a_zero_config_score_is_served_as_unavailable() {
        let zero = kind_medians(Some(90), Some(80), Some(0));
        let config = annotate(committed(7, 100, None), zero)
            .detailed_performance
            .config_score;

        assert_eq!(config.score, 0.0);
        assert!(!config.self_described_api_available);
        assert!(!config.runs_nym_node_binary);
        assert!(!config.accepted_terms_and_conditions);
    }

    #[test]
    fn an_entry_from_an_older_epoch_keeps_default_sub_scores() {
        let performance = annotate(committed(7, 80, Some(70)), measured()).detailed_performance;

        assert_eq!(performance.performance_score, 0.7);
        assert_eq!(performance.config_score.score, 0.0);
        assert!(!performance.config_score.self_described_api_available);
        assert_eq!(performance.stress_testing_score.score, 0.0);
        assert!(!performance.stress_testing_score.was_reachable);
        assert_eq!(performance.liveness_score.score, 0.0);
        assert!(!performance.liveness_score.was_reachable);
    }

    #[test]
    fn an_unscoreable_entry_is_published_at_zero_with_its_medians() {
        let performance = annotate(committed(7, 100, None), measured()).detailed_performance;

        assert_eq!(performance.performance_score, 0.0);
        assert_eq!(performance.liveness_score.score, 0.9);
        assert_eq!(performance.config_score.score, 0.95);
    }

    #[test]
    fn only_bonded_nodes_with_a_committed_entry_are_annotated() {
        // 8 is bonded but never measured, 9 is measured but no longer bonded
        let records = vec![committed(7, 100, Some(85)), committed(9, 100, Some(85))];
        let entry = retained_entry(1000, records, &[7, 8], BTreeMap::new());

        let annotations = entry.annotations(&CachedEpochRewardedSet::default());
        assert_eq!(annotations.keys().copied().collect::<Vec<_>>(), [7]);
    }

    #[test]
    fn the_role_comes_from_the_rewarded_set() {
        let entry = retained_entry(
            1000,
            vec![committed(7, 100, Some(85))],
            &[7],
            BTreeMap::new(),
        );
        let rewarded_set = CachedEpochRewardedSet {
            layer1: [7].into(),
            ..Default::default()
        };

        let annotation = entry.annotations(&rewarded_set).remove(&7).unwrap();
        assert!(matches!(annotation.current_role, Some(DisplayRole::Layer1)));
        assert!(annotation.chain_interaction_capabilities.is_none());
    }
}
