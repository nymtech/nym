// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::cache::data::{
    CachedDirectory, DirectoryCacheUpdate, NymDirectoryCacheData, RawCachedDirectory,
};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::config::DirectoryConfig;
use anyhow::Context;
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_directory_attestation::DigestSnapshot;
use nym_directory_client::anchor::DirectoryTrustAnchor;
use nym_directory_client::client::DirectoryClient;
use nym_directory_client::error::DirectoryClientError;
use nym_validator_client::nyxd::contract_traits::{DirectoryQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::TendermintRpcClient;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::sync::Arc;
use tendermint::block::Height;
use tendermint::chain;

pub struct DirectoryDataProvider {
    /// Number of snapshots to keep
    retention_count: usize,

    /// Specifies the cadence of directory providers (e.g. nym-apis) snapshotting the directory content.
    /// Defined as number of blocks
    snapshot_interval: u32,

    chain_id: chain::Id,

    /// Keys of this api used for signing the directory snapshot.
    signing_keys: Arc<ed25519::KeyPair>,

    cache: SharedCache<NymDirectoryCacheData>,

    directory_client:
        DirectoryClient<Box<dyn DirectoryTrustAnchor + Send + Sync>, QueryHttpRpcNyxdClient>,
}

pub(crate) fn refresher_update_fn(
    main_cache: &mut NymDirectoryCacheData,
    update: DirectoryCacheUpdate,
    retention_count: usize,
) {
    main_cache.update(update, retention_count)
}

impl DirectoryDataProvider {
    pub async fn new(
        config: DirectoryConfig,
        signing_keys: Arc<ed25519::KeyPair>,
        trust_anchor: Box<dyn DirectoryTrustAnchor + Send + Sync>,
        query_client: QueryHttpRpcNyxdClient,
        cache: SharedCache<NymDirectoryCacheData>,
        chain_id: chain::Id,
    ) -> anyhow::Result<Self> {
        let snapshot_interval = query_client
            .get_snapshot_interval()
            .await
            .context("failed to get snapshot from the directory contract interval")?
            .interval;

        let mut this = DirectoryDataProvider {
            retention_count: config.debug.retention_count,
            snapshot_interval,
            chain_id,
            signing_keys,
            cache,
            directory_client: DirectoryClient::new(trust_anchor, query_client),
        };
        this.warmup_cache().await?;

        Ok(this)
    }

    // ensure the shared cache contains the `retention_count` latest snapshots
    async fn warmup_cache(&mut self) -> anyhow::Result<()> {
        let snapshot_interval = self.snapshot_interval as u64;
        let current_height = self.current_height().await?;
        let expected_latest = current_height.value() - (current_height.value() % snapshot_interval);
        let expected_retained = (0..self.retention_count as u64)
            // saturating so a young chain (fewer than retention_count intervals of history)
            // does not underflow; duplicate/zero heights are harmless (deduped by the cache map)
            .map(|i| expected_latest.saturating_sub(snapshot_interval * i))
            .map(|h| Height::from(h as u32))
            .collect::<Vec<_>>();

        let mut cache = self.cache.write().await?;

        // 1. drop stale entries
        cache.remove_stale(&expected_retained);

        // 2. retrieve required snapshots
        for expected in expected_retained {
            if !cache.contains_entry(expected) {
                let snapshot = self.retrieve_directory_snapshot(expected).await?;
                cache.insert_entry(snapshot);
            }
        }

        Ok(())
    }

    #[allow(clippy::unwrap_used, clippy::expect_used)]
    async fn last_snapshot_height(&self) -> Height {
        // SAFETY: we always have at least one snapshot in the cache
        self.cache
            .get()
            .await
            .expect("the cache has been initialised")
            .most_recent_height()
            .unwrap()
    }

    async fn current_height(&self) -> Result<Height, DirectoryClientError> {
        Ok(self
            .directory_client
            .client()
            .latest_block()
            .await?
            .block
            .header
            .height)
    }

    async fn retrieve_directory_snapshot(
        &self,
        height: Height,
    ) -> Result<CachedDirectory, DirectoryClientError> {
        let directory = self.directory_client.verified_directory(height).await?;
        let app_hash = self.directory_client.trusted_app_hash(height).await?;

        // SAFETY: the contract address must be available otherwise we wouldn't have been able to retrieve verified directory
        #[allow(clippy::unwrap_used)]
        let contract_address = self
            .directory_client
            .client()
            .directory_contract_address()
            .unwrap();

        let snapshot = DigestSnapshot {
            chain_id: self.chain_id.clone(),
            directory_contract: contract_address.clone(),
            height,
            app_hash,
            accumulator: directory.directory.accumulator.clone(),
            node_identities_hash: directory.node_identities_digest(),
        }
        .signed(&self.signing_keys);

        let node_identities = directory.node_identities.into_iter().collect();
        let raw_directory = RawCachedDirectory::new(snapshot, directory.records, node_identities);
        Ok(CachedDirectory::new(raw_directory))
    }
}

#[async_trait]
impl CacheItemProvider for DirectoryDataProvider {
    type Item = DirectoryCacheUpdate;
    type Error = DirectoryClientError;

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let current_height = self.current_height().await?;
        let next_snapshot_height =
            Height::from(self.last_snapshot_height().await.value() as u32 + self.snapshot_interval);

        // we need to have one additional block available so that we could retrieve the app hash
        let new_directory = if current_height.value() > next_snapshot_height.value() {
            Some(
                self.retrieve_directory_snapshot(next_snapshot_height)
                    .await?,
            )
        } else {
            None
        };

        Ok(Some(DirectoryCacheUpdate::new(
            new_directory,
            current_height,
        )))
    }
}
