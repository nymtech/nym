// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::auxiliary::PerformanceAuxiliary;
use crate::support::attested_snapshots::chain_anchor::{NyxChainAnchor, NyxContractAnchor};
use crate::support::attested_snapshots::data::{
    AttestedSnapshot, AttestedSnapshotsCacheData, RetainedHeight,
};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::Cache;
use crate::support::config::DirectoryConfigDebug;
use anyhow::{anyhow, Context};
use nym_contract_anchor::error::VerificationError;
use nym_contract_anchor::whole_set::{node_identities_at, verify_chain_read, VerifiedRecords};
use nym_contract_attestation::{node_identities_hash, DigestSnapshot, DirectoryEntryRecord};
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::key as directory_key;
use nym_geolocation_client::key as geolocation_key;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_mixnet_contract_common::NodeId;
use nym_performance_contract_common::constants::storage_keys as performance_storage_keys;
use nym_performance_contract_common::LatestPerformance;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::contract_traits::{
    DirectoryQueryClient, NymContractsProvider, PinnedDirectoryQueryClient,
    PinnedGeolocationQueryClient, PinnedPerformanceQueryClient,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::TendermintRpcClient;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tendermint::block::Height;
use tendermint::hash::AppHash;
use time::OffsetDateTime;
use tracing::{debug, error, trace};

/// Produces the snapshots of every contract nym-api attests, together at each cadence height: a
/// height is retained for every contract or for none.
pub(crate) struct AttestedSnapshotsProducer {
    /// The chain id, app hashes and chain tip the snapshots are taken against.
    chain_anchor: NyxChainAnchor,

    directory: NyxContractAnchor,

    performance: NyxContractAnchor,

    geolocation: NyxContractAnchor,

    /// Keys of this api used for signing the snapshots.
    signing_keys: Arc<ed25519::KeyPair>,

    cadence: SnapshotCadence,

    polling_interval: Duration,

    cache: SharedCache<AttestedSnapshotsCacheData>,

    on_disk_cache_file: PathBuf,

    shutdown_token: ShutdownToken,
}

impl AttestedSnapshotsProducer {
    /// Fails when any attested contract's address is not configured or the snapshot interval
    /// cannot be read.
    pub(crate) async fn new(
        chain_anchor: NyxChainAnchor,
        config: &DirectoryConfigDebug,
        signing_keys: Arc<ed25519::KeyPair>,
        cache: SharedCache<AttestedSnapshotsCacheData>,
        on_disk_cache_file: PathBuf,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        let query_client = chain_anchor.query_client();
        let directory = query_client
            .directory_contract_address()
            .context("failed to get directory contract address")?
            .clone();
        let performance = query_client
            .performance_contract_address()
            .context("failed to get performance contract address")?
            .clone();
        let geolocation = query_client
            .geolocation_contract_address()
            .context("failed to get geolocation contract address")?
            .clone();
        let snapshot_interval = query_client
            .get_snapshot_interval()
            .await
            .context("failed to get the snapshot interval from the directory contract")?
            .interval;

        Ok(AttestedSnapshotsProducer {
            directory: chain_anchor.contract_anchor(directory, directory_key::digest_item_key()),
            performance: chain_anchor.contract_anchor(
                performance,
                performance_storage_keys::DIGEST_STATE.as_bytes().to_vec(),
            ),
            geolocation: chain_anchor
                .contract_anchor(geolocation, geolocation_key::digest_item_key()),
            chain_anchor,
            signing_keys,
            cadence: SnapshotCadence {
                interval: snapshot_interval as u64,
                retention_count: config.retention_count,
            },
            polling_interval: config.polling_interval,
            cache,
            on_disk_cache_file,
            shutdown_token,
        })
    }

    /// Produces the retained heights the cache does not hold, oldest first, so that each is read
    /// with the auxiliary value of the one before it; a shutdown ends it early.
    pub(crate) async fn warm_up(&self) -> anyhow::Result<()> {
        let tip = self.current_height().await?;
        let retained_heights = self.cadence.retained_heights(tip);
        self.cache.write().await?.remove_stale(&retained_heights);

        // best-effort: a height the rpc node cannot serve (typically because it has pruned that
        // state) is logged and skipped rather than failing startup, so the api comes up with a
        // shallower retained window and fills it forward from the tip instead of never coming up
        for height in retained_heights.into_iter().rev() {
            if self.holds(height).await {
                continue;
            }
            let Some(produced) = self
                .shutdown_token
                .run_until_cancelled(self.produce_height(height))
                .await
            else {
                // startup then stops at its wait for the caches
                break;
            };
            match produced {
                Ok(entry) => self.cache.write().await?.insert_entry(entry),
                Err(err) => {
                    error!("failed to produce the attested snapshots at height {height}: {err:#}")
                }
            }
        }

        // record the tip we reconciled against, which the settle lag is measured from
        let mut cache = self.cache.write().await?;
        cache.set_last_polled_height(tip);
        self.persist(&cache);

        Ok(())
    }

    /// Produces the newest cadence height unless it is held, then keeps only the heights retained
    /// at the tip; a failure changes nothing, and a shutdown ends it early.
    async fn poll(&self) -> anyhow::Result<()> {
        let tip = self.current_height().await?;

        // always target the NEWEST cadence boundary rather than walking forward from the last
        // snapshot taken. a boundary the rpc node cannot serve then leaves a hole in the
        // retained window, instead of stalling the walk on one height that only gets older -
        // and thus more likely to be pruned - with every retry.
        let new_entry = match self.cadence.newest_height(tip) {
            Some(target) if !self.holds(target).await => {
                let Some(produced) = self
                    .shutdown_token
                    .run_until_cancelled(self.produce_height(target))
                    .await
                else {
                    return Ok(());
                };
                Some(produced.with_context(|| format!("failed to produce height {target}"))?)
            }
            _ => None,
        };

        let mut cache = self.cache.write().await?;
        cache.remove_stale(&self.cadence.retained_heights(tip));
        cache.set_last_polled_height(tip);
        // every warm-up replaces the persisted tip, so a poll that retained no height has nothing
        // worth writing
        if let Some(entry) = new_entry {
            cache.insert_entry(entry);
            self.persist(&cache);
        }
        Ok(())
    }

    /// Every attested contract's snapshot at `height`, signed, or an error naming each contract
    /// that could not be verified there.
    async fn produce_height(&self, height: Height) -> anyhow::Result<RetainedHeight> {
        // first, so that nothing is read when trust at `height` cannot be established
        let app_hash = self
            .chain_anchor
            .trusted_app_hash(height)
            .await
            .context("failed to establish the app hash")?;
        let node_identities = node_identities_at(self.chain_anchor.query_client(), height)
            .await
            .context("failed to read the bond set")?;

        let (directory, performance, geolocation) = tokio::join!(
            self.verified_directory(height, node_identities.clone()),
            self.verified_performance(height, node_identities.clone()),
            self.verified_geolocation(height, node_identities),
        );
        let (directory, performance, geolocation) =
            all_verified(directory, performance, geolocation)?;
        let performance_auxiliary = self
            .performance_auxiliary(height, &performance.records)
            .await
            .context("failed to read the performance auxiliary")?;

        Ok(RetainedHeight {
            directory: self.attest(&self.directory, &app_hash, directory),
            performance: self
                .attest(&self.performance, &app_hash, performance)
                .into(),
            performance_auxiliary,
            geolocation: self.attest(&self.geolocation, &app_hash, geolocation),
        })
    }

    /// `verified`, read from `contract`'s committed set, signed by this api.
    fn attest<R>(
        &self,
        contract: &NyxContractAnchor,
        app_hash: &AppHash,
        verified: VerifiedRecords<R>,
    ) -> AttestedSnapshot<R> {
        let digest_snapshot = DigestSnapshot {
            chain_id: self.chain_anchor.chain_id().clone(),
            contract: contract.contract().clone(),
            height: verified.height,
            app_hash: app_hash.clone(),
            accumulator: verified.accumulator,
            node_identities_hash: node_identities_hash(&verified.node_identities),
        }
        .signed(&self.signing_keys);

        AttestedSnapshot::new(digest_snapshot, verified.records, verified.node_identities)
    }

    async fn current_height(&self) -> anyhow::Result<Height> {
        let latest_block = self
            .chain_anchor
            .query_client()
            .latest_block()
            .await
            .context("failed to get the chain tip")?;
        Ok(latest_block.block.header.height)
    }

    async fn holds(&self, height: Height) -> bool {
        self.cache
            .get()
            .await
            .is_ok_and(|cache| cache.contains_entry(height))
    }

    /// The auxiliary value of the newest retained height below `height`, cloned so the cache is
    /// not held across the chain reads.
    async fn previous_auxiliary(&self, height: Height) -> Option<PerformanceAuxiliary> {
        self.cache
            .get()
            .await
            .ok()?
            .previous_entry(height)
            .map(|entry| entry.performance_auxiliary.clone())
    }

    fn persist(&self, cache: &Cache<AttestedSnapshotsCacheData>) {
        // the serialiser logs its own failure
        let _ = cache.try_serialise_to_file(&self.on_disk_cache_file);
    }

    /// The directory contract's whole committed set at `height`.
    async fn verified_directory(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<DirectoryEntryRecord>, VerificationError> {
        let records = self
            .chain_anchor
            .query_client()
            .get_all_directory_entries_at_height(height);
        verify_chain_read(&self.directory, height, records, node_identities).await
    }

    /// The geolocation contract's whole committed set at `height`, whitelisted agents included.
    async fn verified_geolocation(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<GeolocationRecord>, VerificationError> {
        let records = self
            .chain_anchor
            .query_client()
            .get_all_geolocation_records_at_height(height);
        verify_chain_read(&self.geolocation, height, records, node_identities).await
    }

    /// The performance contract's committed entries at `height`, in ascending node order.
    async fn verified_performance(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<LatestPerformance>, VerificationError> {
        let records = self
            .chain_anchor
            .query_client()
            .get_all_latest_performance_at_height(height);
        let mut verified =
            verify_chain_read(&self.performance, height, records, node_identities).await?;
        // the per-node lookup's binary search relies on node order, which the digest does not
        // commit to
        verified
            .records
            .sort_unstable_by_key(|record| record.node_id);
        Ok(verified)
    }

    /// What the annotations need beside the performance `records` at `height`, read there with
    /// the value held for the newest retained height below it.
    async fn performance_auxiliary(
        &self,
        height: Height,
        records: &[LatestPerformance],
    ) -> Result<PerformanceAuxiliary, NyxdError> {
        let query_client = self.chain_anchor.query_client();
        let block_time = query_client
            .header(height)
            .await
            .map_err(NyxdError::from)?
            .header
            .time;
        let newest_epoch = records.iter().map(|entry| entry.epoch_id).max();
        let previous = self.previous_auxiliary(height).await;

        PerformanceAuxiliary::read(
            query_client,
            height,
            OffsetDateTime::from(block_time),
            newest_epoch,
            previous,
        )
        .await
    }

    /// Polls every `polling_interval` until shutdown.
    pub(crate) async fn run(self) {
        let mut polls = tokio::time::interval(self.polling_interval);
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("AttestedSnapshotsProducer: Received shutdown");
                    break;
                }
                _ = polls.tick() => {
                    if let Err(err) = self.poll().await {
                        error!("failed to poll the attested snapshots: {err:#}");
                    }
                }
            }
        }

        debug!("AttestedSnapshotsProducer: exiting");
    }
}

/// The directory contract's snapshot cadence, which every attested contract is produced at, and
/// how many of its heights are retained.
struct SnapshotCadence {
    /// In blocks, read from the directory contract once at startup.
    interval: u64,

    retention_count: usize,
}

impl SnapshotCadence {
    /// The newest cadence height that can be snapshotted at `tip`: the largest positive multiple
    /// of the interval strictly below the tip, since the app hash committing state at H lives in
    /// header[H+1]. `None` before the chain passes its first boundary.
    fn newest_height(&self, tip: Height) -> Option<Height> {
        let below_tip = tip.value().checked_sub(1)?;
        let boundary = below_tip - (below_tip % self.interval);
        (boundary > 0).then(|| Height::from(boundary as u32))
    }

    /// The heights held at `tip`, after the warm-up and every poll: the newest one the poll would
    /// take there, plus up to `retention_count - 1` before it (fewer on a young chain).
    fn retained_heights(&self, tip: Height) -> Vec<Height> {
        let Some(newest) = self.newest_height(tip) else {
            return Vec::new();
        };
        (0..self.retention_count as u64)
            .map(|i| newest.value().saturating_sub(self.interval * i))
            .take_while(|height| *height > 0)
            .map(|height| Height::from(height as u32))
            .collect()
    }
}

/// Every contract's read at one height, or an error naming each contract whose read failed.
fn all_verified<D, P, G>(
    directory: Result<D, VerificationError>,
    performance: Result<P, VerificationError>,
    geolocation: Result<G, VerificationError>,
) -> anyhow::Result<(D, P, G)> {
    match (directory, performance, geolocation) {
        (Ok(directory), Ok(performance), Ok(geolocation)) => {
            Ok((directory, performance, geolocation))
        }
        (directory, performance, geolocation) => {
            let failures = [
                ("directory", directory.err()),
                ("performance", performance.err()),
                ("geolocation", geolocation.err()),
            ]
            .into_iter()
            .filter_map(|(contract, err)| Some(format!("the {contract} contract: {}", err?)))
            .collect::<Vec<_>>();
            Err(anyhow!("failed to verify {}", failures.join("; ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heights(values: &[u32]) -> Vec<Height> {
        values.iter().map(|h| Height::from(*h)).collect()
    }

    fn cadence(interval: u64, retention_count: usize) -> SnapshotCadence {
        SnapshotCadence {
            interval,
            retention_count,
        }
    }

    fn tip(height: u32) -> Height {
        Height::from(height)
    }

    fn failure(result: anyhow::Result<((), (), ())>) -> String {
        result.unwrap_err().to_string()
    }

    #[test]
    fn a_height_every_contract_verified_is_kept() {
        assert!(all_verified(Ok(()), Ok(()), Ok(())).is_ok());
    }

    #[test]
    fn one_failed_contract_fails_the_height_and_is_named() {
        let failure = failure(all_verified(
            Ok(()),
            Ok(()),
            Err(VerificationError::DigestMismatch),
        ));

        assert!(failure.contains("geolocation"));
        assert!(!failure.contains("directory"));
        assert!(!failure.contains("performance"));
    }

    #[test]
    fn every_failed_contract_is_named() {
        let failure = failure(all_verified(
            Err(VerificationError::DigestMismatch),
            Err(VerificationError::DigestMismatch),
            Ok(()),
        ));

        assert!(failure.contains("directory"));
        assert!(failure.contains("performance"));
        assert!(!failure.contains("geolocation"));
    }

    #[test]
    fn retained_heights_are_the_last_n_cadence_boundaries() {
        // interval 100, tip 1050 -> latest boundary 1000, then the two before it
        assert_eq!(
            cadence(100, 3).retained_heights(tip(1050)),
            heights(&[1000, 900, 800])
        );
        // exactly on a boundary: the tip cannot be snapshotted yet, as for the poll
        assert_eq!(
            cadence(100, 2).retained_heights(tip(1000)),
            heights(&[900, 800])
        );
    }

    #[test]
    fn a_young_chain_retains_only_the_boundaries_it_has() {
        // fewer than retention_count intervals of history, and height 0 is not a snapshot
        assert_eq!(cadence(100, 4).retained_heights(tip(150)), heights(&[100]));
        assert_eq!(cadence(100, 4).retained_heights(tip(50)), heights(&[]));
    }

    #[test]
    fn the_newest_height_is_the_most_recent_boundary_below_the_tip() {
        assert_eq!(cadence(100, 3).newest_height(tip(1050)), Some(tip(1000)));
        // one block past the boundary is enough: header[1001] carries the app hash for 1000
        assert_eq!(cadence(100, 3).newest_height(tip(1001)), Some(tip(1000)));
    }

    #[test]
    fn the_newest_height_skips_a_boundary_sitting_on_the_tip() {
        // the app hash committing state at H lives in header[H+1], so the boundary has to be
        // strictly behind the tip and the one before it is the newest
        assert_eq!(cadence(100, 3).newest_height(tip(1000)), Some(tip(900)));
    }

    #[test]
    fn there_is_no_newest_height_before_the_first_boundary() {
        // a young chain has no completed interval yet, and height 0 is not a snapshot
        assert_eq!(cadence(100, 3).newest_height(tip(50)), None);
        assert_eq!(cadence(100, 3).newest_height(tip(100)), None);
    }
}
