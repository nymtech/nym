// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::{
    AttestedSnapshot, AttestedSnapshotsCacheData, RetainedHeight,
};
use crate::support::caching::cache::SharedCache;
use nym_contract_attestation::DirectoryEntryRecord;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_performance_contract_common::LatestPerformance;
use nym_task::cancellation::tracker::Cancelled;
use nym_task::ShutdownToken;
use std::time::Duration;
use tendermint::block::Height;
use tokio::sync::RwLockReadGuard;
use tokio::time::interval;
use tracing::info;

/// How often the startup wait checks the cache for a settled snapshot.
const WAIT_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// How often the startup wait logs that it is still waiting.
const WAIT_LOG_INTERVAL: Duration = Duration::from_secs(30);

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

    pub(crate) fn settle_lag(&self) -> usize {
        self.settle_lag
    }

    /// The latest settled entry, read under the cache's guard; `None` while none has settled.
    pub(crate) async fn latest_entry(&self) -> Option<RwLockReadGuard<'_, RetainedHeight>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| cache.most_recent_entry(self.settle_lag)).ok()
    }

    /// Returns once a snapshot has settled, one loaded from the file counting, logging while it
    /// waits; there is no time limit. Fails when shutdown is signalled first.
    pub(crate) async fn wait_until_settled(
        &self,
        shutdown_token: &ShutdownToken,
    ) -> Result<(), Cancelled> {
        // checks this api's own cache, which the producer fills
        let mut check = interval(WAIT_CHECK_INTERVAL);
        let mut log = interval(WAIT_LOG_INTERVAL);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => return Err(Cancelled),
                _ = check.tick() => {
                    if self.latest_entry().await.is_some() {
                        return Ok(());
                    }
                }
                _ = log.tick() => info!("waiting for a snapshot to settle"),
            }
        }
    }

    /// The entry retained at `height`, read under the cache's guard.
    async fn entry_at(&self, height: Height) -> Option<RwLockReadGuard<'_, RetainedHeight>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| cache.get_entry(height)).ok()
    }

    pub(crate) async fn latest_directory(&self) -> Option<SnapshotGuard<'_, DirectoryEntryRecord>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.reads.directory))
    }

    pub(crate) async fn directory_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, DirectoryEntryRecord>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| &entry.reads.directory))
    }

    pub(crate) async fn latest_performance(&self) -> Option<SnapshotGuard<'_, LatestPerformance>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            entry.reads.performance.snapshot()
        }))
    }

    pub(crate) async fn performance_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, LatestPerformance>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            entry.reads.performance.snapshot()
        }))
    }

    pub(crate) async fn latest_geolocation(&self) -> Option<SnapshotGuard<'_, GeolocationRecord>> {
        let entry = self.latest_entry().await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            &entry.reads.geolocation
        }))
    }

    pub(crate) async fn geolocation_at(
        &self,
        height: Height,
    ) -> Option<SnapshotGuard<'_, GeolocationRecord>> {
        let entry = self.entry_at(height).await?;
        Some(RwLockReadGuard::map(entry, |entry| {
            &entry.reads.geolocation
        }))
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::support::attested_snapshots::data::HeightReads;

    /// Snapshots whose one height holds `reads`, settled.
    pub(crate) fn settled_snapshots(reads: HeightReads) -> ContractSnapshots {
        let entry = RetainedHeight::new(reads, false);
        let mut data = AttestedSnapshotsCacheData::default();
        data.set_last_polled_height(entry.height());
        data.insert_entry(entry);
        ContractSnapshots::new(SharedCache::new_with_value(data), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::data::test_helpers::empty_retained_height;
    use tokio::time::sleep;

    const SETTLE_LAG: usize = 10;

    /// The snapshots at `heights`, polled `polled` blocks past the newest.
    fn polled_past_newest(heights: &[u32], polled: u32) -> ContractSnapshots {
        ContractSnapshots::new(
            SharedCache::new_with_value(retained(heights, polled)),
            SETTLE_LAG,
        )
    }

    /// The cache holding `heights`, polled `polled` blocks past the newest.
    fn retained(heights: &[u32], polled: u32) -> AttestedSnapshotsCacheData {
        let mut data = AttestedSnapshotsCacheData::default();
        for &height in heights {
            data.insert_entry(empty_retained_height(height));
        }
        let newest = heights.iter().max().unwrap();
        data.set_last_polled_height(Height::from(newest + polled));
        data
    }

    #[tokio::test(start_paused = true)]
    async fn the_wait_returns_once_a_snapshot_has_settled_and_not_before() {
        let cache = SharedCache::new();
        let snapshots = ContractSnapshots::new(cache.clone(), SETTLE_LAG);
        let wait =
            tokio::spawn(async move { snapshots.wait_until_settled(&ShutdownToken::new()).await });

        // produced, but the tip is still within the settle lag
        let produced = cache.try_overwrite_old_value(retained(&[1000], 5), "test");
        assert!(produced.await.is_ok());
        sleep(Duration::from_secs(60)).await;
        assert!(!wait.is_finished());

        let settled = cache.try_overwrite_old_value(retained(&[1000], SETTLE_LAG as u32), "test");
        assert!(settled.await.is_ok());
        sleep(2 * WAIT_CHECK_INTERVAL).await;
        assert!(wait.is_finished());
        assert!(wait.await.unwrap().is_ok());
    }

    /// As after loading a file that holds a settled snapshot.
    #[tokio::test(start_paused = true)]
    async fn a_settled_snapshot_already_held_ends_the_wait_at_once() {
        let snapshots = polled_past_newest(&[1000], SETTLE_LAG as u32);

        let start = tokio::time::Instant::now();
        let waited = snapshots.wait_until_settled(&ShutdownToken::new()).await;
        assert!(waited.is_ok());
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn a_shutdown_ends_the_wait() {
        let shutdown = ShutdownToken::new();
        let snapshots = ContractSnapshots::new(SharedCache::new(), SETTLE_LAG);
        let wait = tokio::spawn({
            let shutdown = shutdown.clone();
            async move { snapshots.wait_until_settled(&shutdown).await }
        });

        sleep(Duration::from_secs(60)).await;
        shutdown.cancel();
        assert!(wait.await.unwrap().is_err());
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
