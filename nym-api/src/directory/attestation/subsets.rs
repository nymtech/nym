// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::attestation::source::DirectorySnapshotSource;
use crate::support::attested_snapshots::state::AttestedSnapshotsState;
use nym_contract_attestation::{AttestedSubset, SignedSubsetDigest};
use tendermint::block::Height;

impl AttestedSnapshotsState<DirectorySnapshotSource> {
    pub(crate) async fn directory_subset_digest(
        &self,
        subset_id: &str,
        height: Height,
    ) -> Option<SignedSubsetDigest> {
        // let entry = self
        //     .cache
        //     .get()
        //     .await
        //     .ok()?
        //     .get_entry(height)?;

        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        let _ = height;
        None
    }

    pub(crate) async fn directory_subset(
        &self,
        subset_id: &str,
        height: Height,
    ) -> Option<AttestedSubset> {
        // let entry = self
        //     .cache
        //     .get()
        //     .await
        //     .ok()?
        //     .get_entry(height)?;

        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        let _ = height;
        None
    }
}
