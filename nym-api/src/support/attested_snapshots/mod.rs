// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The attested snapshot producer shared by every contract nym-api attests (the directory,
//! performance): the retained snapshots, the refresher producing them at the directory
//! contract's cadence, and the state the routes serve them from.

use crate::support::attested_snapshots::data::AttestedSnapshotsCacheData;
use crate::support::attested_snapshots::provider::AttestedSnapshotsProvider;
use crate::support::attested_snapshots::source::AttestedSnapshotSource;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::config::DirectoryConfigDebug;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::path::PathBuf;
use std::sync::Arc;
use tendermint::chain;

pub(crate) mod data;
pub(crate) mod provider;
pub(crate) mod source;
pub(crate) mod state;

/// Warms up the snapshots of `source` and starts their refresher, with the `[directory.debug]`
/// knobs every attested contract shares.
pub(crate) async fn start_refresher<S: AttestedSnapshotSource>(
    source: S,
    config: DirectoryConfigDebug,
    signing_keys: Arc<ed25519::KeyPair>,
    query_client: QueryHttpRpcNyxdClient,
    chain_id: chain::Id,
    on_disk_cache_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<SharedCache<AttestedSnapshotsCacheData<S>>> {
    // the provider's warm-up drops the stale snapshots
    let cache = SharedCache::new_with_persistent_or_default(&on_disk_cache_file);

    let item_provider = AttestedSnapshotsProvider::new(
        source,
        config.retention_count,
        signing_keys,
        query_client,
        cache.clone(),
        chain_id,
        &shutdown_manager.clone_shutdown_token(),
    )
    .await?;

    CacheRefresher::new_with_initial_value(
        Box::new(item_provider),
        config.polling_interval,
        cache.clone(),
    )
    .named(format!("{}-cache-refresher", S::NAME))
    .with_update_fn(|main_cache, update| main_cache.update(update))
    .with_persistent_cache(on_disk_cache_file)
    .start(shutdown_manager.clone_shutdown_token());

    Ok(cache)
}
