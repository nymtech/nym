// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::cache::data::NymDirectoryCacheData;
use crate::directory::cache::refresher::{refresher_update_fn, DirectoryDataProvider};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::config;
use crate::support::nyxd::Client;
use anyhow::{bail, Context};
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::anchor::proven::ProvenTrustAnchor;
use nym_task::ShutdownManager;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::TendermintRpcClient;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod data;
pub(crate) mod refresher;

pub(crate) async fn start_cache_refresher(
    config: config::DirectoryConfig,
    signing_keys: Arc<ed25519::KeyPair>,
    client: Client,
    on_disk_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<SharedCache<NymDirectoryCacheData>> {
    let query_client = client.query_client().await;
    let (trust_anchor, chain_id) = if config.debug.trusted_rpc_node {
        let directory_contract = query_client
            .directory_contract_address()
            .context("failed to get directory contract address")?;
        let anchor = ProvenTrustAnchor::new(
            query_client.clone_query_client(),
            directory_contract.clone(),
        );
        let chain_id = query_client.latest_block().await?.block.header.chain_id;
        (Box::new(anchor), chain_id)
    } else {
        // we need to be able to have a base chain checkpoint from which we could advance the light client
        bail!("unimplemented external checkpoint retrieval")
    };

    // cache invalidation is not time-based for this cache. it will happen when item_provider
    // is created and `warmup_cache` is called, so the value passed here doesn't matter too much
    // as long as it's higher than the expected block times of the number of retained snapshots
    let directory_cache =
        SharedCache::new_with_persistent(&on_disk_file, Duration::from_secs(9999999), None);

    let item_provider = DirectoryDataProvider::new(
        config,
        signing_keys,
        trust_anchor,
        query_client,
        directory_cache.clone(),
        chain_id,
    )
    .await?;

    CacheRefresher::new_with_initial_value(
        Box::new(item_provider),
        config.debug.polling_interval,
        directory_cache.clone(),
    )
    .named("directory-cache-refresher")
    .with_update_fn(move |main_cache, update| {
        refresher_update_fn(main_cache, update, config.debug.retention_count)
    })
    .with_persistent_cache(on_disk_file)
    .start(shutdown_manager.clone_shutdown_token());

    Ok(directory_cache)
}
