// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::support::attested_snapshots::state::ContractSnapshots;
use crate::support::caching::refresher::{RefreshRequester, FIRST_BUILD_RETRY_INTERVAL};
use crate::support::caching::CacheNotificationWatcher;
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache, node_status_api::cache::NodeStatusCacheError,
};
use ::time::OffsetDateTime;
use nym_task::ShutdownToken;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{info, trace, warn};

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    /// Interval of the refreshes nothing requested, and the least time between notified ones.
    fallback_caching_interval: Duration,

    // Main stored data
    cache: NodeStatusCache,

    // Sources for when refreshing data
    mixnet_contract_cache: MixnetContractCache,

    /// The source of the annotations.
    contract_snapshots: ContractSnapshots,

    /// channel notifying us when mixnet cache has been refreshed,
    /// so that this cache could also be recreated
    mixnet_contract_cache_listener: CacheNotificationWatcher,

    /// channel explicitly requesting cache refresh. it does not follow the usual rate limiting
    refresh_requester: RefreshRequester,

    /// Path to an on-disk location where the contents of the retrieved items should be written
    /// upon refresh
    on_disk_file: PathBuf,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: MixnetContractCache,
        contract_snapshots: ContractSnapshots,
        contract_cache_listener: CacheNotificationWatcher,
        on_disk_file: PathBuf,
    ) -> Self {
        Self {
            fallback_caching_interval,
            cache,
            mixnet_contract_cache: contract_cache,
            contract_snapshots,
            mixnet_contract_cache_listener: contract_cache_listener,
            refresh_requester: Default::default(),
            on_disk_file,
        }
    }

    pub(crate) fn refresh_requester(&self) -> RefreshRequester {
        self.refresh_requester.clone()
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        // `None` until the first build, which is therefore not held back by the rate limit
        let mut last_update = None;
        let mut fallback_interval = time::interval(self.fallback_caching_interval);
        loop {
            let requested = tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => break,
                // Update node status cache when the contract cache is updated
                Ok(_) = self.mixnet_contract_cache_listener.changed() => false,
                // note: `Notify` is not cancellation safe, HOWEVER, there's only one listener,
                // so it doesn't matter if we lose our queue position
                _ = self.refresh_requester.notified() => true,
                // ... however, if we don't receive any notifications we fall back to periodic
                // refreshes
                _ = fallback_interval.tick() => false,
            };

            let refresh = async {
                if requested {
                    // perform full refresh regardless of the rates
                    self.refresh_and_reschedule(&mut fallback_interval, &mut last_update)
                        .await
                } else {
                    self.maybe_refresh(&mut fallback_interval, &mut last_update)
                        .await
                }
            };
            tokio::select! {
                _ = refresh => {}
                _ = shutdown_token.cancelled() => break,
            }
        }
        trace!("NodeStatusCacheRefresher: Received shutdown");
        info!("NodeStatusCacheRefresher: Exiting");
    }

    /// Whether the caches the annotations are built from hold values, a persisted value counting:
    /// the mixnet contract cache and a settled performance snapshot.
    async fn caches_available(&self) -> bool {
        let contract_cache = self.mixnet_contract_cache.inner().get().await.is_ok();
        let performance_snapshot = self.contract_snapshots.latest_entry().await.is_some();

        let available = contract_cache && performance_snapshot;
        if !available {
            warn!(
                contract_cache,
                performance_snapshot, "auxiliary caches data is not yet available"
            )
        }

        available
    }

    async fn maybe_refresh(
        &mut self,
        fallback_interval: &mut time::Interval,
        last_updated: &mut Option<OffsetDateTime>,
    ) {
        if let Some(last) = *last_updated {
            if OffsetDateTime::now_utc() - last < self.fallback_caching_interval {
                // don't update too often
                trace!("not updating the cache since they've been updated recently");
                return;
            }
        }

        self.refresh_and_reschedule(fallback_interval, last_updated)
            .await
    }

    /// Refreshes once the caches it reads hold values, retrying in 30 s until the first success.
    async fn refresh_and_reschedule(
        &mut self,
        fallback_interval: &mut time::Interval,
        last_updated: &mut Option<OffsetDateTime>,
    ) {
        let refreshed = self.caches_available().await && self.refresh().await.is_ok();
        if refreshed || last_updated.is_some() {
            *last_updated = Some(OffsetDateTime::now_utc());
            fallback_interval.reset();
        } else {
            // until the first build, retry soon rather than a whole interval later: the
            // performance snapshots notify nothing when one settles
            fallback_interval.reset_after(FIRST_BUILD_RETRY_INTERVAL);
        }
    }

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    async fn refresh(&mut self) -> Result<(), NodeStatusCacheError> {
        info!("Updating node status cache");

        // callers act only on success, so this is the only place a failure is reported
        if let Err(err) = self.update_node_annotations().await {
            warn!("failed to update the node annotations: {err}");
            return Err(err);
        }

        // attempt to update on-disk cache
        let new_cached = self.cache.cache().await?;
        // error reporting is handled by the serialise function itself
        let _ = new_cached.try_serialise_to_file(&self.on_disk_file);

        Ok(())
    }

    /// Replaces the cached annotations with the performance contract's figure for every bonded
    /// node it holds an entry for, from the latest settled performance snapshot; an error while
    /// none has settled, so the map is left as it is.
    async fn update_node_annotations(&mut self) -> Result<(), NodeStatusCacheError> {
        let rewarded_set = self.mixnet_contract_cache.rewarded_set_owned().await?;
        let entry = self
            .contract_snapshots
            .latest_entry()
            .await
            .ok_or(NodeStatusCacheError::NoSettledPerformanceSnapshot)?;

        self.cache.update(entry.annotations(&rewarded_set)).await
    }
}
