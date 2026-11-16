// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::data::MixnetContractData;
use crate::node_families::data::NodeFamiliesData;
use crate::node_performance::attestation::auxiliary::PerformanceAuxiliary;
use crate::support::attested_snapshots::chain_anchor::{NyxChainAnchor, NyxContractAnchor};
use crate::support::attested_snapshots::data::{
    AttestedSnapshot, AttestedSnapshotsCacheData, HeightReads, RetainedHeight,
};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::Cache;
use crate::support::config::DirectoryConfigDebug;
use anyhow::{anyhow, Context};
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::error::VerificationError;
use nym_contract_anchor::whole_set::{verify_records, VerifiedRecords};
use nym_contract_attestation::{node_identities_hash, DigestSnapshot, DirectoryEntryRecord};
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::cadence::SnapshotCadence;
use nym_directory_client::key as directory_key;
use nym_geolocation_client::key as geolocation_key;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_mixnet_contract_common::NodeId;
use nym_performance_contract_common::constants::storage_keys as performance_storage_keys;
use nym_performance_contract_common::LatestPerformance;
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::contract_traits::{
    NymContractsProvider, PinnedDirectoryQueryClient, PinnedGeolocationQueryClient,
    PinnedPerformanceQueryClient,
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
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error, trace};

/// A cadence height is produced as soon as it is provable; the settle lag applies only to which
/// retained height is advertised as the latest.
const PRODUCTION_LAG: u64 = 1;

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

    /// How many of the newest cadence heights are retained.
    retention_count: usize,

    polling_interval: Duration,

    /// Whether nodes announcing non-public addresses are listed, for local test networks.
    unsafe_allow_illegal_ips: bool,

    cache: SharedCache<AttestedSnapshotsCacheData>,

    on_disk_cache_file: PathBuf,

    shutdown_token: ShutdownToken,
}

