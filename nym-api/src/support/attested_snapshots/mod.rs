// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The attested snapshots of every contract nym-api attests (the directory, performance and
//! geolocation contracts), produced together: at each cadence height of the directory contract,
//! one producer reads the mixnet bond set once, verifies every contract's whole committed set
//! against it, and retains the height only when every contract verified, as one entry of one
//! persisted cache the routes serve from.

use crate::support::attested_snapshots::chain_anchor::NyxChainAnchor;
use crate::support::attested_snapshots::producer::AttestedSnapshotsProducer;
use crate::support::attested_snapshots::state::ContractSnapshots;
use crate::support::caching::cache::SharedCache;
use crate::support::config::DirectoryConfigDebug;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) mod chain_anchor;
pub(crate) mod data;
pub(crate) mod producer;
pub(crate) mod state;

/// Builds the producer of every attested contract's snapshots, runs its warm-up and spawns its
/// polls, with the `[directory.debug]` knobs. The light client, when configured, persists its
/// head to `on_disk_checkpoint_file`.
pub(crate) async fn start(
    config: &DirectoryConfigDebug,
    query_client: QueryHttpRpcNyxdClient,
    signing_keys: Arc<ed25519::KeyPair>,
    on_disk_checkpoint_file: &Path,
    on_disk_cache_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<ContractSnapshots> {
    let chain_anchor = NyxChainAnchor::new(config, query_client, on_disk_checkpoint_file).await?;

    // the warm-up drops the stale heights
    let cache = SharedCache::new_with_persistent_or_default(&on_disk_cache_file);

    let producer = AttestedSnapshotsProducer::new(
        chain_anchor,
        config,
        signing_keys,
        cache.clone(),
        on_disk_cache_file,
        shutdown_manager.clone_shutdown_token(),
    )
    .await?;
    producer.warm_up().await?;
    tokio::spawn(producer.run());

    Ok(ContractSnapshots::new(cache, config.settle_lag))
}
