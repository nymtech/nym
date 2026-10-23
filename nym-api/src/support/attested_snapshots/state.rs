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

    pub(crate) async fn snapshot_at(&self, height: u64) -> Option<SignedDigestSnapshot> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height.try_into().ok()?)
            .map(|entry| entry.snapshot().digest_snapshot().clone())
    }

    /// One page of the records at `height`, `per_page` defaulting to 200 and capped at 500.
    pub(crate) async fn paged_entries_at(
        &self,
        height: u64,
        page: u32,
        per_page: Option<u32>,
    ) -> Option<PaginatedResponse<S::Record>> {
        let per_page = per_page
            .unwrap_or(DEFAULT_ENTRIES_PAGE_SIZE)
            .min(MAX_ENTRIES_PAGE_SIZE);
        let guard = self.cache.get().await.ok()?;

        let records = guard
            .get_entry(height.try_into().ok()?)?
            .snapshot()
            .records();
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
        height: u64,
    ) -> Option<BTreeMap<NodeId, ed25519::PublicKey>> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height.try_into().ok()?)
            .map(|entry| entry.snapshot().node_identities().clone())
    }
}
