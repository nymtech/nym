// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::auxiliary::PerformanceAuxiliary;
use nym_api_requests::pagination::{PaginatedResponse, Pagination};
use nym_contract_attestation::{DirectoryEntryRecord, SignedDigestSnapshot};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_mixnet_contract_common::NodeId;
use nym_performance_contract_common::LatestPerformance;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use tendermint::block::Height;

const DEFAULT_RECORDS_PAGE_SIZE: u32 = 200;
const MAX_RECORDS_PAGE_SIZE: u32 = 500;

/// A signed snapshot with the record set and node identities it was built from.
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
}

impl<R: Clone> AttestedSnapshot<R> {
    /// One page of the records, `per_page` defaulting to 200 and capped at 500.
    pub(crate) fn paged_records(&self, page: u32, per_page: Option<u32>) -> PaginatedResponse<R> {
        let per_page = per_page
            .unwrap_or(DEFAULT_RECORDS_PAGE_SIZE)
            .min(MAX_RECORDS_PAGE_SIZE);
        let offset = (page as usize).saturating_mul(per_page as usize);

        let data = self
            .records
            .iter()
            .skip(offset)
            .take(per_page as usize)
            .cloned()
            .collect::<Vec<_>>();

        PaginatedResponse {
            pagination: Pagination {
                total: self.records.len(),
                page,
                size: data.len(),
            },
            data,
        }
    }
}

/// The performance contract's snapshot, with each bonded node's committed entry indexed by node,
/// since the topology routes every client fetches look one up per node.
#[derive(Deserialize)]
#[serde(from = "AttestedSnapshot<LatestPerformance>")]
pub(crate) struct PerformanceSnapshot {
    snapshot: AttestedSnapshot<LatestPerformance>,

    /// Each bonded node's position in the snapshot's records, built from them on construction
    /// and on load, and never saved.
    bonded: HashMap<NodeId, usize>,
}

impl From<AttestedSnapshot<LatestPerformance>> for PerformanceSnapshot {
    fn from(snapshot: AttestedSnapshot<LatestPerformance>) -> Self {
        let bonded = snapshot
            .records()
            .iter()
            .enumerate()
            .filter(|(_, committed)| snapshot.node_identities().contains_key(&committed.node_id))
            .map(|(position, committed)| (committed.node_id, position))
            .collect();
        PerformanceSnapshot { snapshot, bonded }
    }
}

// saved as the snapshot alone, so the on-disk format is the snapshot's
impl Serialize for PerformanceSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.snapshot.serialize(serializer)
    }
}

impl PerformanceSnapshot {
    pub(crate) fn snapshot(&self) -> &AttestedSnapshot<LatestPerformance> {
        &self.snapshot
    }

    /// `node_id`'s committed entry, if it is bonded.
    pub(crate) fn bonded_entry(&self, node_id: NodeId) -> Option<&LatestPerformance> {
        let position = *self.bonded.get(&node_id)?;
        self.snapshot.records().get(position)
    }
}

/// Every attested contract's snapshot at one cadence height, retained together: a height is
/// held for all of them or for none.
#[derive(Serialize, Deserialize)]
pub(crate) struct RetainedHeight {
    pub(crate) directory: AttestedSnapshot<DirectoryEntryRecord>,

    pub(crate) performance: PerformanceSnapshot,

    /// Read at the same height as the performance snapshot, but never signed or served on the
    /// attestation routes.
    pub(crate) performance_auxiliary: PerformanceAuxiliary,

    pub(crate) geolocation: AttestedSnapshot<GeolocationRecord>,
}