impl AttestedSnapshotsProducer {
    /// Fails when any attested contract's address is not configured.
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
            retention_count: config.retention_count,
            polling_interval: config.polling_interval,
            unsafe_allow_illegal_ips: config.unsafe_allow_illegal_ips,
            cache,
            on_disk_cache_file,
            shutdown_token,
        })
    }

    /// Produces the retained heights the cache does not hold, oldest first, so that each is read
    /// with the auxiliary value of the one before it; a shutdown ends it early.
    pub(crate) async fn warm_up(&self) -> anyhow::Result<()> {
        let cadence = self.read_cadence().await?;
        let retained_heights = self.retained_heights(&cadence);
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
                .run_until_cancelled(self.produce_height(height, cadence.interval.interval))
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
        cache.set_last_polled_height(cadence.tip);
        self.persist(&cache);

        Ok(())
    }

    /// Produces the newest cadence height unless it is held, then keeps only the heights retained
    /// at the tip; a failure changes nothing, and a shutdown ends it early.
    async fn poll(&self) -> anyhow::Result<()> {
        let cadence = self.read_cadence().await?;

        // always target the NEWEST cadence boundary rather than walking forward from the last
        // snapshot taken. a boundary the rpc node cannot serve then leaves a hole in the
        // retained window, instead of stalling the walk on one height that only gets older -
        // and thus more likely to be pruned - with every retry.
        let new_entry = match cadence.newest_settled_height(PRODUCTION_LAG) {
            Some(target) if !self.holds(target).await => {
                let Some(produced) = self
                    .shutdown_token
                    .run_until_cancelled(self.produce_height(target, cadence.interval.interval))
                    .await
                else {
                    return Ok(());
                };
                Some(produced.with_context(|| format!("failed to produce height {target}"))?)
            }
            _ => None,
        };

        let mut cache = self.cache.write().await?;
        cache.remove_stale(&self.retained_heights(&cadence));
        cache.set_last_polled_height(cadence.tip);
        // every warm-up replaces the persisted tip, so a poll that retained no height has nothing
        // worth writing
        if let Some(entry) = new_entry {
            cache.insert_entry(entry);
            self.persist(&cache);
        }
        Ok(())
    }

    /// Every attested contract's snapshot at `height`, signed, with the mixnet and node families
    /// contracts read there, or an error naming each contract that could not be read or verified
    /// there. `snapshot_interval` is that of the cadence `height` was chosen on.
    async fn produce_height(
        &self,
        height: Height,
        snapshot_interval: u32,
    ) -> anyhow::Result<RetainedHeight> {
        // first, so that nothing is read when trust at `height` cannot be established
        let app_hash = self
            .chain_anchor
            .trusted_app_hash(height)
            .await
            .context("failed to establish the app hash")?;
        let block_time = self
            .block_time(height)
            .await
            .context("failed to read the block time")?;
        let mixnet = MixnetContractData::read(self.chain_anchor.query_client(), height)
            .await
            .context("failed to read the mixnet contract")?;
        let node_identities = mixnet.node_identities();
        let node_families =
            NodeFamiliesData::read(self.chain_anchor.query_client(), height, &mixnet.nym_nodes)
                .await
                .context("failed to read the node families contract")?;

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

        let reads = HeightReads {
            block_time,
            snapshot_interval,
            directory: self.attest(&self.directory, &app_hash, directory),
            performance: self
                .attest(&self.performance, &app_hash, performance)
                .into(),
            performance_auxiliary,
            geolocation: self.attest(&self.geolocation, &app_hash, geolocation),
            mixnet,
            node_families,
        };
        Ok(RetainedHeight::new(reads, self.unsafe_allow_illegal_ips))
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

    /// The interval is read on every call, so that an update to it moves this api onto the new
    /// grid at its next poll, together with every other reader of the cadence.
    async fn read_cadence(&self) -> anyhow::Result<SnapshotCadence> {
        SnapshotCadence::read(self.chain_anchor.query_client())
            .await
            .context("failed to read the snapshot cadence")
    }

    /// The heights held after the warm-up and every poll: the newest one the poll would take,
    /// plus up to `retention_count - 1` before it (fewer on a young chain).
    fn retained_heights(&self, cadence: &SnapshotCadence) -> Vec<Height> {
        cadence.settled_heights(PRODUCTION_LAG, self.retention_count)
    }

    async fn holds(&self, height: Height) -> bool {
        self.cache
            .get()
            .await
            .is_ok_and(|cache| cache.contains_entry(height))
    }

    /// The reads of the newest retained height below `height`, under the cache's guard: take what
    /// is needed and drop it before any chain read.
    async fn previous_reads(&self, height: Height) -> Option<RwLockReadGuard<'_, HeightReads>> {
        let guard = self.cache.get().await.ok()?;
        RwLockReadGuard::try_map(guard, |cache| {
            cache.previous_entry(height).map(|entry| &entry.reads)
        })
        .ok()
    }

    /// The auxiliary value of the newest retained height below `height`.
    async fn previous_auxiliary(&self, height: Height) -> Option<PerformanceAuxiliary> {
        self.previous_reads(height)
            .await
            .map(|reads| reads.performance_auxiliary.clone())
    }

    fn persist(&self, cache: &Cache<AttestedSnapshotsCacheData>) {
        // the serialiser logs its own failure
        let _ = cache.try_serialise_to_file(&self.on_disk_cache_file);
    }

    /// The directory contract's whole committed set at `height`, taken from the newest retained
    /// height below it when the contract's digest is unchanged.
    async fn verified_directory(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<DirectoryEntryRecord>, VerificationError> {
        let trusted = self.directory.trusted_digest(height).await?;
        let unchanged = self
            .previous_reads(height)
            .await
            .and_then(|reads| reads.directory.records_if_unchanged(&trusted));

        let records = match unchanged {
            Some(records) => records,
            None => {
                let records = self
                    .chain_anchor
                    .query_client()
                    .get_all_directory_entries_at_height(height)
                    .await?;
                verify_records(&records, &trusted.accumulator)?;
                records
            }
        };

        Ok(VerifiedRecords {
            height,
            accumulator: trusted.accumulator,
            records,
            node_identities,
        })
    }

    /// The geolocation contract's whole committed set at `height`, whitelisted agents included,
    /// taken from the newest retained height below it when the contract's digest is unchanged.
    async fn verified_geolocation(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<GeolocationRecord>, VerificationError> {
        let trusted = self.geolocation.trusted_digest(height).await?;
        let unchanged = self
            .previous_reads(height)
            .await
            .and_then(|reads| reads.geolocation.records_if_unchanged(&trusted));

        let records = match unchanged {
            Some(records) => records,
            None => {
                let records = self
                    .chain_anchor
                    .query_client()
                    .get_all_geolocation_records_at_height(height)
                    .await?;
                verify_records(&records, &trusted.accumulator)?;
                records
            }
        };

        Ok(VerifiedRecords {
            height,
            accumulator: trusted.accumulator,
            records,
            node_identities,
        })
    }

    /// The performance contract's committed entries at `height`, in ascending node order, taken
    /// from the newest retained height below it when the contract's digest is unchanged.
    async fn verified_performance(
        &self,
        height: Height,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> Result<VerifiedRecords<LatestPerformance>, VerificationError> {
        let trusted = self.performance.trusted_digest(height).await?;
        let unchanged = self
            .previous_reads(height)
            .await
            .and_then(|reads| reads.performance.snapshot().records_if_unchanged(&trusted));

        let records = match unchanged {
            // retained in node order already
            Some(records) => records,
            None => {
                let mut records = self
                    .chain_anchor
                    .query_client()
                    .get_all_latest_performance_at_height(height)
                    .await?;
                verify_records(&records, &trusted.accumulator)?;
                // the per-node lookup's binary search relies on node order, which the digest
                // does not commit to
                records.sort_unstable_by_key(|record| record.node_id);
                records
            }
        };

        Ok(VerifiedRecords {
            height,
            accumulator: trusted.accumulator,
            records,
            node_identities,
        })
    }

    /// What the annotations need beside the performance `records` at `height`, read there with
    /// the value held for the newest retained height below it.
    async fn performance_auxiliary(
        &self,
        height: Height,
        records: &[LatestPerformance],
    ) -> Result<PerformanceAuxiliary, NyxdError> {
        let newest_epoch = records.iter().map(|entry| entry.epoch_id).max();
        let previous = self.previous_auxiliary(height).await;

        PerformanceAuxiliary::read(
            self.chain_anchor.query_client(),
            height,
            newest_epoch,
            previous,
        )
        .await
    }

    /// The time of the block at `height`.
    async fn block_time(&self, height: Height) -> Result<OffsetDateTime, NyxdError> {
        let header = self
            .chain_anchor
            .query_client()
            .header(height)
            .await
            .map_err(NyxdError::from)?;
        Ok(OffsetDateTime::from(header.header.time))
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
}
