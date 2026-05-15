// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::orchestrator::NetworkMonitorOrchestrator;
use crate::orchestrator::config::Config;
use anyhow::{Context, anyhow, bail};
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::bip39;
use std::mem;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use url::Url;
use zeroize::Zeroizing;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Bearer token required by the agents requesting work assignments and submitting results.
    #[clap(long, env = NYM_NETWORK_MONITOR_ORCHESTRATOR_AGENTS_TOKEN_ARG)]
    agents_token: String,

    /// Bearer token used for accessing the metrics and results endpoints.
    #[clap(long, env = NYM_NETWORK_MONITOR_ORCHESTRATOR_METRICS_AND_RESULTS_TOKEN_ARG)]
    metrics_and_results_token: String,

    /// How often each node should be stress-tested (e.g. `30m`, `1h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TEST_INTERVAL_ARG, value_parser = humantime::parse_duration, default_value = "2h")]
    test_interval: Duration,

    /// Maximum time a single test run is allowed to run before being considered timed out
    /// (e.g. `5m`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TEST_TIMEOUT_ARG, value_parser = humantime::parse_duration, default_value = "5m")]
    test_timeout: Duration,

    /// HTTP address to bind the HTTP server to (e.g. `0.0.0.0:8080`).
    #[clap(long, env = NYM_NETWORK_MONITOR_HTTP_SERVER_BIND_ADDRESS_ARG, default_value = "0.0.0.0:8080")]
    http_server_bind_address: SocketAddr,

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
    #[clap(long, env = NYM_NETWORK_MONITOR_NODE_REFRESH_RATE_ARG, value_parser = humantime::parse_duration, default_value = "2h")]
    node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). Queries that exceed this budget leave the corresponding fields as `NULL`
    /// (e.g. `10s`).
    #[clap(long, env = NYM_NETWORK_MONITOR_NODE_INFO_QUERY_TIMEOUT_ARG, value_parser = humantime::parse_duration, default_value = "10s")]
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
    #[clap(long, env = NYM_NETWORK_MONITOR_TESTRUN_EVICTION_AGE_ARG, value_parser = humantime::parse_duration, default_value = "7d",)]
    testrun_eviction_age: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    #[clap(long, env = NYM_NETWORK_MONITOR_CONCURRENT_NODE_QUERIES_ARG, default_value_t = 32)]
    number_of_concurrent_node_queries: usize,

    /// Maximum number of attempts (including the initial one) made to verify that this
    /// orchestrator's account is authorised in the network monitors contract before start-up.
    /// The process exits with an error once the budget is exhausted.
    #[clap(long, env = NYM_NETWORK_MONITOR_CHAIN_AUTH_CHECK_MAX_ATTEMPTS_ARG, default_value = "10")]
    chain_authorisation_check_max_attempts: NonZeroU32,

    /// Delay between consecutive chain authorisation checks during start-up (e.g. `1m`, `30s`).
    /// Applied both when the query itself fails and when it succeeds but the orchestrator is not
    /// (yet) listed.
    #[clap(long, env = NYM_NETWORK_MONITOR_CHAIN_AUTH_CHECK_RETRY_DELAY_ARG, value_parser = humantime::parse_duration, default_value = "1m")]
    chain_authorisation_check_retry_delay: Duration,

    /// How often the orchestrator flushes accumulated test results to the nym-api as a signed
    /// batch submission (e.g. `15m`, `1h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_RESULT_SUBMISSION_INTERVAL_ARG, value_parser = humantime::parse_duration, default_value = "15m")]
    result_submission_interval: Duration,
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
            http_server_bind_address: self.http_server_bind_address,
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
            number_of_concurrent_node_queries: self.number_of_concurrent_node_queries,
            chain_authorisation_check_max_attempts: self.chain_authorisation_check_max_attempts,
            chain_authorisation_check_retry_delay: self.chain_authorisation_check_retry_delay,
            result_submission_interval: self.result_submission_interval,
        })
    }

    /// Moves the orchestrator agents token out of `self`, zeroizing the original.
    ///
    /// Returns an error if the token is empty.
    pub(crate) fn take_agents_orchestrator_token(&mut self) -> anyhow::Result<Zeroizing<String>> {
        // we must never accept empty tokens
        if self.agents_token.is_empty() {
            bail!("provided orchestrator token is empty, please provide a non-empty value")
        }
        let taken = mem::take(&mut self.agents_token);
        Ok(Zeroizing::new(taken))
    }

    /// Moves the orchestrator metrics-and-results token out of `self`, zeroizing the original.
    ///
    /// Returns an error if the token is empty.
    pub(crate) fn take_metrics_and_results_orchestrator_token(
        &mut self,
    ) -> anyhow::Result<Zeroizing<String>> {
        // we must never accept empty tokens
        if self.metrics_and_results_token.is_empty() {
            bail!("provided orchestrator token is empty, please provide a non-empty value")
        }
        let taken = mem::take(&mut self.metrics_and_results_token);
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
    let agents_auth_token = args.take_agents_orchestrator_token()?;
    let metrics_and_results_auth_token = args.take_metrics_and_results_orchestrator_token()?;
    let mnemonic = args.into_mnemonic();

    let mut orchestrator = NetworkMonitorOrchestrator::new(
        config,
        identity_keys,
        agents_auth_token,
        metrics_and_results_auth_token,
        mnemonic,
    )
    .await?;
    orchestrator.run().await?;
    Ok(())
}
