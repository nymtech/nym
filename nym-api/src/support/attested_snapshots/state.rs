// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::{AttestedSnapshotsCacheData, RetainedEntry};
use crate::support::attested_snapshots::source::AttestedSnapshotSource;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_contract_attestation::SignedDigestSnapshot;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;
use tendermint::block::Height;
use tokio::sync::RwLockReadGuard;

const DEFAULT_ENTRIES_PAGE_SIZE: u32 = 200;
const MAX_ENTRIES_PAGE_SIZE: u32 = 500;

pub(crate) struct AttestedSnapshotsState<S: AttestedSnapshotSource> {
    cache: SharedCache<AttestedSnapshotsCacheData<S>>,

    /// Number of blocks to wait before promoting the most recently pulled snapshot as latest.
    settle_lag: usize,
}

// derived `Clone` would ask for `S: Clone`
impl<S: AttestedSnapshotSource> Clone for AttestedSnapshotsState<S> {
    fn clone(&self) -> Self {
        AttestedSnapshotsState {
            cache: self.cache.clone(),
            settle_lag: self.settle_lag,
        }
    }
}

impl<S: AttestedSnapshotSource> AttestedSnapshotsState<S> {
    pub(crate) fn new(
        cache: SharedCache<AttestedSnapshotsCacheData<S>>,
        settle_lag: usize,
    ) -> Self {
        AttestedSnapshotsState { cache, settle_lag }
    }

    /// The latest settled entry, read under the cache's guard; `None` while none has settled.
    pub(crate) async fn latest_entry(&self) -> Option<RwLockReadGuard<'_, RetainedEntry<S>>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| cache.most_recent_entry(self.settle_lag)).ok()
    }

    pub(crate) async fn latest_snapshot(&self) -> Option<SignedDigestSnapshot> {
        let entry = self.latest_entry().await?;
        Some(entry.snapshot().digest_snapshot().clone())
    }

    pub(crate) async fn snapshot_at(&self, height: Height) -> Option<SignedDigestSnapshot> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height)
            .map(|entry| entry.snapshot().digest_snapshot().clone())
    }

    /// One page of the records at `height`, `per_page` defaulting to 200 and capped at 500.
    pub(crate) async fn paged_entries_at(
        &self,
        height: Height,
        page: u32,
        per_page: Option<u32>,
    ) -> Option<PaginatedResponse<S::Record>> {
        let per_page = per_page
            .unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
            .min(MAX_ENTRIES_PAGE_SIZE);
        let guard = self.cache.get().await.ok()?;

        let records = guard.get_entry(height)?.snapshot().records();
        let total = records.len();
        let offset = (page as usize).saturating_mul(per_page as usize);

        let data = records
            .iter()
            .skip(offset)
            .take(per_page as usize)
            .cloned()
            .collect::<Vec<_>>();

        Some(PaginatedResponse {
            pagination: Pagination {
                total,
                page,
                size: data.len(),
            },
            data,
        })
    }

    pub(crate) async fn node_identities_at(
        &self,
        height: Height,
    ) -> Option<BTreeMap<NodeId, ed25519::PublicKey>> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height)
            .map(|entry| entry.snapshot().node_identities().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_performance::attestation::source::PerformanceSnapshotSource;
    use crate::node_performance::attestation::test_helpers::{committed, retained_entry};

    /// A settled snapshot at 1000 holding 600 records.
    fn six_hundred_records() -> AttestedSnapshotsState<PerformanceSnapshotSource> {
        let records = (1..=600)
            .map(|node_id| committed(node_id, 100, Some(50)))
            .collect();
        let mut data = AttestedSnapshotsCacheData::default();
        data.insert_entry(retained_entry(1000, records, &[], BTreeMap::new()));
        AttestedSnapshotsState::new(SharedCache::new_with_value(data), 0)
    }

    #[tokio::test]
    async fn records_are_paged_by_200_by_default_and_by_at_most_500() {
        let state = six_hundred_records();
        let height = Height::from(1000u32);

        let default = state.paged_entries_at(height, 0, None).await.unwrap();
        assert_eq!(default.data.len(), 200);
        assert_eq!(default.pagination.total, 600);

        let capped = state.paged_entries_at(height, 0, Some(1000)).await.unwrap();
        assert_eq!(capped.data.len(), 500);

        let last = state.paged_entries_at(height, 2, None).await.unwrap();
        assert_eq!(last.data.first().map(|record| record.node_id), Some(401));
        assert_eq!(last.data.len(), 200);
    }

    #[tokio::test]
    async fn a_height_outside_the_window_has_no_records() {
        let state = six_hundred_records();

        let outside = Height::from(900u32);
        assert!(state.paged_entries_at(outside, 0, None).await.is_none());
    }

    const SETTLE_LAG: usize = 10;

    /// The state over snapshots at `heights`, polled `polled` blocks past the newest.
    fn polled_past_newest(
        heights: &[u32],
        polled: u32,
    ) -> AttestedSnapshotsState<PerformanceSnapshotSource> {
        let mut data = AttestedSnapshotsCacheData::default();
        for &height in heights {
            data.insert_entry(retained_entry(height, Vec::new(), &[], BTreeMap::new()));
        }
        let newest = heights.iter().max().unwrap();
        data.set_last_polled_height(Height::from(newest + polled));
        AttestedSnapshotsState::new(SharedCache::new_with_value(data), SETTLE_LAG)
    }

    #[tokio::test]
    async fn the_latest_entry_is_the_newest_settled_one_rather_than_a_newer_one() {
        // 1100 is polled 5 blocks past, short of the settle lag
        let state = polled_past_newest(&[1000, 1100], 5);

        let latest = state.latest_entry().await.unwrap();
        assert_eq!(latest.snapshot().height(), Height::from(1000u32));
    }

    #[tokio::test]
    async fn there_is_no_latest_entry_while_none_has_settled() {
        let state = polled_past_newest(&[1000], 5);

        assert!(state.latest_entry().await.is_none());
    }
}
