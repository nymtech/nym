// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::cache::data::NymDirectoryCacheData;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{
    AttestedSubset, DirectoryEntryRecord, DirectorySnapshotData, SignedDigestSnapshot,
    SignedSubsetDigest,
};
use nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct DirectoryState {
    cache: SharedCache<NymDirectoryCacheData>,

    /// Number of blocks to wait before promoting the most recently pulled snapshot as latest.
    settle_lag: usize,
}

impl DirectoryState {
    pub(crate) fn new(cache: SharedCache<NymDirectoryCacheData>, settle_lag: usize) -> Self {
        DirectoryState { cache, settle_lag }
    }

    pub(crate) async fn latest_snapshot(&self) -> Option<SignedDigestSnapshot> {
        self.cache
            .get()
            .await
            .ok()?
            .most_recent_entry(self.settle_lag)
            .map(|entry| entry.digest_snapshot().clone())
    }

    pub(crate) async fn snapshot_at(&self, height: u64) -> Option<SignedDigestSnapshot> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height.try_into().ok()?)
            .map(|entry| entry.digest_snapshot().clone())
    }

    pub(crate) async fn directory_subset_digest(
        &self,
        subset_id: &str,
        height: u64,
    ) -> Option<SignedSubsetDigest> {
        // let entry = self
        //     .cache
        //     .get()
        //     .await
        //     .ok()?
        //     .get_entry(height.try_into().ok()?)?;

        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        let _ = height;
        None
    }

    pub(crate) async fn directory_subset(
        &self,
        subset_id: &str,
        height: u64,
    ) -> Option<AttestedSubset> {
        // let entry = self
        //     .cache
        //     .get()
        //     .await
        //     .ok()?
        //     .get_entry(height.try_into().ok()?)?;

        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        let _ = height;
        None
    }

    /// The whole-directory transfer payload at an explicit height (by height only - the
    /// client drives this with the height it already quorum'd a snapshot for, so there is
    /// no settle-lag skew to reconcile). `None` if the height is outside the retained window.
    #[allow(dead_code)]
    // TODO: perhaps expose it under some internal, helper, endpoint?
    pub(crate) async fn entries_at(&self, height: u64) -> Option<DirectorySnapshotData> {
        self.cache
            .get()
            .await
            .ok()?
            .get_entry(height.try_into().ok()?)
            .map(|entry| entry.snapshot_data())
    }

    pub(crate) async fn paged_entries_at(
        &self,
        height: u64,
        page: u32,
        per_page: u32,
    ) -> Option<PaginatedResponse<DirectoryEntryRecord>> {
        let guard = self.cache.get().await.ok()?;

        let entry = guard.get_entry(height.try_into().ok()?)?;
        let total = entry.directory_records().len();
        let offset = (page as usize).saturating_mul(per_page as usize);

        let data = entry
            .directory_records()
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
            .map(|entry| entry.directory_node_identities().clone())
    }
}
