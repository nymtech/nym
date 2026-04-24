// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::{NymNetworkDetails, ValidatorDetails};
use nym_validator_client::client;
use nym_validator_client::nyxd::AccountId;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// HTTPS RPC URL of a Nyx node (e.g. `https://rpc.nymtech.net`).
    /// If not provided, the default value from the environment will be retrieved (if available).
    pub(crate) nyxd_rpc_endpoint: Option<Url>,

    /// HTTP endpoint of the nym-api to which test results are submitted.
    pub(crate) nym_api_endpoint: Url,

    /// HTTP address to bind the HTTP server to (e.g. `0.0.0.0:8080`).
    pub(crate) http_server_bind_address: SocketAddr,

    /// How often each node should be stress-tested (e.g. `30m`, `1h`).
    pub(crate) test_interval: Duration,

    /// Maximum time a single test run is allowed to run before being considered timed out
    /// (e.g. `5m`).
    pub(crate) test_timeout: Duration,

    /// Path to the SQLite database file.
    pub(crate) database_path: PathBuf,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    pub(crate) node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). Queries that exceed this budget leave the corresponding fields as `NULL`
    /// (e.g. `10s`).
    pub(crate) node_info_query_timeout: Duration,

    /// Bech32 address of the mixnet contract used to retrieve the list of bonded nodes.
    /// If not provided, the default value from the environment will be retrieved (if available).
    pub(crate) mixnet_contract_address: Option<AccountId>,

    /// Bech32 address of the networks monitors contract used to authorise agents
    /// If not provided, the default value from the environment will be retrieved (if available).
    pub(crate) network_monitors_contract_address: Option<AccountId>,

    /// Maximum age of a completed test run row before it is evicted from the local database.
    /// Rows older than this are assumed to have already been submitted to the nym-api
    /// (e.g. `7d`, `24h`).
    pub(crate) testrun_eviction_age: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    pub(crate) number_of_concurrent_node_queries: usize,
}

impl Config {
    /// Builds the validator client configuration from the orchestrator config.
    /// Falls back to environment-provided network details when RPC endpoint or
    /// contract addresses are not explicitly set.
    pub(crate) fn try_build_validator_client_config(&self) -> anyhow::Result<client::Config> {
        // if one if the values is missing, we have no choice but to attempt to use the env
        let mut base_network_details = if self.nyxd_rpc_endpoint.is_none()
            || self.mixnet_contract_address.is_none()
            || self.network_monitors_contract_address.is_none()
        {
            NymNetworkDetails::new_from_env()
        } else {
            NymNetworkDetails::new_mainnet()
        };

        base_network_details.set_nym_api_urls(vec![self.nym_api_endpoint.clone()]);

        if let Some(rpc_endpoint) = &self.nyxd_rpc_endpoint {
            base_network_details.endpoints =
                vec![ValidatorDetails::new_nyxd_only(rpc_endpoint.as_str())];
        }

        if let Some(mixnet_contract_address) = &self.mixnet_contract_address {
            base_network_details.contracts.mixnet_contract_address =
                Some(mixnet_contract_address.to_string());
        }

        if let Some(network_monitors_contract_address) = &self.network_monitors_contract_address {
            base_network_details
                .contracts
                .network_monitors_contract_address =
                Some(network_monitors_contract_address.to_string());
        }

        let client_config = client::Config::try_from_nym_network_details(&base_network_details)?;
        Ok(client_config)
    }
}
