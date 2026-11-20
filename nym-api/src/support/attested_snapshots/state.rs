// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::{
    AttestedSnapshot, AttestedSnapshotsCacheData, RetainedHeight,
};
use crate::support::caching::cache::SharedCache;
use nym_contract_attestation::DirectoryEntryRecord;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_performance_contract_common::LatestPerformance;
use tendermint::block::Height;
use tokio::sync::RwLockReadGuard;

/// One contract's snapshot, read under the cache's guard.
type SnapshotGuard<'a, R> = RwLockReadGuard<'a, AttestedSnapshot<R>>;

/// The verified snapshots of every contract this nym-api attests, as the routes and the
/// annotations read them.
#[derive(Clone)]
pub(crate) struct ContractSnapshots {
    cache: SharedCache<AttestedSnapshotsCacheData>,

    /// Number of blocks to wait before promoting the most recently pulled snapshot as latest.
    settle_lag: usize,
}

impl ContractSnapshots {
    pub(crate) fn new(cache: SharedCache<AttestedSnapshotsCacheData>, settle_lag: usize) -> Self {
        ContractSnapshots { cache, settle_lag }
    }

    /// The latest settled entry, read under the cache's guard; `None` while none has settled.
    pub(crate) async fn latest_entry(&self) -> Option<RwLockReadGuard<'_, RetainedHeight>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| cache.most_recent_entry(self.settle_lag)).ok()
    }

    /// The entry retained at `height`, read under the cache's guard.
    async fn entry_at(&self, height: Height) -> Option<RwLockReadGuard<'_, RetainedHeight>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| cache.get_entry(height)).ok()
    }

    pub(crate) async fn latest_directory(&self) -> Option<SnapshotGuard<'_, DirectoryEntryRecord>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.directory))
    }

    pub(crate) async fn directory_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, DirectoryEntryRecord>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.directory))
    }

    pub(crate) async fn latest_performance(&self) -> Option<SnapshotGuard<'_, LatestPerformance>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            entry.performance.snapshot()
        }))
    }

    pub(crate) async fn performance_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, LatestPerformance>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            entry.performance.snapshot()
        }))
    }

    pub(crate) async fn latest_geolocation(&self) -> Option<SnapshotGuard<'_, GeolocationRecord>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.geolocation))
    }

    pub(crate) async fn geolocation_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, GeolocationRecord>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.geolocation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::data::test_helpers::empty_retained_height;

    const SETTLE_LAG: usize = 10;

    /// The snapshots at `heights`, polled `polled` blocks past the newest.
    fn polled_past_newest(heights: &[u32], polled: u32) -> ContractSnapshots {
        let mut data = AttestedSnapshotsCacheData::default();
        for &height in heights {
            data.insert_entry(empty_retained_height(height));
        }
        let newest = heights.iter().max().unwrap();
        data.set_last_polled_height(Height::from(newest + polled));
        ContractSnapshots::new(SharedCache::new_with_value(data), SETTLE_LAG)
    }

    #[tokio::test]
    async fn the_latest_snapshot_is_the_newest_settled_one_rather_than_a_newer_one() {
        // 1100 is polled 5 blocks past, short of the settle lag
        let snapshots = polled_past_newest(&[1000, 1100], 5);

        let latest = snapshots.latest_performance().await.unwrap();
        assert_eq!(latest.height(), Height::from(1000u32));
    }

    #[tokio::test]
    async fn there_is_no_latest_snapshot_while_none_has_settled() {
        let snapshots = polled_past_newest(&[1000], 5);

        assert!(snapshots.latest_performance().await.is_none());
    }

    #[tokio::test]
    async fn a_height_outside_the_window_has_no_snapshot() {
        let snapshots = polled_past_newest(&[1000], 5);

        assert!(snapshots
            .performance_at(Height::from(900u32))
            .await
            .is_none());
        assert!(snapshots
            .performance_at(Height::from(1000u32))
            .await
            .is_some());
    }
}
