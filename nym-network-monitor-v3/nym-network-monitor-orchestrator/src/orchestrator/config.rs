// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::models::TestedRole;
use anyhow::Context;
use nym_network_defaults::{NymNetworkDetails, env_configured};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{client, nyxd};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;
use url::Url;

/// The liveness kind's own scheduling knobs. Grouped rather than flattened into [`Config`] because
/// every one of them is per-kind: the stress kind keeps `test_interval` and `test_timeout`, and
/// this is the same set of decisions taken for liveness.
///
/// Every value is provisional and deployment-tunable by design - no behaviour may depend on a
/// specific one.
#[derive(Debug, Copy, Clone)]
pub(crate) struct LivenessConfig {
    /// Whether liveness work may be assigned at all. On by default, so switching it off is a
    /// deployment-time decision; an agent that predates wave support cannot deserialise a liveness
    /// assignment, so a fleet mid-upgrade is a reason to set it.
    pub(crate) enabled: bool,

    /// How often each (node, role) pairing should be liveness-tested (e.g. `15m`). Well below the
    /// stress `test_interval`, since liveness is the low-volume probe.
    pub(crate) test_interval: Duration,

    /// Lease stamped on a liveness assignment's in-progress rows. Bounds ONE concurrent wave rather
    /// than the sum over its targets, because the agent probes the whole wave at once, so it does
    /// not scale with the wave size. It does have to cover the SLOWER of the two probes: a gateway
    /// wave pays session setup and measures two interfaces where a mixnode wave measures one.
    pub(crate) test_timeout: Duration,

    /// Maximum number of targets in a mixnode liveness wave.
    pub(crate) mixnode_wave_size: usize,

    /// Maximum number of targets in a gateway liveness wave. Lower than the mixnode wave: since a
    /// wave is one concurrent batch, this is what an agent holds open at once, and a gateway target
    /// costs a full client session where a mixnode target costs a Noise connection. v1 ran a
    /// 50-client window over its whole gateway population per cycle.
    pub(crate) gateway_wave_size: usize,
}

impl LivenessConfig {
    /// The wave size that applies to `role`.
    pub(crate) fn wave_size(&self, role: TestedRole) -> usize {
        match role {
            TestedRole::Mixnode => self.mixnode_wave_size,
            TestedRole::Gateway => self.gateway_wave_size,
        }
    }
}

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
    /// (e.g. `5m`). The stress kind's lease budget.
    pub(crate) test_timeout: Duration,

    /// Scheduling knobs of the liveness kind, whose cadence and lease are its own.
    pub(crate) liveness: LivenessConfig,

    /// Path to the SQLite database file.
    pub(crate) database_path: PathBuf,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    pub(crate) node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). A node that exceeds this budget keeps whatever an earlier cycle learned about it
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

    /// Maximum number of attempts (including the initial one) made to verify that the
    /// orchestrator's account is authorised in the network monitors contract before start-up.
    /// The process exits with an error once the budget is exhausted.
    pub(crate) chain_authorisation_check_max_attempts: NonZeroU32,

    /// Delay between consecutive chain authorisation checks during start-up. Applied both when
    /// the query itself fails and when it succeeds but the orchestrator is not (yet) listed.
    pub(crate) chain_authorisation_check_retry_delay: Duration,

    /// How often the orchestrator flushes accumulated test results to the nym-api as a signed
    /// batch submission (e.g. `15m`, `1h`).
    pub(crate) result_submission_interval: Duration,

    /// Maximum number of results to submit in a single POST request, applied per stream
    pub(crate) result_submission_batch_size: usize,
}

impl Config {
    /// Builds the validator client configuration from the orchestrator config.
    /// Falls back to environment-provided network details when RPC endpoint or
    /// contract addresses are not explicitly set.
    pub(crate) fn try_build_validator_client_config(&self) -> anyhow::Result<client::Config> {
        let mut base_network_details = if env_configured() {
            info!("using base NymNetworkDetails from env vars");
            NymNetworkDetails::new_from_env()
        } else {
            info!("using mainnet as base for NymNetworkDetails");
            NymNetworkDetails::new_mainnet()
        };

        let nyxd_endpoint = if let Some(provided) = &self.nyxd_rpc_endpoint {
            provided.clone()
        } else {
            base_network_details
                .endpoints
                .first()
                .context("no nyxd endpoints provided")?
                .nyxd_url
                .parse()?
        };

        if let Some(mixnet_contract_address) = &self.mixnet_contract_address {
            info!("overwriting mixnet contract address with {mixnet_contract_address}");
            base_network_details.contracts.mixnet_contract_address =
                Some(mixnet_contract_address.to_string());
        }

        if let Some(network_monitors_contract_address) = &self.network_monitors_contract_address {
            info!(
                "overwriting network monitors contract address with {network_monitors_contract_address}"
            );
            base_network_details
                .contracts
                .network_monitors_contract_address =
                Some(network_monitors_contract_address.to_string());
        }

        let nyxd_config = nyxd::Config::try_from_nym_network_details(&base_network_details)?;
        let client_config =
            client::Config::new(nyxd_endpoint, self.nym_api_endpoint.clone(), nyxd_config);

        info!("using the following config: {client_config:#?}");
        Ok(client_config)
    }
}
