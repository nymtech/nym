// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use nym_validator_client::nyxd::bip39;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;
use url::Url;

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
    #[clap(long, env = NYM_NETWORK_MONITOR_RPC_URL_ARG)]
    rpc_url: Url,

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

    /// Bech32 address of the mixnet contract used to retrieve the list of bonded nodes.
    #[clap(long, env = NYM_NETWORK_MONITOR_MIXNET_CONTRACT_ADDRESS_ARG)]
    mixnet_contract_address: String,

    /// Maximum age of a completed test run row before it is evicted from the local database.
    /// Rows older than this are assumed to have already been submitted to the nym-api
    /// (e.g. `7d`, `24h`).
    #[clap(long, env = NYM_NETWORK_MONITOR_TESTRUN_EVICTION_AGE_ARG, value_parser = humantime::parse_duration)]
    testrun_eviction_age: Duration,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    info!("Starting network monitor orchestrator");
    Ok(())
}
