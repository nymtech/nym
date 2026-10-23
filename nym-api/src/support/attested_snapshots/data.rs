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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::source::VerifiedSet;
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use async_trait::async_trait;
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_contract_attestation::DirectoryEntryRecord;
    use nym_directory_client::error::VerificationError;
    use nym_directory_contract_common::CuratedEntry;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use nym_validator_client::nyxd::AccountId;

    /// Gives the cache its record and auxiliary types; the cache itself never fetches.
    struct TestSource;

    #[async_trait]
    impl AttestedSnapshotSource for TestSource {
        type Record = DirectoryEntryRecord;
        type Auxiliary = ();

        const NAME: &'static str = "test";

        fn contract_address(&self) -> &AccountId {
            unreachable!("the cache never fetches")
        }

        async fn verified_set(
            &self,
            _height: Height,
            _previous: Option<&()>,
        ) -> Result<VerifiedSet<Self>, VerificationError> {
            unreachable!("the cache never fetches")
        }
    }

    fn signed_snapshot(height: u32) -> SignedDigestSnapshot {
        mock_digest_snapshot(Height::from(height)).signed(&dummy_ed25519_keypair(height as u64))
    }

    fn entry(height: u32) -> RetainedEntry<TestSource> {
        let snapshot = AttestedSnapshot::new(signed_snapshot(height), Vec::new(), BTreeMap::new());
        RetainedEntry::new(snapshot, ())
    }

    /// An entry carrying actual content. Empty `records` / `node_identities` never reach their
    /// element encoders, so only a populated value can exercise them.
    fn populated_entry(height: u32) -> RetainedEntry<TestSource> {
        let records = vec![DirectoryEntryRecord::new_curated(
            "nym-api-1".to_string(),
            CuratedEntry::try_from_bytes(b"curated-payload").unwrap(),
        )];
        let identity = *dummy_ed25519_keypair(height as u64).public_key();

        let snapshot = AttestedSnapshot::new(
            signed_snapshot(height),
            records,
            BTreeMap::from([(1, identity)]),
        );
        RetainedEntry::new(snapshot, ())
    }

    fn empty_cache(last_polled: u32) -> AttestedSnapshotsCacheData<TestSource> {
        AttestedSnapshotsCacheData {
            last_polled_height: Height::from(last_polled),
            entries: BTreeMap::new(),
        }
    }

    #[test]
    fn update_prunes_to_the_retention_count() {
        let mut cache = empty_cache(0);
        for h in [100u32, 200, 300, 400] {
            cache.update(
                AttestedSnapshotsUpdate::new(Some(entry(h)), Height::from(h)),
                3,
            );
        }
        // once the window exceeds the retention count the oldest snapshot is dropped
        assert_eq!(cache.entries.len(), 3);
        assert!(cache.get_entry(Height::from(100u32)).is_none());
        assert!(cache.get_entry(Height::from(200u32)).is_some());
        assert!(cache.get_entry(Height::from(400u32)).is_some());
    }

    #[test]
    fn remove_stale_retains_only_the_listed_heights() {
        let mut cache = empty_cache(300);
        for h in [100u32, 200, 300] {
            cache.insert_entry(entry(h));
        }
        cache.remove_stale(&[Height::from(200u32), Height::from(300u32)]);
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.get_entry(Height::from(100u32)).is_none());
        assert!(cache.get_entry(Height::from(300u32)).is_some());
    }

    #[test]
    fn most_recent_entry_applies_the_settle_lag() {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 950] {
            cache.insert_entry(entry(h));
        }
        // the chain tip is ahead of the newest snapshot
        cache.last_polled_height = Height::from(1000u32);

        // lag 60 -> allowed latest 940 -> newest snapshot at or below is 900
        assert_eq!(
            cache.most_recent_entry(60).unwrap().snapshot().height(),
            Height::from(900u32)
        );
        // a smaller lag lets the newest snapshot through
        assert_eq!(
            cache.most_recent_entry(10).unwrap().snapshot().height(),
            Height::from(950u32)
        );
    }

    #[test]
    fn recording_the_tip_after_warmup_makes_the_newest_snapshot_servable() {
        let mut cache = empty_cache(0);
        cache.insert_entry(entry(900));
        // insert alone leaves last_polled at the snapshot height, so the settle lag hides it
        assert!(cache.most_recent_entry(60).is_none());

        cache.set_last_polled_height(Height::from(1000u32));
        assert_eq!(
            cache.most_recent_entry(60).unwrap().snapshot().height(),
            Height::from(900u32)
        );
    }

    #[test]
    fn most_recent_entry_saturates_when_lag_exceeds_tip() {
        let mut cache = empty_cache(0);
        cache.insert_entry(entry(100));
        cache.last_polled_height = Height::from(10u32);
        // allowed latest saturates to 0, so nothing qualifies (rather than underflowing)
        assert!(cache.most_recent_entry(50).is_none());
    }

    #[test]
    fn previous_entry_is_the_newest_entry_below_the_height() {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 1000] {
            cache.insert_entry(entry(h));
        }

        assert_eq!(
            cache
                .previous_entry(Height::from(1000u32))
                .unwrap()
                .snapshot()
                .height(),
            Height::from(900u32)
        );
        // a height between retained ones takes the one below it
        assert_eq!(
            cache
                .previous_entry(Height::from(950u32))
                .unwrap()
                .snapshot()
                .height(),
            Height::from(900u32)
        );
        assert!(cache.previous_entry(Height::from(800u32)).is_none());
    }

    // The cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn populated_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 1000] {
            cache.insert_entry(populated_entry(h));
        }
        cache.set_last_polled_height(Height::from(1050u32));

        let restored = round_trip_through_disk_cache(cache)?;

        assert_eq!(restored.last_polled_height, Height::from(1050u32));
        assert_eq!(restored.entries.len(), 3);
        let entry = restored.get_entry(Height::from(900u32)).unwrap();
        assert_eq!(entry.snapshot().records().len(), 1);
        assert_eq!(entry.snapshot().node_identities().len(), 1);
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
    fn snapshot_data_round_trips_through_json() {
        let data = SnapshotData::<DirectoryEntryRecord> {
            height: Height::from(500u32),
            records: Vec::new(),
            node_identities: BTreeMap::from([(1, *dummy_ed25519_keypair(7).public_key())]),
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: SnapshotData<DirectoryEntryRecord> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.height, data.height);
        // exercises the custom bs58 NodeId -> pubkey map codec
        assert_eq!(back.node_identities, data.node_identities);
    }
}
