// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::source::AttestedSnapshotSource;
use nym_contract_attestation::{SignedDigestSnapshot, SnapshotData};
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;

/// A signed snapshot with the record set and node identities it was built from.
// the fields keep the order of the directory's old `RawCachedDirectory`, so a directory cache
// file written before the move still decodes
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AttestedSnapshot<R> {
    digest_snapshot: SignedDigestSnapshot,

    /// The raw record set the digest was recomputed from - served verbatim by the
    /// whole-set route, so a client can recompute the accumulator offline.
    records: Vec<R>,

    /// The node-identity map hashed into the snapshot's `node_identities_hash`.
    node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
}

impl<R> AttestedSnapshot<R> {
    pub(crate) fn new(
        digest_snapshot: SignedDigestSnapshot,
        records: Vec<R>,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Self {
        Self {
            digest_snapshot,
            records,
            node_identities,
        }
    }

    pub(crate) fn height(&self) -> Height {
        self.digest_snapshot.snapshot.height
    }

    pub(crate) fn digest_snapshot(&self) -> &SignedDigestSnapshot {
        &self.digest_snapshot
    }

    pub(crate) fn records(&self) -> &[R] {
        &self.records
    }

    pub(crate) fn node_identities(&self) -> &BTreeMap<NodeId, ed25519::PublicKey> {
        &self.node_identities
    }

    /// The whole-set transfer payload: the raw record set + node-identity map, pinned to this
    /// snapshot's height, so a client can recompute the accumulator and node-identities hash
    /// offline against the (separately quorum'd) `SignedDigestSnapshot`.
    pub(crate) fn snapshot_data(&self) -> SnapshotData<R>
    where
        R: Clone,
    {
        SnapshotData {
            height: self.height(),
            records: self.records.clone(),
            node_identities: self.node_identities.clone(),
        }
    }
}

/// A retained snapshot with the source's auxiliary value, read at the same height but never
/// signed or served on the attestation routes.
// bincode encodes the nested snapshot exactly as its fields inline, and the directory's `()`
// auxiliary value as nothing, so this is byte-identical to the directory's old `CachedDirectory`
#[derive(Serialize, Deserialize)]
pub(crate) struct RetainedEntry<S: AttestedSnapshotSource> {
    snapshot: AttestedSnapshot<S::Record>,
    auxiliary: S::Auxiliary,
}

impl<S: AttestedSnapshotSource> RetainedEntry<S> {
    pub(crate) fn new(snapshot: AttestedSnapshot<S::Record>, auxiliary: S::Auxiliary) -> Self {
        Self {
            snapshot,
            auxiliary,
        }
    }

    pub(crate) fn snapshot(&self) -> &AttestedSnapshot<S::Record> {
        &self.snapshot
    }

    pub(crate) fn auxiliary(&self) -> &S::Auxiliary {
        &self.auxiliary
    }
}

// the trait bounds every field already; the derived bound would ask for `S: Serialize`
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub(crate) struct AttestedSnapshotsUpdate<S: AttestedSnapshotSource> {
    new_entry: Option<RetainedEntry<S>>,
    polled_height: Height,
}

impl<S: AttestedSnapshotSource> AttestedSnapshotsUpdate<S> {
    pub(crate) fn new(new_entry: Option<RetainedEntry<S>>, polled_height: Height) -> Self {
        AttestedSnapshotsUpdate {
            new_entry,
            polled_height,
        }
    }
}

impl<S: AttestedSnapshotSource> From<AttestedSnapshotsUpdate<S>> for AttestedSnapshotsCacheData<S> {
    fn from(update: AttestedSnapshotsUpdate<S>) -> Self {
        let entries = match update.new_entry {
            None => BTreeMap::new(),
            Some(entry) => BTreeMap::from([(entry.snapshot.height(), entry)]),
        };

        AttestedSnapshotsCacheData {
            last_polled_height: update.polled_height,
            entries,
        }
    }
}

// the trait bounds every field already; the derived bound would ask for `S: Serialize`
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub(crate) struct AttestedSnapshotsCacheData<S: AttestedSnapshotSource> {
    last_polled_height: Height,
    entries: BTreeMap<Height, RetainedEntry<S>>,
}

impl<S: AttestedSnapshotSource> Default for AttestedSnapshotsCacheData<S> {
    fn default() -> Self {
        AttestedSnapshotsCacheData {
            last_polled_height: Height::default(),
            entries: BTreeMap::new(),
        }
    }
}

impl<S: AttestedSnapshotSource> AttestedSnapshotsCacheData<S> {
    pub(crate) fn update(&mut self, update: AttestedSnapshotsUpdate<S>, retention_count: usize) {
        if let Some(new_entry) = update.new_entry {
            self.insert_entry(new_entry);
        }
        self.last_polled_height = update.polled_height;

        if self.entries.len() > retention_count {
            self.entries.pop_first();
        }
    }

    pub(crate) fn insert_entry(&mut self, entry: RetainedEntry<S>) {
        let height = entry.snapshot.height();
        if height > self.last_polled_height {
            self.last_polled_height = height;
        }

        self.entries.insert(height, entry);
    }

    /// Records the chain tip the cache was last reconciled against.
    pub(crate) fn set_last_polled_height(&mut self, height: Height) {
        self.last_polled_height = height;
    }

    pub(crate) fn remove_stale(&mut self, to_retain: &[Height]) {
        self.entries.retain(|height, _| to_retain.contains(height));
    }

    pub(crate) fn contains_entry(&self, height: Height) -> bool {
        self.entries.contains_key(&height)
    }

    pub(crate) fn most_recent_entry(&self, settle_lag: usize) -> Option<&RetainedEntry<S>> {
        let allowed_latest = self
            .last_polled_height
            .value()
            .saturating_sub(settle_lag as u64);

        for (h, entry) in self.entries.iter().rev() {
            if h.value() <= allowed_latest {
                return Some(entry);
            }
        }
        None
    }

    pub(crate) fn get_entry(&self, height: Height) -> Option<&RetainedEntry<S>> {
        self.entries.get(&height)
    }

    /// The newest retained entry below `height`.
    pub(crate) fn previous_entry(&self, height: Height) -> Option<&RetainedEntry<S>> {
        self.entries
            .range(..height)
            .next_back()
            .map(|(_, entry)| entry)
    }
}
