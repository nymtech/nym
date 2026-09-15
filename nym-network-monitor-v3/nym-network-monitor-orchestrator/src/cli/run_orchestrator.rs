// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::orchestrator::NetworkMonitorOrchestrator;
use crate::orchestrator::config::{Config, LivenessConfig};
use anyhow::{Context, anyhow, bail};
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::bip39;
use std::mem;
use std::net::SocketAddr;
use std::num::{NonZeroU32, NonZeroUsize};
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
    /// (e.g. `5m`). Used as the stress kind's lease budget.
    #[clap(long, env = NYM_NETWORK_MONITOR_TEST_TIMEOUT_ARG, value_parser = humantime::parse_duration, default_value = "5m")]
    test_timeout: Duration,

    /// Whether liveness testing may be assigned to agents (e.g. `--liveness-enabled false`).
    /// Takes an explicit value rather than being a bare flag, so that a deployment can switch
    /// liveness off through the environment without a redeploy.
    #[clap(long, env = NYM_NETWORK_MONITOR_LIVENESS_ENABLED_ARG, action = clap::ArgAction::Set, default_value_t = true)]
    liveness_enabled: bool,

    /// How often each node should be liveness-tested, per role (e.g. `15m`).
    #[clap(long, env = NYM_NETWORK_MONITOR_LIVENESS_TEST_INTERVAL_ARG, value_parser = humantime::parse_duration, default_value = "15m")]
    liveness_test_interval: Duration,

    /// Maximum time a single liveness wave is allowed to run before its targets are released for
    /// reassignment (e.g. `1m`). Bounds ONE concurrent wave, not the sum over its targets, and has
    /// to cover the slower of the two probes, which is the gateway one.
    #[clap(long, env = NYM_NETWORK_MONITOR_LIVENESS_TEST_TIMEOUT_ARG, value_parser = humantime::parse_duration, default_value = "1m")]
    liveness_test_timeout: Duration,

    /// Maximum number of targets handed out in a single mixnode liveness assignment.
    #[clap(long, env = NYM_NETWORK_MONITOR_LIVENESS_MIXNODE_WAVE_SIZE_ARG, default_value = "100")]
    liveness_mixnode_wave_size: NonZeroUsize,

    /// Maximum number of targets handed out in a single gateway liveness assignment. Lower than
    /// the mixnode wave, since each target costs the agent a live client session.
    #[clap(long, env = NYM_NETWORK_MONITOR_LIVENESS_GATEWAY_WAVE_SIZE_ARG, default_value = "50")]
    liveness_gateway_wave_size: NonZeroUsize,

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
    /// etc.). A node that exceeds this budget keeps whatever an earlier cycle learned about it
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

    /// Maximum number of results to submit in a single POST request, applied per stream
    #[clap(long, env = NYM_NETWORK_MONITOR_RESULT_SUBMISSION_BATCH_SIZE_ARG, default_value = "50")]
    result_submission_batch_size: NonZeroUsize,
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
            liveness: LivenessConfig {
                enabled: self.liveness_enabled,
                test_interval: self.liveness_test_interval,
                test_timeout: self.liveness_test_timeout,
                mixnode_wave_size: self.liveness_mixnode_wave_size.get(),
                gateway_wave_size: self.liveness_gateway_wave_size.get(),
            },
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
            result_submission_batch_size: self.result_submission_batch_size.get(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // `Args` is a subcommand's argument group, so it needs a parser root to be exercised on its own
    #[derive(Parser)]
    struct TestCli {
        #[clap(flatten)]
        args: Args,
    }

    /// The arguments with no default, which every parse has to supply. The mnemonic is the
    /// all-zeros bip39 test vector - it still has to pass checksum validation to parse.
    const REQUIRED: &[&str] = &[
        "run-orchestrator",
        "--agents-token",
        "agents-token",
        "--metrics-and-results-token",
        "metrics-token",
        "--nym-api-endpoint",
        "https://nym-api.example.com/api",
        "--mnemonic",
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "--database-path",
        "/var/lib/nym-network-monitor/db.sqlite",
        "--private-key",
        "6HRy7XkUqDPr1JdKPKGdBnDaKvbNJhCTAqrnQNVJEmS7",
    ];

    fn parse(overrides: &[&str]) -> LivenessConfig {
        let argv: Vec<&str> = REQUIRED.iter().chain(overrides.iter()).copied().collect();
        TestCli::try_parse_from(argv)
            .expect("failed to parse arguments")
            .args
            .build_orchestrator_config()
            .expect("failed to build the config")
            .liveness
    }

    #[test]
    fn liveness_knobs_carry_their_documented_defaults() {
        let liveness = parse(&[]);
        assert!(liveness.enabled);
        assert_eq!(liveness.test_interval, Duration::from_secs(15 * 60));
        assert_eq!(liveness.test_timeout, Duration::from_secs(60));

        // the two waves are sized independently, the gateway one lower because each of its targets
        // costs a live client session rather than a Noise connection
        assert_eq!(liveness.mixnode_wave_size, 100);
        assert_eq!(liveness.gateway_wave_size, 50);
    }

    // every one of these values is provisional, so being able to move it without a code change is
    // itself a requirement. the enable flag is the load-bearing case: as a bare presence flag it
    // would parse and then be impossible to switch off, which is the one thing it exists to do
    #[test]
    fn every_liveness_knob_is_overridable() {
        let liveness = parse(&[
            "--liveness-enabled",
            "false",
            "--liveness-test-interval",
            "3m",
            "--liveness-test-timeout",
            "30s",
            "--liveness-mixnode-wave-size",
            "7",
            "--liveness-gateway-wave-size",
            "3",
        ]);

        assert!(!liveness.enabled);
        assert_eq!(liveness.test_interval, Duration::from_secs(3 * 60));
        assert_eq!(liveness.test_timeout, Duration::from_secs(30));
        assert_eq!(liveness.mixnode_wave_size, 7);
        assert_eq!(liveness.gateway_wave_size, 3);
    }

    // an assignment with no targets is not a valid assignment, so an empty wave is rejected at
    // parse time rather than producing one at dispatch. asserted per flag: the two waves are
    // separate knobs, so one of them keeping its NonZero parser proves nothing about the other
    #[test]
    fn a_zero_wave_size_is_rejected() {
        for flag in [
            "--liveness-mixnode-wave-size",
            "--liveness-gateway-wave-size",
        ] {
            let argv: Vec<&str> = REQUIRED.iter().copied().chain([flag, "0"]).collect();
            assert!(
                TestCli::try_parse_from(argv).is_err(),
                "{flag} accepted an empty wave"
            );
        }
    }
}
