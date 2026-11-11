// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The attested snapshots of every contract nym-api attests (the directory, performance and
//! geolocation contracts), produced together: at each cadence height of the directory contract,
//! one producer reads the mixnet and node families contracts, verifies every attested contract's
//! whole committed set against the identities of the node details read there, and retains the
//! height only when every read succeeded and every contract verified, as one entry of one
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
use std::time::Duration;

pub(crate) mod chain_anchor;
pub(crate) mod data;
pub(crate) mod producer;
pub(crate) mod state;

/// The block time assumed wherever a number of blocks is turned into time: a family's average
/// node age and the `Expires` estimate of the active-set routes. The same value is used on every
/// network.
///
/// It is mainnet's average over its whole history, from genesis (2022-01-31) to the tip on
/// 2026-11-10, measured from block header times on an archival node. Over shorter ranges the
/// average moved between 5.52 s and 5.77 s (it stepped up at the v0.60 upgrade), so no single
/// value is exact for every span.
///
/// The error is small for both uses. Against the true bonding times of 41 sampled mainnet nodes,
/// the computed age was at most 9.3 days (1%) too old and 2 days too old on average. `Expires`
/// looks at most one epoch ahead, about 2 s off at today's block time. In exchange every nym-api
/// derives the same value from heights alone, without looking up block times its RPC node may
/// have pruned.
pub(crate) const AVERAGE_BLOCK_TIME: Duration = Duration::from_millis(5687);

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
