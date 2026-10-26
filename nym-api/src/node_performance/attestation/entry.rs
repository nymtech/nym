// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots::data::RetainedEntry;
use nym_api_requests::models::performance::{DetailedNodePerformanceV3, NodePerformanceResponseV3};
use nym_performance_contract_common::{LatestPerformance, NodeId};

// the routing view of a retained performance snapshot: bonded nodes only
impl RetainedEntry<PerformanceSnapshotSource> {
    /// Every bonded node's committed entry, in node order.
    pub(crate) fn bonded_entries(&self) -> impl Iterator<Item = &LatestPerformance> + '_ {
        let snapshot = self.snapshot();
        snapshot
            .records()
            .iter()
            .filter(|committed| snapshot.node_identities().contains_key(&committed.node_id))
    }

    /// `node_id`'s committed entry, if it is bonded.
    pub(crate) fn bonded_entry(&self, node_id: NodeId) -> Option<&LatestPerformance> {
        let snapshot = self.snapshot();
        if !snapshot.node_identities().contains_key(&node_id) {
            return None;
        }

        // the records are in node order, as the source collects them from the verified map
        let records = snapshot.records();
        let index = records
            .binary_search_by_key(&node_id, |record| record.node_id)
            .ok()?;
        records.get(index)
    }

    /// `node_id`'s committed entry, with the medians held for its epoch.
    pub(crate) fn node_performance(&self, node_id: NodeId) -> NodePerformanceResponseV3 {
        let auxiliary = self.auxiliary();
        NodePerformanceResponseV3 {
            node_id,
            performance: self
                .bonded_entry(node_id)
                .map(|committed| DetailedNodePerformanceV3 {
                    performance_score: committed.score,
                    source_epoch: committed.epoch_id,
                    computed_at: auxiliary.block_time(),
                    medians: auxiliary.node_medians(committed.epoch_id, node_id),
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_performance::attestation::test_helpers::{
        block_time, committed, kind_medians, percent, retained_entry,
    };
    use nym_performance_contract_common::KindMedians;
    use std::collections::BTreeMap;

    fn measured() -> KindMedians {
        kind_medians(Some(90), Some(80), Some(95))
    }

    /// A snapshot at 1000 holding `records`, with 7 and 8 bonded and node 7's medians held at
    /// 99 and 100.
    fn entry(records: Vec<LatestPerformance>) -> RetainedEntry<PerformanceSnapshotSource> {
        let medians = BTreeMap::from([
            (99, BTreeMap::from([(7, measured())])),
            (100, BTreeMap::from([(7, measured())])),
        ]);
        retained_entry(1000, records, &[7, 8], medians)
    }

    #[test]
    fn a_measured_node_is_served_its_entry_and_medians() {
        let entry = entry(vec![committed(7, 100, Some(85))]);

        assert_eq!(
            entry.node_performance(7).performance,
            Some(DetailedNodePerformanceV3 {
                performance_score: Some(percent(85)),
                source_epoch: 100,
                computed_at: block_time(1000),
                medians: Some(measured()),
            })
        );
    }

    #[test]
    fn an_entry_from_an_older_epoch_is_served_without_medians() {
        let entry = entry(vec![committed(7, 80, Some(70))]);

        let performance = entry.node_performance(7).performance.unwrap();
        assert_eq!(performance.performance_score, Some(percent(70)));
        assert_eq!(performance.source_epoch, 80);
        assert_eq!(performance.medians, None);
    }

    #[test]
    fn an_unscoreable_entry_is_served_without_a_score() {
        let entry = entry(vec![committed(7, 100, None)]);

        let performance = entry.node_performance(7).performance.unwrap();
        assert_eq!(performance.performance_score, None);
        assert_eq!(performance.source_epoch, 100);
        assert_eq!(performance.medians, Some(measured()));
    }

    #[test]
    fn a_node_never_measured_has_no_performance() {
        // 8 is bonded with no committed entry
        let entry = entry(vec![committed(7, 100, Some(85))]);

        let served = entry.node_performance(8);
        assert_eq!(served.node_id, 8);
        assert_eq!(served.performance, None);
    }

    #[test]
    fn an_unbonded_node_is_left_out_of_the_routing_view() {
        // 9 has a committed entry but is not bonded
        let entry = entry(vec![
            committed(7, 100, Some(85)),
            committed(9, 100, Some(60)),
        ]);

        assert!(entry.bonded_entry(9).is_none());
        let bonded = entry
            .bonded_entries()
            .map(|committed| committed.node_id)
            .collect::<Vec<_>>();
        assert_eq!(bonded, vec![7]);
    }
}