impl RetainedHeight {
    /// The height every snapshot in it was taken at.
    pub(crate) fn height(&self) -> Height {
        self.directory.height()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct AttestedSnapshotsCacheData {
    last_polled_height: Height,
    entries: BTreeMap<Height, RetainedHeight>,
}

impl AttestedSnapshotsCacheData {
    pub(crate) fn insert_entry(&mut self, entry: RetainedHeight) {
        self.entries.insert(entry.height(), entry);
    }

    /// Records the chain tip the cache was last reconciled against.
    pub(crate) fn set_last_polled_height(&mut self, height: Height) {
        self.last_polled_height = height;
    }

    /// Drops every entry older than the oldest height in `retained`, every entry when it is
    /// empty: pruning by height rather than by count, so that every producer keeps the same
    /// heights whichever boundaries it missed, and by age rather than by membership, so that
    /// after an interval change the previous grid's entries stay served until they age out.
    pub(crate) fn remove_stale(&mut self, retained: &[Height]) {
        match retained.iter().min() {
            Some(oldest) => self.entries.retain(|height, _| height >= oldest),
            None => self.entries.clear(),
        }
    }

    pub(crate) fn contains_entry(&self, height: Height) -> bool {
        self.entries.contains_key(&height)
    }

    pub(crate) fn most_recent_entry(&self, settle_lag: usize) -> Option<&RetainedHeight> {
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

    pub(crate) fn get_entry(&self, height: Height) -> Option<&RetainedHeight> {
        self.entries.get(&height)
    }

    /// The newest retained entry below `height`.
    pub(crate) fn previous_entry(&self, height: Height) -> Option<&RetainedHeight> {
        self.entries
            .range(..height)
            .next_back()
            .map(|(_, entry)| entry)
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::node_performance::attestation::test_helpers::{block_time, weights};
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    /// A snapshot signed at `height`.
    pub(crate) fn snapshot<R>(
        height: u32,
        records: Vec<R>,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> AttestedSnapshot<R> {
        let digest_snapshot = mock_digest_snapshot(Height::from(height))
            .signed(&dummy_ed25519_keypair(height as u64));
        AttestedSnapshot::new(digest_snapshot, records, node_identities)
    }

    /// A retained height whose snapshots hold no records.
    pub(crate) fn empty_retained_height(height: u32) -> RetainedHeight {
        RetainedHeight {
            directory: snapshot(height, Vec::new(), BTreeMap::new()),
            performance: snapshot(height, Vec::new(), BTreeMap::new()).into(),
            performance_auxiliary: PerformanceAuxiliary::new_for_tests(
                weights(),
                block_time(height),
                BTreeMap::new(),
            ),
            geolocation: snapshot(height, Vec::new(), BTreeMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::{empty_retained_height, snapshot};
    use super::*;
    use crate::node_performance::attestation::test_helpers::{block_time, committed};
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use cosmwasm_std::{Addr, Binary};
    use nym_directory_contract_common::{CuratedEntry, NodeEntry};
    use nym_geolocation_contract_common::{
        AgentPermissions, LocationAttestation, LocationEntry, LocationPayload, Method, Source,
        Subject,
    };
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    fn location_entry(checked_at: u64, attestation: Option<LocationAttestation>) -> LocationEntry {
        LocationEntry {
            payload: LocationPayload {
                version: 1,
                content: Binary::new(b"payload".to_vec()),
            },
            checked_at,
            attestation,
        }
    }

    /// A retained height holding every record shape of every contract, every `Option` set.
    /// Empty collections and unset options never reach their element encoders, so only such a
    /// value can exercise them.
    fn populated_entry(height: u32) -> RetainedHeight {
        let identities = BTreeMap::from([(1, *dummy_ed25519_keypair(1).public_key())]);

        let directory = vec![
            DirectoryEntryRecord::new_curated(
                "nym-api-1".to_string(),
                CuratedEntry::try_from_bytes(b"curated-payload").unwrap(),
            ),
            DirectoryEntryRecord::new_node(
                1,
                "label".to_string(),
                NodeEntry {
                    data: Binary::new(b"node-payload".to_vec()),
                    updated_at_height: 900,
                    sequence: 3,
                    signature: Binary::new(vec![7; 64]),
                },
            ),
        ];
        let geolocation = vec![
            GeolocationRecord::new_whitelisted_agent(
                Addr::unchecked("agent"),
                AgentPermissions {
                    can_measure: true,
                    can_relay_self_declared: true,
                },
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(1),
                Source::Measured {
                    method: Method::IpInfo,
                    agent: Addr::unchecked("agent"),
                },
                location_entry(1_700_000_000, None),
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(1),
                Source::SelfDeclared,
                location_entry(
                    1_700_000_100,
                    Some(LocationAttestation {
                        declared_at: 1_700_000_100,
                        signature: Binary::new(vec![7; 64]),
                    }),
                ),
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(2),
                Source::Override,
                location_entry(1_700_000_200, None),
            ),
        ];

        RetainedHeight {
            directory: snapshot(height, directory, identities.clone()),
            performance: snapshot(
                height,
                vec![committed(1, 100, Some(87))],
                identities.clone(),
            )
            .into(),
            performance_auxiliary: PerformanceAuxiliary::populated_for_tests(block_time(height)),
            geolocation: snapshot(height, geolocation, identities),
        }
    }

    fn empty_cache(last_polled: u32) -> AttestedSnapshotsCacheData {
        AttestedSnapshotsCacheData {
            last_polled_height: Height::from(last_polled),
            entries: BTreeMap::new(),
        }
    }

    fn heights(values: &[u32]) -> Vec<Height> {
        values.iter().map(|h| Height::from(*h)).collect()
    }

    fn held_heights(cache: &AttestedSnapshotsCacheData) -> Vec<Height> {
        cache.entries.keys().copied().collect()
    }

    #[test]
    fn a_missed_boundary_leaves_a_hole_rather_than_keeping_an_older_height() {
        // 200 was never taken
        let mut cache = empty_cache(350);
        for h in [100u32, 300, 400] {
            cache.insert_entry(empty_retained_height(h));
        }

        // the poll at 450 retains 400, 300 and 200
        cache.remove_stale(&heights(&[400, 300, 200]));

        // a producer that took 200 holds 200, 300 and 400: both agree on every height held
        assert_eq!(held_heights(&cache), heights(&[300, 400]));
    }

    #[test]
    fn an_interval_change_keeps_the_previous_grid_until_it_ages_out() {
        // held on a grid of 100, when the interval becomes 300
        let mut cache = empty_cache(1_050);
        for h in [800u32, 900, 1_000] {
            cache.insert_entry(empty_retained_height(h));
        }

        // the poll at 1050 retains 900, 600 and 300 on the new grid: nothing held is older, so
        // the latest settled entry is still served
        cache.remove_stale(&heights(&[900, 600, 300]));
        assert_eq!(held_heights(&cache), heights(&[800, 900, 1_000]));
        assert_eq!(
            cache.most_recent_entry(10).map(RetainedHeight::height),
            Some(Height::from(1_000u32))
        );

        // the old heights fall out as the new grid's window moves past them
        cache.insert_entry(empty_retained_height(1_500));
        cache.remove_stale(&heights(&[1_500, 1_200, 900]));
        assert_eq!(held_heights(&cache), heights(&[900, 1_000, 1_500]));
    }

    #[test]
    fn most_recent_entry_applies_the_settle_lag() {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 950] {
            cache.insert_entry(empty_retained_height(h));
        }
        // the chain tip is ahead of the newest snapshot
        cache.last_polled_height = Height::from(1000u32);

        // lag 60 -> allowed latest 940 -> newest snapshot at or below is 900
        assert_eq!(
            cache.most_recent_entry(60).unwrap().height(),
            Height::from(900u32)
        );
        // a smaller lag lets the newest snapshot through
        assert_eq!(
            cache.most_recent_entry(10).unwrap().height(),
            Height::from(950u32)
        );
    }

    #[test]
    fn recording_the_tip_after_warmup_makes_the_newest_snapshot_servable() {
        let mut cache = empty_cache(0);
        cache.insert_entry(empty_retained_height(900));
        // insert alone records no tip, so the settle lag hides it
        assert!(cache.most_recent_entry(60).is_none());

        cache.set_last_polled_height(Height::from(1000u32));
        assert_eq!(
            cache.most_recent_entry(60).unwrap().height(),
            Height::from(900u32)
        );
    }

    #[test]
    fn most_recent_entry_saturates_when_lag_exceeds_tip() {
        let mut cache = empty_cache(0);
        cache.insert_entry(empty_retained_height(100));
        cache.last_polled_height = Height::from(10u32);
        // allowed latest saturates to 0, so nothing qualifies (rather than underflowing)
        assert!(cache.most_recent_entry(50).is_none());
    }

    #[test]
    fn previous_entry_is_the_newest_entry_below_the_height() {
        let mut cache = empty_cache(0);
        for h in [800u32, 900, 1000] {
            cache.insert_entry(empty_retained_height(h));
        }

        assert_eq!(
            cache
                .previous_entry(Height::from(1000u32))
                .unwrap()
                .height(),
            Height::from(900u32)
        );
        // a height between retained ones takes the one below it
        assert_eq!(
            cache.previous_entry(Height::from(950u32)).unwrap().height(),
            Height::from(900u32)
        );
        assert!(cache.previous_entry(Height::from(800u32)).is_none());
    }

    #[test]
    fn records_are_paged_by_200_by_default_and_by_at_most_500() {
        let records = (1..=600)
            .map(|node_id| committed(node_id, 100, Some(50)))
            .collect();
        let snapshot = snapshot(1000, records, BTreeMap::new());

        let default = snapshot.paged_records(0, None);
        assert_eq!(default.data.len(), 200);
        assert_eq!(default.pagination.total, 600);

        let capped = snapshot.paged_records(0, Some(1000));
        assert_eq!(capped.data.len(), 500);

        let last = snapshot.paged_records(2, None);
        assert_eq!(last.data.first().map(|record| record.node_id), Some(401));
        assert_eq!(last.data.len(), 200);
    }

    // The cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn a_populated_retained_height_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let mut cache = empty_cache(0);
        cache.insert_entry(populated_entry(1000));
        cache.set_last_polled_height(Height::from(1050u32));

        let restored = round_trip_through_disk_cache(cache)?;

        assert_eq!(restored.last_polled_height, Height::from(1050u32));
        let expected = populated_entry(1000);
        let entry = restored.get_entry(Height::from(1000u32)).unwrap();
        assert_eq!(entry.directory.records(), expected.directory.records());
        assert_eq!(
            entry.performance.snapshot().records(),
            expected.performance.snapshot().records()
        );
        // the index is not saved, so loading must rebuild it
        assert_eq!(
            entry.performance.bonded_entry(1),
            expected.performance.bonded_entry(1)
        );
        assert!(entry.performance.bonded_entry(1).is_some());
        assert_eq!(entry.performance_auxiliary, expected.performance_auxiliary);
        assert_eq!(entry.geolocation.records(), expected.geolocation.records());
        assert_eq!(
            entry.geolocation.node_identities(),
            expected.geolocation.node_identities()
        );
        Ok(())
    }
}
