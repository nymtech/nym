// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use nym_config::defaults::mainnet::DIRECTORY_CHECKPOINT;
use nym_config::defaults::{mainnet, var_names};
use nym_contract_anchor::anchor::checkpoint::fetcher::client::basic_checkpoint_fetcher;
use nym_contract_anchor::anchor::checkpoint::fetcher::HttpsCheckpointProvider;
use nym_contract_anchor::anchor::checkpoint::provider::{
    load_checkpoint, CheckpointProvider, HardcodedCheckpointProvider, StoredCheckpointProvider,
};
use nym_contract_anchor::anchor::checkpoint::store::FileCheckpointStore;
use nym_contract_anchor::anchor::checkpoint::Checkpoint;
use nym_crypto::asymmetric::ed25519;
use time::OffsetDateTime;
use tracing::warn;
use url::Url;

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

pub(super) async fn get_trusted_checkpoint(
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
        .context("failed to obtain a valid nyx light-client checkpoint")
}
