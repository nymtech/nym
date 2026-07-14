// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::{
    DirectoryEntryRecord, DirectorySnapshotData, SignedDigestSnapshot,
};
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RawCachedDirectory {
    digest_snapshot: SignedDigestSnapshot,

    /// The raw entry set the digest was recomputed from - served verbatim by the
    /// whole-directory route, so a client can recompute the accumulator offline.
    records: Vec<DirectoryEntryRecord>,

    /// The node-identity map hashed into the snapshot's `node_identities_hash`.
    node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
}

impl RawCachedDirectory {
    pub(crate) fn new(
        digest_snapshot: SignedDigestSnapshot,
        records: Vec<DirectoryEntryRecord>,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Self {
        Self {
            digest_snapshot,
            records,
            node_identities,
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
    pub(crate) fn digest_snapshot(&self) -> &SignedDigestSnapshot {
        &self.raw.digest_snapshot
    }

    /// The whole-directory transfer payload served by the `/entries/{height}` route: the
    /// raw entry set + node-identity map, pinned to this snapshot's height, so a client
    /// can recompute the accumulator and node-identities hash offline against the
    /// (separately quorum'd) `SignedDigestSnapshot`.
    pub(crate) fn snapshot_data(&self) -> DirectorySnapshotData {
        DirectorySnapshotData {
            height: self.raw.digest_snapshot.snapshot.height,
            records: self.raw.records.clone(),
            node_identities: self.raw.node_identities.clone(),
        }
    }

    pub(crate) fn directory_records(&self) -> &[DirectoryEntryRecord] {
        &self.raw.records
    }

    pub(crate) fn directory_node_identities(&self) -> &BTreeMap<NodeId, ed25519::PublicKey> {
        &self.raw.node_identities
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DirectoryCacheUpdate {
    new_directory: Option<CachedDirectory>,
    polled_height: Height,
}

impl DirectoryCacheUpdate {
    pub(crate) fn new(new_directory: Option<CachedDirectory>, polled_height: Height) -> Self {
        DirectoryCacheUpdate {
            new_directory,
            polled_height,
        }
    }
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

impl From<DirectoryCacheUpdate> for NymDirectoryCacheData {
    fn from(update: DirectoryCacheUpdate) -> Self {
        let directory = match update.new_directory {
            None => BTreeMap::new(),
            Some(directory) => {
                BTreeMap::from([(directory.raw.digest_snapshot.snapshot.height, directory)])
            }
        };

        NymDirectoryCacheData {
            last_polled_height: update.polled_height,
            directory,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NymDirectoryCacheData {
    last_polled_height: Height,
    directory: BTreeMap<Height, CachedDirectory>,
}

impl NymDirectoryCacheData {
    pub(crate) fn update(&mut self, update: DirectoryCacheUpdate, retention_count: usize) {
        if let Some(new_entry) = update.new_directory {
            self.insert_entry(new_entry);
        }
        self.last_polled_height = update.polled_height;

        if self.directory.len() > retention_count {
            self.directory.pop_first();
        }
    }

    pub(crate) fn insert_entry(&mut self, directory: CachedDirectory) {
        if directory.raw.digest_snapshot.snapshot.height > self.last_polled_height {
            self.last_polled_height = directory.raw.digest_snapshot.snapshot.height;
        }

        self.directory
            .insert(directory.raw.digest_snapshot.snapshot.height, directory);
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

    pub(crate) fn most_recent_entry(&self, settle_lag: usize) -> Option<&CachedDirectory> {
        let allowed_latest = self
            .last_polled_height
            .value()
            .saturating_sub(settle_lag as u64);

        for (h, entry) in self.directory.iter().rev() {
            if h.value() <= allowed_latest {
                return Some(entry);
            }
        }
        None
    }

    pub(crate) fn get_entry(&self, height: Height) -> Option<&CachedDirectory> {
        self.directory.get(&height)
    }
}
