// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::cache::data::NymDirectoryCacheData;
use crate::directory::cache::refresher::{refresher_update_fn, DirectoryDataProvider};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::config;
use crate::support::nyxd::Client;
use anyhow::Context;
use nym_config::defaults::mainnet::DIRECTORY_CHECKPOINT;
use nym_config::defaults::{mainnet, var_names};
use nym_crypto::asymmetric::ed25519;
use nym_directory_client::anchor::checkpoint::fetcher::client::basic_checkpoint_fetcher;
use nym_directory_client::anchor::checkpoint::fetcher::HttpsCheckpointProvider;
use nym_directory_client::anchor::checkpoint::provider::{
    load_checkpoint, CheckpointProvider, HardcodedCheckpointProvider, StoredCheckpointProvider,
};
use nym_directory_client::anchor::checkpoint::store::FileCheckpointStore;
use nym_directory_client::anchor::checkpoint::Checkpoint;
use nym_directory_client::anchor::proven::ProvenTrustAnchor;
use nym_directory_client::anchor::{nyx_default_options, DirectoryTrustAnchor, LightClientAnchor};
use nym_task::ShutdownManager;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::TendermintRpcClient;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tendermint::chain;
use time::OffsetDateTime;
use tracing::warn;
use url::Url;

pub(crate) mod data;
pub(crate) mod refresher;

struct AnchorWithChainId {
    trust_anchor: Box<dyn DirectoryTrustAnchor + Send + Sync + 'static>,
    chain_id: chain::Id,
}

impl AnchorWithChainId {
    fn new<A: DirectoryTrustAnchor + Send + Sync + 'static>(
        anchor: A,
        chain_id: chain::Id,
    ) -> Self {
        Self {
            trust_anchor: Box::new(anchor),
            chain_id,
        }
    }
}

async fn build_proven_trust_anchor(
    query_client: &QueryHttpRpcNyxdClient,
) -> anyhow::Result<AnchorWithChainId> {
    let directory_contract = query_client
        .directory_contract_address()
        .context("failed to get directory contract address")?;
    let anchor = ProvenTrustAnchor::new(
        query_client.clone_query_client(),
        directory_contract.clone(),
    );
    // we trust our rpc node, so we can get the chain id from it
    let chain_id = query_client.latest_block().await?.block.header.chain_id;
    Ok(AnchorWithChainId::new(anchor, chain_id))
}

fn get_root_pubkey() -> anyhow::Result<ed25519::PublicKey> {
    // `setup_env` backfills this env var from the compiled-in mainnet const (or an operator
    // override in a custom env file); fall back to the const directly if it is somehow unset.
    std::env::var(var_names::ROOT_ATTESTER_ED25519_BS58_PUBKEY)
        .unwrap_or_else(|_| mainnet::ROOT_ATTESTER_ED25519_BS58_PUBKEY.to_string())
        .parse()
        .context("configured root attester public key is malformed")
}

/// The checkpoint datum to trust: resolved from env (which `setup_env` backfills from the
/// compiled-in mainnet const, or an operator override), falling back to the const directly. An
/// empty value is treated as absent by [`HardcodedCheckpointProvider`].
fn get_checkpoint_datum() -> String {
    std::env::var(var_names::DIRECTORY_CHECKPOINT)
        .unwrap_or_else(|_| DIRECTORY_CHECKPOINT.to_string())
}

/// The well-known HTTPS checkpoint endpoint, or `None` when it is unconfigured (an empty URL, e.g.
/// the mainnet placeholder) or malformed. Never fatal: a missing or broken URL simply means "no
/// HTTPS provider", so it can never short-circuit the higher-priority checkpoint sources.
fn get_well_known_checkpoint_url() -> Option<Url> {
    // resolved from env (backfilled from the mainnet const, or an operator override)
    let raw = std::env::var(var_names::NYX_TRUSTED_CHECKPOINT_URL)
        .unwrap_or_else(|_| mainnet::NYX_TRUSTED_CHECKPOINT_URL.to_string());
    if raw.trim().is_empty() {
        return None;
    }
    match raw.parse() {
        Ok(url) => Some(url),
        Err(err) => {
            warn!("configured trusted nyx checkpoint url is malformed, skipping HTTPS provider: {err}");
            None
        }
    }
}

