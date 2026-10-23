// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::{
    AttestedSnapshot, AttestedSnapshotsCacheData, AttestedSnapshotsUpdate, RetainedEntry,
};
use crate::support::attested_snapshots::source::AttestedSnapshotSource;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheItemProvider;
use anyhow::Context;
use async_trait::async_trait;
use nym_contract_anchor::error::VerificationError;
use nym_contract_attestation::{node_identities_hash, DigestSnapshot};
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::contract_traits::DirectoryQueryClient;
use nym_validator_client::nyxd::TendermintRpcClient;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::sync::Arc;
use tendermint::block::Height;
use tendermint::chain;
use tracing::error;

pub(crate) struct AttestedSnapshotsProvider<S: AttestedSnapshotSource> {
    /// Number of snapshots to keep
    retention_count: usize,

    /// The cadence of snapshot producers (e.g. nym-apis), in blocks, read from the directory
    /// contract and shared by every attested contract.
    snapshot_interval: u32,

    chain_id: chain::Id,

    /// Keys of this api used for signing the snapshots.
    signing_keys: Arc<ed25519::KeyPair>,

    /// Reads the chain tip and the snapshot interval.
    query_client: QueryHttpRpcNyxdClient,

    cache: SharedCache<AttestedSnapshotsCacheData<S>>,

    source: S,
}

impl<S: AttestedSnapshotSource> AttestedSnapshotsProvider<S> {
    pub(crate) async fn new(
        source: S,
        retention_count: usize,
        signing_keys: Arc<ed25519::KeyPair>,
        query_client: QueryHttpRpcNyxdClient,
        cache: SharedCache<AttestedSnapshotsCacheData<S>>,
        chain_id: chain::Id,
    ) -> anyhow::Result<Self> {
        let snapshot_interval = query_client
            .get_snapshot_interval()
            .await
            .context("failed to get the snapshot interval from the directory contract")?
            .interval;

        let this = AttestedSnapshotsProvider {
            retention_count,
            snapshot_interval,
            chain_id,
            signing_keys,
            query_client,
            cache,
            source,
        };
        this.warmup_cache().await?;

        Ok(this)
    }

    // ensure the shared cache contains the `retention_count` latest snapshots
    async fn warmup_cache(&self) -> anyhow::Result<()> {
        let current_height = self.current_height().await?;
        let expected_retained = expected_retained_heights(
            current_height.value(),
            self.snapshot_interval as u64,
            self.retention_count,
        );

        let mut cache = self.cache.write().await?;

        // 1. drop stale entries
        cache.remove_stale(&expected_retained);

        // 2. retrieve the missing snapshots, oldest first, so each is read with the auxiliary
        //    value of the one before it. best-effort: a boundary the rpc node cannot serve
        //    (typically because it has pruned that state) is logged and skipped rather than
        //    failing startup, so the api comes up with a shallower retained window and fills
        //    it forward from the tip instead of never coming up at all
        for expected in expected_retained.into_iter().rev() {
            if cache.contains_entry(expected) {
                continue;
            }
            let previous = cache
                .previous_entry(expected)
                .map(|entry| entry.auxiliary().clone());
            let retrieved = self.retrieve_entry(expected, previous).await;
            match retrieved {
                Ok(entry) => cache.insert_entry(entry),
                Err(err) => error!(
                    "failed to retrieve the {} snapshot at height {expected}: {err}",
                    S::NAME
                ),
            }
        }

        // 3. record the tip we reconciled against, so the settle lag is measured from it rather
        //    than from the newest snapshot (which would hide that snapshot until the first refresh)
        cache.set_last_polled_height(current_height);

        Ok(())
    }

    async fn current_height(&self) -> Result<Height, VerificationError> {
        Ok(self.query_client.latest_block().await?.block.header.height)
    }

    async fn retrieve_entry(
        &self,
        height: Height,
        previous: Option<S::Auxiliary>,
    ) -> Result<RetainedEntry<S>, VerificationError> {
        let set = self.source.verified_set(height, previous).await?;

        let digest_snapshot = DigestSnapshot {
            chain_id: self.chain_id.clone(),
            contract: self.source.contract_address().clone(),
            height,
            app_hash: set.app_hash,
            accumulator: set.accumulator,
            node_identities_hash: node_identities_hash(&set.node_identities),
        }
        .signed(&self.signing_keys);

        let snapshot = AttestedSnapshot::new(digest_snapshot, set.records, set.node_identities);
        Ok(RetainedEntry::new(snapshot, set.auxiliary))
    }
}

