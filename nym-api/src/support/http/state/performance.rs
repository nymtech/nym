// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots::state::AttestedSnapshotsState;
use nym_api_requests::models::performance::{DetailedNodePerformanceV3, NodePerformanceResponseV3};
use nym_performance_contract_common::{EpochWeights, NodeId};

impl AttestedSnapshotsState<PerformanceSnapshotSource> {
    /// `node_id`'s committed entry in the latest settled snapshot; `None` while none has settled.
    pub(crate) async fn latest_node_performance(
        &self,
        node_id: NodeId,
    ) -> Option<NodePerformanceResponseV3> {
        let entry = self.latest_entry().await?;
        let snapshot = entry.snapshot();
        let auxiliary = entry.auxiliary();

        // the records are in node order, as the source collects them from the verified map
        let records = snapshot.records();
        let committed = match records.binary_search_by_key(&node_id, |record| record.node_id) {
            // bonded nodes only, as the routing view
            Ok(index) if snapshot.node_identities().contains_key(&node_id) => records.get(index),
            _ => None,
        };

        Some(NodePerformanceResponseV3 {
            node_id,
            performance: committed.map(|committed| DetailedNodePerformanceV3 {
                performance_score: committed.score,
                source_epoch: committed.epoch_id,
                computed_at: auxiliary.block_time(),
                medians: auxiliary.node_medians(committed.epoch_id, node_id),
            }),
        })
    }

    /// The weights in force at the latest settled snapshot's height; `None` while none has settled.
    pub(crate) async fn latest_weights(&self) -> Option<EpochWeights> {
        Some(self.latest_entry().await?.auxiliary().weights())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_performance::attestation::test_helpers::{
        block_time, committed, kind_medians, percent, retained_entry, weights,
    };
    use crate::support::attested_snapshots::data::{AttestedSnapshotsCacheData, RetainedEntry};
    use crate::support::caching::cache::SharedCache;
    use nym_performance_contract_common::{KindMedians, LatestPerformance};
    use std::collections::BTreeMap;
    use tendermint::block::Height;

    const SETTLE_LAG: usize = 10;

    /// The state over `entries`, polled `polled` blocks past the newest.
    fn state(
        entries: Vec<RetainedEntry<PerformanceSnapshotSource>>,
        polled: u64,
    ) -> AttestedSnapshotsState<PerformanceSnapshotSource> {
        let mut data = AttestedSnapshotsCacheData::default();
        for entry in entries {
            data.insert_entry(entry);
        }
        let newest = data.most_recent_entry(0).unwrap().snapshot().height();
        data.set_last_polled_height(Height::try_from(newest.value() + polled).unwrap());
        AttestedSnapshotsState::new(SharedCache::new_with_value(data), SETTLE_LAG)
    }

    fn measured() -> KindMedians {
        kind_medians(Some(90), Some(80), Some(95))
    }

    /// A settled snapshot at 1000 holding node 7's `entry`, with node 7's medians held at 99
    /// and 100.
    fn settled(entry: LatestPerformance) -> AttestedSnapshotsState<PerformanceSnapshotSource> {
        let medians = BTreeMap::from([
            (99, BTreeMap::from([(7, measured())])),
            (100, BTreeMap::from([(7, measured())])),
        ]);
        state(
            vec![retained_entry(1000, vec![entry], &[7, 8], medians)],
            10,
        )
    }

    #[tokio::test]
    async fn a_measured_node_is_served_its_entry_and_medians() {
        let state = settled(committed(7, 100, Some(85)));

        let served = state.latest_node_performance(7).await.unwrap();
        assert_eq!(
            served.performance,
            Some(DetailedNodePerformanceV3 {
                performance_score: Some(percent(85)),
                source_epoch: 100,
                computed_at: block_time(1000),
                medians: Some(measured()),
            })
        );
    }

    #[tokio::test]
    async fn an_entry_from_an_older_epoch_is_served_without_medians() {
        let state = settled(committed(7, 80, Some(70)));

        let performance = state.latest_node_performance(7).await.unwrap().performance;
        let performance = performance.unwrap();
        assert_eq!(performance.performance_score, Some(percent(70)));
        assert_eq!(performance.source_epoch, 80);
        assert_eq!(performance.medians, None);
    }

    #[tokio::test]
    async fn an_unscoreable_entry_is_served_without_a_score() {
        let state = settled(committed(7, 100, None));

        let performance = state.latest_node_performance(7).await.unwrap().performance;
        let performance = performance.unwrap();
        assert_eq!(performance.performance_score, None);
        assert_eq!(performance.source_epoch, 100);
        assert_eq!(performance.medians, Some(measured()));
    }

    #[tokio::test]
    async fn a_node_never_measured_has_no_performance() {
        // 8 is bonded with no committed entry
        let state = settled(committed(7, 100, Some(85)));

        let served = state.latest_node_performance(8).await.unwrap();
        assert_eq!(served.node_id, 8);
        assert_eq!(served.performance, None);
    }

    #[tokio::test]
    async fn the_latest_settled_snapshot_is_served_rather_than_a_newer_one() {
        let older = retained_entry(
            1000,
            vec![committed(7, 100, Some(85))],
            &[7],
            BTreeMap::new(),
        );
        let newer = retained_entry(
            1100,
            vec![committed(7, 101, Some(60))],
            &[7],
            BTreeMap::new(),
        );
        // 1100 is polled 5 blocks past, short of the settle lag
        let state = state(vec![older, newer], 5);

        let performance = state.latest_node_performance(7).await.unwrap().performance;
        let performance = performance.unwrap();
        assert_eq!(performance.source_epoch, 100);
        assert_eq!(performance.computed_at, block_time(1000));
    }

    #[tokio::test]
    async fn nothing_is_served_while_no_snapshot_has_settled() {
        let entry = retained_entry(
            1000,
            vec![committed(7, 100, Some(85))],
            &[7],
            BTreeMap::new(),
        );
        let state = state(vec![entry], 5);

        assert_eq!(state.latest_node_performance(7).await, None);
        assert_eq!(state.latest_weights().await, None);
    }

    #[tokio::test]
    async fn the_weights_of_the_latest_settled_snapshot_are_served() {
        let state = settled(committed(7, 100, Some(85)));

        assert_eq!(state.latest_weights().await, Some(weights()));
    }
}
