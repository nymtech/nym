// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::cache::source::DirectorySource;
use crate::support::attested_snapshots::data::AttestedSnapshotsCacheData;
use crate::support::attested_snapshots::state::AttestedSnapshotsState;
use crate::support::caching::cache::SharedCache;
use nym_contract_attestation::{AttestedSubset, SignedSubsetDigest};

#[derive(Clone)]
pub struct DirectoryState {
    snapshots: AttestedSnapshotsState<DirectorySource>,
}

impl DirectoryState {
    pub(crate) fn new(
        cache: SharedCache<AttestedSnapshotsCacheData<DirectorySource>>,
        settle_lag: usize,
    ) -> Self {
        DirectoryState {
            snapshots: AttestedSnapshotsState::new(cache, settle_lag),
        }
    }

    pub(crate) fn snapshots(&self) -> &AttestedSnapshotsState<DirectorySource> {
        &self.snapshots
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
}
