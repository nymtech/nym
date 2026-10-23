// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots;
use crate::support::attested_snapshots::data::AttestedSnapshotsCacheData;
use crate::support::caching::cache::SharedCache;
use crate::support::chain_anchor::SharedChainAnchor;
use crate::support::config::DirectoryConfigDebug;
use crate::support::nyxd::Client;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) mod auxiliary;
pub(crate) mod handlers;
pub(crate) mod source;
#[cfg(test)]
pub(crate) mod test_helpers;

pub(crate) async fn start_cache_refresher(
    config: DirectoryConfigDebug,
    signing_keys: Arc<ed25519::KeyPair>,
    client: Client,
    chain_anchor: &SharedChainAnchor,
    on_disk_cache_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<SharedCache<AttestedSnapshotsCacheData<PerformanceSnapshotSource>>> {
    let source = PerformanceSnapshotSource::new(chain_anchor, client.query_client())?;

    attested_snapshots::start_refresher(
        source,
        config,
        signing_keys,
        client.query_client(),
        chain_anchor.chain_id().clone(),
        on_disk_cache_file,
        shutdown_manager,
    )
    .await
}
