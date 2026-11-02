// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::AttestedSnapshot;
use nym_contract_attestation::{AttestedSubset, DirectoryEntryRecord, SignedSubsetDigest};

impl AttestedSnapshot<DirectoryEntryRecord> {
    pub(crate) fn subset_digest(&self, subset_id: &str) -> Option<SignedSubsetDigest> {
        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        None
    }

    pub(crate) fn subset(&self, subset_id: &str) -> Option<AttestedSubset> {
        // parse subset id into some well-defined enum and then either build data from scratch
        // (if cheap) or use the cached data (if expensive)
        let _ = subset_id;
        None
    }
}
