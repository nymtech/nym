// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::orchestrator::NetworkMonitorOrchestrator;
use crate::orchestrator::config::Config;
use anyhow::{Context, anyhow, bail};
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::bip39;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use url::Url;
use zeroize::Zeroizing;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Bearer token required by the agents requesting work assignments and submitting results.
    #[clap(long, env = NYM_NETWORK_MONITOR_ORCHESTRATOR_TOKEN_ARG)]
    orchestrator_token: String,

    /// How often each node should be stress-tested (e.g. `30m`, `1h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TEST_INTERVAL_ARG, value_parser = humantime::parse_duration)]
    test_interval: Duration,

    /// Maximum time a single test run is allowed to run before being considered timed out
    /// (e.g. `5m`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TEST_TIMEOUT_ARG, value_parser = humantime::parse_duration)]
    test_timeout: Duration,

    /// HTTP endpoint of the nym-api to which test results are submitted.
    #[clap(long, env = NYM_NETWORK_MONITOR_NYM_API_ENDPOINT_ARG)]
    nym_api_endpoint: Url,

    /// Mnemonic of the account used to authorise network monitor agents in the
    /// network monitors contract.
    #[clap(long, env = NYM_NETWORK_MONITOR_MNEMONIC_ARG)]
    mnemonic: bip39::Mnemonic,

    /// HTTPS RPC URL of a Nyx node (e.g. `https://rpc.nymtech.net`).
    /// If not provided, the default value from the environment will be retrieved (if available).
    #[clap(long, env = NYM_NETWORK_MONITOR_RPC_URL_ARG)]
    rpc_url: Option<Url>,

    /// Path to the SQLite database file.
    #[clap(long, env = NYM_NETWORK_MONITOR_DATABASE_PATH_ARG)]
    database_path: PathBuf,

    /// Base58-encoded Ed25519 private key used to authorise result submissions to the nym-api.
    #[clap(long, env = NYM_NETWORK_MONITOR_PRIVATE_KEY_ARG)]
    private_key: String,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_NODE_REFRESH_RATE_ARG, value_parser = humantime::parse_duration)]
    node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). Queries that exceed this budget leave the corresponding fields as `NULL`
    /// (e.g. `10s`).
    #[clap(long, env = NYM_NETWORK_MONITOR_NODE_INFO_QUERY_TIMEOUT_ARG, value_parser = humantime::parse_duration)]
    node_info_query_timeout: Duration,

    /// Bech32 address of the networks monitors contract used to authorise agents
    /// If not provided, the default value from the environment will be retrieved (if available).
    #[clap(long, env = NYM_NETWORK_MONITOR_NETWORK_MONITORS_CONTRACT_ADDRESS_ARG)]
    network_monitors_contract_address: Option<String>,

    /// Bech32 address of the mixnet contract used to retrieve the list of bonded nodes.
    /// If not provided, the default value from the environment will be retrieved (if available).
    #[clap(long, env = NYM_NETWORK_MONITOR_MIXNET_CONTRACT_ADDRESS_ARG)]
    mixnet_contract_address: Option<String>,

    /// Maximum age of a completed test run row before it is evicted from the local database.
    /// Rows older than this are assumed to have already been submitted to the nym-api
    /// (e.g. `7d`, `24h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TESTRUN_EVICTION_AGE_ARG, value_parser = humantime::parse_duration)]
    testrun_eviction_age: Duration,
}

impl Args {
    /// Converts the parsed CLI arguments into a [`Config`].
    ///
    /// Returns an error if `mixnet_contract_address` is not a valid bech32 account address.
    ///
    /// Note: `orchestrator_token`, `mnemonic`, and `private_key` are not part of [`Config`]
    /// and must be handled separately by the caller.
    pub(crate) fn build_orchestrator_config(&self) -> anyhow::Result<Config> {
        Ok(Config {
            nyxd_rpc_endpoint: self.rpc_url.clone(),
            nym_api_endpoint: self.nym_api_endpoint.clone(),
            test_interval: self.test_interval,
            test_timeout: self.test_timeout,
            database_path: self.database_path.clone(),
            node_refresh_rate: self.node_refresh_rate,
            node_info_query_timeout: self.node_info_query_timeout,
            network_monitors_contract_address: self
                .network_monitors_contract_address
                .as_ref()
                .map(|addr| addr.parse())
                .transpose()
                .map_err(|err| anyhow!("invalid network monitors contract address: {err}"))?,
            mixnet_contract_address: self
                .mixnet_contract_address
                .as_ref()
                .map(|addr| addr.parse())
                .transpose()
                .map_err(|err| anyhow!("invalid mixnet contract address: {err}"))?,
            testrun_eviction_age: self.testrun_eviction_age,
        })
    }

    /// Moves the orchestrator token out of `self`, zeroizing the original.
    ///
    /// Returns an error if the token is empty.
    pub(crate) fn take_orchestrator_token(&mut self) -> anyhow::Result<Zeroizing<String>> {
        // we must never accept empty tokens
        if self.orchestrator_token.is_empty() {
            bail!("provided orchestrator token is empty, please provide a non-empty value")
        }
        let taken = mem::take(&mut self.orchestrator_token);
        Ok(Zeroizing::new(taken))
    }

    /// Moves the raw Base58-encoded private key out of `self`, parses it into an Ed25519 key pair,
    /// and zeroizes the original string.
    ///
    /// Returns an error if the value is not a valid Base58-encoded Ed25519 private key.
    pub(crate) fn take_identity_key(&mut self) -> anyhow::Result<Arc<ed25519::KeyPair>> {
        // whatever happens, we'll zeroize the value
        let taken = Zeroizing::new(mem::take(&mut self.private_key));

        let private_key = ed25519::PrivateKey::from_base58_string(&taken)
            .context("malformed identity key provided")?;
        Ok(Arc::new(private_key.into()))
    }

    /// Consumes `self` and returns the mnemonic.
    pub(crate) fn into_mnemonic(self) -> bip39::Mnemonic {
        self.mnemonic
    }
}

pub(crate) async fn execute(mut args: Args) -> anyhow::Result<()> {
    info!("Starting network monitor orchestrator");
    let config = args.build_orchestrator_config()?;
    let identity_keys = args.take_identity_key()?;
    let auth_token = args.take_orchestrator_token()?;
    let mnemonic = args.into_mnemonic();

    let mut orchestrator =
        NetworkMonitorOrchestrator::new(config, identity_keys, auth_token, mnemonic).await?;
    orchestrator.run().await?;
    Ok(())
}