/// Build the optional last-resort HTTPS checkpoint provider. Returns `None` (never an error) when
/// the well-known URL is unconfigured or the fetcher cannot be built, so it can never short-circuit
/// the higher-priority stored/hardcoded sources.
fn build_https_provider(
    root_pubkey: ed25519::PublicKey,
) -> Option<HttpsCheckpointProvider<nym_http_api_client::Client>> {
    let url = get_well_known_checkpoint_url()?;
    match basic_checkpoint_fetcher(
        url.as_str(),
        false,
        Some(nym_http_api_client::generate_user_agent!()),
    ) {
        Ok(fetcher) => Some(HttpsCheckpointProvider::new(fetcher, root_pubkey)),
        Err(err) => {
            warn!("failed to build the well-known checkpoint HTTPS fetcher, skipping it: {err}");
            None
        }
    }
}

async fn get_trusted_checkpoint(
    checkpoint_store: &FileCheckpointStore,
) -> anyhow::Result<Checkpoint> {
    let root_pubkey = get_root_pubkey()?;

    // providers, tried in priority order (first non-stale candidate wins):
    // 1. the locally persisted, previously light-client-verified head
    // 2. the hardcoded datum (env override, backfilled from the compiled-in const)
    // 3. the well-known HTTPS endpoint (optional last resort; skipped if unconfigured/unreachable)
    let mut providers: Vec<&dyn CheckpointProvider> = Vec::new();

    // 1.
    let stored = StoredCheckpointProvider::new(checkpoint_store.clone());
    providers.push(&stored);

    // 2.
    let hardcoded = HardcodedCheckpointProvider::new(root_pubkey, get_checkpoint_datum());
    providers.push(&hardcoded);

    // 3. optional and non-fatal: an unconfigured (empty) or unbuildable HTTPS source is simply
    // omitted, so it can never prevent the sources above from being tried.
    let https_provider = build_https_provider(root_pubkey);
    if let Some(https_provider) = &https_provider {
        providers.push(https_provider);
    }

    load_checkpoint(&providers, OffsetDateTime::now_utc())
        .await
        .context("failed to obtain a valid directory light-client checkpoint")
}

async fn build_light_client_anchor(
    query_client: &QueryHttpRpcNyxdClient,
    on_disk_checkpoint_file: PathBuf,
) -> anyhow::Result<AnchorWithChainId> {
    // seed a light client from a self-authenticating checkpoint, then advance it forward.
    // the loader tries the locally persisted head first, then the compiled-in / well-known
    // seed; the proven-RPC path above remains the default (`trusted_rpc_node`).
    let directory_contract = query_client
        .directory_contract_address()
        .context("failed to get directory contract address")?;

    // the light-client-verified head is persisted alongside the on-disk directory cache in
    // the node data dir, so a restart can reseed from it without a fresh checkpoint
    let store = FileCheckpointStore::new(&on_disk_checkpoint_file);

    let checkpoint = get_trusted_checkpoint(&store).await?;
    let chain_id = checkpoint.signed_header.header.chain_id.clone();

    let anchor = LightClientAnchor::new(
        query_client.clone_query_client(),
        directory_contract.clone(),
        checkpoint,
        nyx_default_options(),
    )
    .with_store(store);

    Ok(AnchorWithChainId::new(anchor, chain_id))
}

pub(crate) async fn start_cache_refresher(
    config: config::DirectoryConfig,
    signing_keys: Arc<ed25519::KeyPair>,
    client: Client,
    on_disk_cache_file: PathBuf,
    on_disk_checkpoint_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<SharedCache<NymDirectoryCacheData>> {
    let query_client = client.query_client().await;
    let built_anchor = if config.debug.trusted_rpc_node {
        build_proven_trust_anchor(&query_client).await?
    } else {
        build_light_client_anchor(&query_client, on_disk_checkpoint_file).await?
    };

    // cache invalidation is not time-based for this cache. it will happen when item_provider
    // is created and `warmup_cache` is called, so the value passed here doesn't matter too much
    // as long as it's higher than the expected block times of the number of retained snapshots
    let directory_cache =
        SharedCache::new_with_persistent(&on_disk_cache_file, Duration::from_secs(9999999), None);

    let item_provider = DirectoryDataProvider::new(
        config,
        signing_keys,
        built_anchor.trust_anchor,
        query_client,
        directory_cache.clone(),
        built_anchor.chain_id,
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
    .with_persistent_cache(on_disk_cache_file)
    .start(shutdown_manager.clone_shutdown_token());

    Ok(directory_cache)
}
