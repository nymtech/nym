// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_directory_attestation::SignedDigestSnapshot;
use nym_directory_client::verify::VerifiedDirectory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RawCachedDirectory {
    digest_snapshot: SignedDigestSnapshot,
    directory: VerifiedDirectory,
}

impl RawCachedDirectory {
    pub(crate) fn new(digest_snapshot: SignedDigestSnapshot, directory: VerifiedDirectory) -> Self {
        Self {
            digest_snapshot,
            directory,
        }
    }

    pub(crate) fn parse(&self) -> ParsedDirectory {
        // TODO (emit error logs on failures)
        ParsedDirectory {}
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ParsedDirectory {
    // structured data, like description, published keys, ports, etc.
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CachedDirectory {
    // required for proofs, hashes, etc.
    raw: RawCachedDirectory,

    parsed_directory: ParsedDirectory,
}

impl CachedDirectory {
    pub(crate) fn new(raw: RawCachedDirectory) -> Self {
        let parsed_directory = raw.parse();
        CachedDirectory {
            raw,
            parsed_directory,
        }
    }
}

impl From<CachedDirectory> for NymDirectoryCacheData {
    fn from(cached_directory: CachedDirectory) -> Self {
        NymDirectoryCacheData {
            directory: BTreeMap::from([(
                cached_directory.raw.digest_snapshot.snapshot.height,
                cached_directory,
            )]),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NymDirectoryCacheData {
    directory: BTreeMap<Height, CachedDirectory>,
}

impl NymDirectoryCacheData {
    pub(crate) fn insert_new(&mut self, update: CachedDirectory, retention_count: usize) {
        self.directory
            .insert(update.raw.digest_snapshot.snapshot.height, update);
        if self.directory.len() > retention_count {
            self.directory.pop_first();
        }
    }

    pub(crate) fn remove_stale(&mut self, to_retain: &[Height]) {
        self.directory
            .retain(|height, _| to_retain.contains(height));
    }

    pub(crate) fn contains_entry(&self, height: Height) -> bool {
        self.directory.contains_key(&height)
    }

    pub(crate) fn most_recent_height(&self) -> Option<Height> {
        self.directory.keys().last().copied()
    }
}