#[async_trait]
impl<S: AttestedSnapshotSource> CacheItemProvider for AttestedSnapshotsProvider<S> {
    type Item = AttestedSnapshotsUpdate<S>;
    type Error = VerificationError;

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let current_height = self.current_height().await?;

        // always target the NEWEST cadence boundary rather than walking forward from the last
        // snapshot taken. a boundary the rpc node cannot serve then leaves a hole in the
        // retained window, instead of stalling the walk on one height that only gets older -
        // and thus more likely to be pruned - with every retry.
        let Some(target) =
            latest_snapshot_height(current_height.value(), self.snapshot_interval as u64)
        else {
            return Ok(Some(AttestedSnapshotsUpdate::new(None, current_height)));
        };

        // cloned so the cache is not held across the source's reads
        let previous = match self.cache.get().await {
            Ok(cache) if cache.contains_entry(target) => {
                return Ok(Some(AttestedSnapshotsUpdate::new(None, current_height)));
            }
            Ok(cache) => cache
                .previous_entry(target)
                .map(|entry| entry.auxiliary().clone()),
            // an uninitialised cache holds nothing, so the boundary is still worth taking
            Err(_) => None,
        };

        let new_entry = self.retrieve_entry(target, previous).await?;
        Ok(Some(AttestedSnapshotsUpdate::new(
            Some(new_entry),
            current_height,
        )))
    }
}

/// The snapshot heights the warmup should hold at `current_height`: the most recent cadence
/// boundary at or below it, plus the preceding `retention_count - 1` boundaries. Saturating
/// so a young chain (fewer than `retention_count` intervals of history) does not underflow;
/// duplicate/zero heights are harmless (the cache is keyed by height).
fn expected_retained_heights(
    current_height: u64,
    snapshot_interval: u64,
    retention_count: usize,
) -> Vec<Height> {
    let expected_latest = current_height - (current_height % snapshot_interval);
    (0..retention_count as u64)
        .map(|i| expected_latest.saturating_sub(snapshot_interval * i))
        .map(|h| Height::from(h as u32))
        .collect()
}

/// The newest cadence boundary that can be snapshotted at `current_height`: the largest
/// multiple of `snapshot_interval` strictly below the tip. `None` when the chain has not
/// completed an interval yet, or when that boundary is the tip itself - the app hash
/// committing state at H lives in header[H+1], so H has to be behind the tip, and the next
/// poll picks it up once the chain moves on.
fn latest_snapshot_height(current_height: u64, snapshot_interval: u64) -> Option<Height> {
    let boundary = current_height - (current_height % snapshot_interval);
    if boundary == 0 || boundary == current_height {
        return None;
    }
    Some(Height::from(boundary as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heights(values: &[u32]) -> Vec<Height> {
        values.iter().map(|h| Height::from(*h)).collect()
    }

    #[test]
    fn expected_retained_heights_are_the_last_n_cadence_boundaries() {
        // interval 100, current 1050 -> latest boundary 1000, then the two before it
        assert_eq!(
            expected_retained_heights(1050, 100, 3),
            heights(&[1000, 900, 800])
        );
        // exactly on a boundary: that boundary is the latest
        assert_eq!(
            expected_retained_heights(1000, 100, 2),
            heights(&[1000, 900])
        );
    }

    #[test]
    fn expected_retained_heights_saturate_on_a_young_chain() {
        // fewer than retention_count intervals of history: earlier boundaries clamp to 0
        // rather than underflowing
        assert_eq!(
            expected_retained_heights(150, 100, 4),
            heights(&[100, 0, 0, 0])
        );
    }

    #[test]
    fn latest_snapshot_height_is_the_most_recent_boundary_below_the_tip() {
        assert_eq!(
            latest_snapshot_height(1050, 100),
            Some(Height::from(1000u32))
        );
        // one block past the boundary is enough: header[1001] carries the app hash for 1000
        assert_eq!(
            latest_snapshot_height(1001, 100),
            Some(Height::from(1000u32))
        );
    }

    #[test]
    fn latest_snapshot_height_skips_a_boundary_sitting_on_the_tip() {
        // the app hash committing state at H lives in header[H+1], so the boundary has to be
        // strictly behind the tip; the next poll picks it up once the chain moves on
        assert_eq!(latest_snapshot_height(1000, 100), None);
    }

    #[test]
    fn latest_snapshot_height_is_none_before_the_first_boundary() {
        // a young chain has no completed interval yet, and height 0 is not a snapshot
        assert_eq!(latest_snapshot_height(50, 100), None);
    }
}
