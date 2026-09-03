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

#[derive(Debug, Serialize, Deserialize, Default)]
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

    /// Records the chain tip the cache was last reconciled against.
    pub(crate) fn set_last_polled_height(&mut self, height: Height) {
        self.last_polled_height = height;
    }

    pub(crate) fn remove_stale(&mut self, to_retain: &[Height]) {
        self.directory
            .retain(|height, _| to_retain.contains(height));
    }

    pub(crate) fn contains_entry(&self, height: Height) -> bool {
        self.directory.contains_key(&height)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use nym_directory_attestation::source::mock::mock_digest_snapshot;
    use nym_directory_contract_common::CuratedEntry;
    use rand_chacha::rand_core::SeedableRng;

    fn signed_snapshot(height: u32) -> SignedDigestSnapshot {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(height as u64);
        let kp = ed25519::KeyPair::new(&mut rng);
        mock_digest_snapshot(Height::from(height)).signed(&kp)
    }

    fn cached_directory(height: u32) -> CachedDirectory {
        CachedDirectory::new(RawCachedDirectory::new(
            signed_snapshot(height),
            Vec::new(),
            BTreeMap::new(),
        ))
    }

    /// A cached directory carrying actual content. Empty `records` / `node_identities`
    /// never reach their element encoders, so only a populated value can exercise them.
    fn populated_cached_directory(height: u32) -> CachedDirectory {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(height as u64);
        let kp = ed25519::KeyPair::new(&mut rng);

        let records = vec![DirectoryEntryRecord::new_curated(
            "nym-api-1".to_string(),
            CuratedEntry::try_from_bytes(b"curated-payload").unwrap(),
        )];

        CachedDirectory::new(RawCachedDirectory::new(
            signed_snapshot(height),
            records,
            BTreeMap::from([(1, *kp.public_key())]),
        ))
    }

    fn empty_cache(last_polled: u32) -> NymDirectoryCacheData {
        NymDirectoryCacheData {
            last_polled_height: Height::from(last_polled),
            directory: BTreeMap::new(),
        }
    }

    #[test]
    fn update_prunes_to_the_retention_count() {
        let mut cache = empty_cache(0);
        for h in [100u32, 200, 300, 400] {
            cache.update(
                DirectoryCacheUpdate::new(Some(cached_directory(h)), Height::from(h)),
                3,
            );
        }
        // once the window exceeds the retention count the oldest snapshot is dropped
        assert_eq!(cache.directory.len(), 3);
        assert!(cache.get_entry(Height::from(100u32)).is_none());
        assert!(cache.get_entry(Height::from(200u32)).is_some());
        assert!(cache.get_entry(Height::from(400u32)).is_some());
    }

    #[test]
    fn remove_stale_retains_only_the_listed_heights() {
        let mut cache = empty_cache(300);
        for h in [100u32, 200, 300] {
            cache.insert_entry(cached_directory(h));
        }
        cache.remove_stale(&[Height::from(200u32), Height::from(300u32)]);
        assert_eq!(cache.directory.len(), 2);
        assert!(cache.get_entry(Height::from(100u32)).is_none());
        assert!(cache.get_entry(Height::from(300u32)).is_some());
    }

    #[test]
    fn most_recent_entry_applies_the_settle_lag() {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 950] {
            cache.insert_entry(cached_directory(h));
        }
        // the chain tip is ahead of the newest snapshot
        cache.last_polled_height = Height::from(1000u32);

        // lag 60 -> allowed latest 940 -> newest snapshot at or below is 900
        assert_eq!(
            cache
                .most_recent_entry(60)
                .unwrap()
                .digest_snapshot()
                .snapshot
                .height,
            Height::from(900u32)
        );
        // a smaller lag lets the newest snapshot through
        assert_eq!(
            cache
                .most_recent_entry(10)
                .unwrap()
                .digest_snapshot()
                .snapshot
                .height,
            Height::from(950u32)
        );
    }

    #[test]
    fn recording_the_tip_after_warmup_makes_the_newest_snapshot_servable() {
        let mut cache = empty_cache(0);
        cache.insert_entry(cached_directory(900));
        // insert alone leaves last_polled at the snapshot height, so the settle lag hides it
        assert!(cache.most_recent_entry(60).is_none());

        cache.set_last_polled_height(Height::from(1000u32));
        assert_eq!(
            cache
                .most_recent_entry(60)
                .unwrap()
                .digest_snapshot()
                .snapshot
                .height,
            Height::from(900u32)
        );
    }

    #[test]
    fn most_recent_entry_saturates_when_lag_exceeds_tip() {
        let mut cache = empty_cache(0);
        cache.insert_entry(cached_directory(100));
        cache.last_polled_height = Height::from(10u32);
        // allowed latest saturates to 0, so nothing qualifies (rather than underflowing)
        assert!(cache.most_recent_entry(50).is_none());
    }

    // The cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn populated_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 1000] {
            cache.insert_entry(populated_cached_directory(h));
        }
        cache.set_last_polled_height(Height::from(1050u32));

        let restored = round_trip_through_disk_cache(cache)?;

        assert_eq!(restored.last_polled_height, Height::from(1050u32));
        assert_eq!(restored.directory.len(), 3);
        let entry = restored.get_entry(Height::from(900u32)).unwrap();
        assert_eq!(entry.directory_records().len(), 1);
        assert_eq!(entry.directory_node_identities().len(), 1);
        Ok(())
    }

    #[test]
    fn signed_snapshot_round_trips_through_json() {
        let snapshot = signed_snapshot(500);
        let json = serde_json::to_string(&snapshot).unwrap();
        let back: SignedDigestSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.snapshot, snapshot.snapshot);
        assert_eq!(back.signer, snapshot.signer);
    }

    #[test]
    fn directory_snapshot_data_round_trips_through_json() {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(7);
        let kp = ed25519::KeyPair::new(&mut rng);
        let data = DirectorySnapshotData {
            height: Height::from(500u32),
            records: Vec::new(),
            node_identities: BTreeMap::from([(1, *kp.public_key())]),
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: DirectorySnapshotData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.height, data.height);
        // exercises the custom bs58 NodeId -> pubkey map codec
        assert_eq!(back.node_identities, data.node_identities);
    }
}
