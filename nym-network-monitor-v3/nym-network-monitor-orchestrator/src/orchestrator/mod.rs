// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::materialiser::{Materialiser, MaterialiserConfig};
use crate::http::api::{build_router, run_http_server};
use crate::http::state::{AppState, KnownAgents};
use crate::orchestrator::chain_capability_refresher::ChainCapabilityRefresher;
use crate::orchestrator::config::Config;
use crate::orchestrator::node_refresher::NodeRefresher;
use crate::orchestrator::result_submitter::ResultSubmitter;
use crate::orchestrator::stale_results_eviction::StaleResultsEviction;
use crate::signing_client::SigningClient;
use crate::storage::NetworkMonitorStorage;
use crate::submission::PerformanceSubmitter;
use anyhow::{Context, bail};
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nyxd::contract_traits::{
    NetworkMonitorsQueryClient, NetworkMonitorsSigningClient, PagedNetworkMonitorsQueryClient,
    PerformanceQueryClient,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{AccountId, bip39};
use nym_validator_client::rpc::TendermintRpcClientExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use zeroize::Zeroizing;

mod chain_capability_refresher;
pub(crate) mod config;
mod node_refresher;
pub(crate) mod prometheus;
mod result_submitter;
mod stale_results_eviction;

pub(crate) struct NetworkMonitorOrchestrator {
    /// Runtime configuration for the orchestrator.
    pub(crate) config: Config,

    /// Validator client used to:
    /// - submit test results to the nym-api
    /// - query node information from the chain
    /// - send authorisation transactions to the network monitors contract
    /// - submit measurements to the performance contract
    pub(crate) client: SigningClient,

    /// Ed25519 key pair used to sign result submissions to the nym-api.
    pub(crate) identity_keys: Arc<ed25519::KeyPair>,

    /// Bearer token presented by agents when requesting work assignments and submitting results.
    pub(crate) agents_http_auth_token: Arc<Zeroizing<String>>,

    /// Bearer token required when attempting to access the metrics or results endpoints.
    pub(crate) metrics_and_results_http_auth_token: Arc<Zeroizing<String>>,

    /// Handle to the local SQLite database used to track nodes and test runs.
    pub(crate) storage: NetworkMonitorStorage,

    /// Manages graceful shutdown signalling across all orchestrator tasks.
    pub(crate) shutdown_manager: ShutdownManager,
}

impl NetworkMonitorOrchestrator {
    /// Initialises the orchestrator: connects to the database, builds the validator client,
    /// and verifies that the orchestrator is authorised on both the chain and the nym-api, and in
    /// the performance contract when submission is enabled.
    pub(crate) async fn new(
        config: Config,
        identity_keys: Arc<ed25519::KeyPair>,
        agents_http_auth_token: Zeroizing<String>,
        metrics_and_results_http_auth_token: Zeroizing<String>,
        mnemonic: bip39::Mnemonic,
    ) -> anyhow::Result<Self> {
        let storage = NetworkMonitorStorage::init(&config.database_path).await?;

        let client_config = config.try_build_validator_client_config()?;
        let client = SigningClient::new(client_config, mnemonic)?;

        let this = NetworkMonitorOrchestrator {
            config,
            client,
            identity_keys,
            agents_http_auth_token: Arc::new(agents_http_auth_token),
            metrics_and_results_http_auth_token: Arc::new(metrics_and_results_http_auth_token),
            storage,
            shutdown_manager: ShutdownManager::build_new_default()?,
        };
        this.verify_on_chain_balance().await?;

        let announced_identity_key = this.verify_orchestrator_chain_authorisation().await?;
        if this.config.performance_submission_enabled {
            this.verify_orchestrator_performance_authorisation().await?;
        } else {
            info!("submission to the performance contract is disabled");
        }
        this.reconcile_announced_identity_key(announced_identity_key)
            .await?;
        this.verify_orchestrator_nym_api_authorisation().await?;

        Ok(this)
    }

    /// Returns the on-chain bech32 address of the orchestrator's signing account.
    async fn address(&self) -> AccountId {
        self.client.read().await.nyxd.address()
    }

    /// Ensure the orchestrator has sufficient balance for transaction fees
    async fn verify_on_chain_balance(&self) -> anyhow::Result<()> {
        let address = self.address().await;
        let Some(balance) = self
            .client
            .read()
            .await
            .nyxd
            .get_balance(&address, "unym".to_string())
            .await?
        else {
            bail!("the orchestrator does not hold any unym balance");
        };
        if balance.amount < 1_000_000 {
            bail!(
                "the orchestrator does not hold sufficient amount of tokens. its current balance is {balance}"
            )
        }
        Ok(())
    }

    /// Waits for `contract` to confirm the orchestrator's account as authorised, returning what
    /// `check` found for it. `check` answers `None` while the account is not listed.
    ///
    /// Retries both on query failures (RPC flakiness) and on successful queries that don't list
    /// this orchestrator - the latter happens routinely when the admin has scheduled an
    /// authorisation transaction that hasn't landed yet, so giving it a bounded window to appear
    /// avoids crash-looping the process in that race. The total budget is
    /// `chain_authorisation_check_max_attempts` attempts spaced by
    /// `chain_authorisation_check_retry_delay`; once exhausted the function returns an error and
    /// `new()` aborts before any background tasks are spawned. A contract address missing from the
    /// network details fails at once.
    async fn await_chain_authorisation<T>(
        &self,
        contract: &str,
        check: impl AsyncFn(&AccountId) -> Result<Option<T>, NyxdError>,
    ) -> anyhow::Result<T> {
        let address = self.address().await;
        let max_attempts = self.config.chain_authorisation_check_max_attempts.get();
        let retry_delay = self.config.chain_authorisation_check_retry_delay;

        for attempt in 1..=max_attempts {
            match check(&address).await {
                Ok(Some(found)) => {
                    info!("orchestrator {address} is authorised in the {contract}");
                    return Ok(found);
                }
                Ok(None) => {
                    warn!(
                        attempt,
                        max_attempts,
                        "orchestrator {address} is not (yet) listed in the {contract}"
                    );
                }
                // a misconfiguration no retry can fix
                Err(err @ NyxdError::NoContractAddressAvailable(_)) => {
                    return Err(err).with_context(|| {
                        format!("cannot check the orchestrator's authorisation in the {contract}")
                    });
                }
                Err(err) => {
                    warn!(
                        attempt,
                        max_attempts,
                        "failed to query the {contract} for orchestrator authorisation: {err}"
                    );
                }
            }

            if attempt < max_attempts {
                sleep(retry_delay).await;
            }
        }

        bail!(
            "orchestrator {address} failed to confirm its authorisation in the {contract} after {max_attempts} attempts"
        )
    }

    /// Verifies that the orchestrator's account is authorised to send transactions to the network
    /// monitors contract (i.e. to authorise agents on-chain) and returns the identity key it has
    /// previously announced on-chain, if any.
    async fn verify_orchestrator_chain_authorisation(&self) -> anyhow::Result<Option<String>> {
        let query_client = self.client.read().await.nyxd.clone_query_client();
        self.await_chain_authorisation("network monitors contract", async |address| {
            let orchestrators = query_client.get_network_monitor_orchestrators().await?;
            Ok(orchestrators
                .authorised
                .into_iter()
                .find(|o| o.address.as_str() == address.as_ref())
                .map(|entry| entry.identity_key))
        })
        .await
    }

    /// Verifies that the orchestrator's account is an authorised network monitor in the performance
    /// contract, so a missing authorisation fails the deploy rather than an epoch of submissions.
    async fn verify_orchestrator_performance_authorisation(&self) -> anyhow::Result<()> {
        self.await_chain_authorisation("performance contract", async |address| {
            let monitor = self.client.get_network_monitor(address).await?;
            Ok(monitor.info.map(|_| ()))
        })
        .await
    }

    /// Ensures the identity key announced on-chain matches the key the orchestrator is running
    /// with. If the on-chain key is missing or stale, an update transaction is submitted so that
    /// agents and the nym-api can verify signatures against the correct key.
    async fn reconcile_announced_identity_key(
        &self,
        announced: Option<String>,
    ) -> anyhow::Result<()> {
        let current = self.identity_keys.public_key().to_base58_string();

        if announced.as_deref() == Some(current.as_str()) {
            info!("on-chain announced identity key matches the local identity key");
            return Ok(());
        }

        match &announced {
            Some(stale) => info!(
                "on-chain announced identity key ({stale}) does not match the local identity key ({current}); submitting an update"
            ),
            None => info!(
                "no identity key currently announced on-chain for this orchestrator; submitting the local one ({current})"
            ),
        }

        self.client
            .write()
            .await
            .nyxd
            .update_orchestrator_identity_key(current, None)
            .await
            .context(
                "failed to announce the orchestrator identity key to the network monitors contract",
            )?;

        info!("waiting for the key information to propagate");
        sleep(Duration::from_secs(30)).await;

        Ok(())
    }

    /// Verifies that the orchestrator's identity key is authorised to submit
    /// test results to the nym-api.
    async fn verify_orchestrator_nym_api_authorisation(&self) -> anyhow::Result<()> {
        // ensure our key is authorised to submit test results to the nym-api
        let auth_result = self
            .client
            .read()
            .await
            .nym_api
            .get_known_network_monitor(&self.identity_keys.public_key().to_base58_string())
            .await?;
        if !auth_result.authorised {
            bail!(
                "orchestrator identity key is not authorised to submit test results to the nym-api"
            );
        }
        Ok(())
    }

    /// Starts all orchestrator background tasks (HTTP server, node refresher, etc.)
    /// and blocks until a shutdown signal is received.
    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        // this shouldn't fail as we have no tasks using this client yet
        let query_client = self
            .client
            .try_read()
            .context("failed to acquire read lock on client")?
            .nyxd
            .clone_query_client();

        // the materialiser reads epochs and config-score params from the same contract. cloned off
        // the handle above rather than taken from the shared client, so it needs no lock and neither
        // task's queries wait behind the other's
        let materialiser_query_client = query_client.clone_query_client();

        // 1. build the shared state
        // 1.1. retrieve all registered agents (by this orchestrator) from the contract
        // (we assume the orchestrator has restarted and the agents are still out there as authorised)
        let address = self.address().await;
        let agents = query_client
            .get_all_network_monitor_agents()
            .await?
            .into_iter()
            .filter(|a| a.authorised_by.as_str() == address.as_ref())
            .collect::<Vec<_>>();
        let agents_state = KnownAgents::try_from(agents)?;
        let app_state = AppState::new(
            agents_state,
            self.storage.clone(),
            self.config.schedules(),
            self.client.clone(),
        );

        // 2. build node information refresher
        let node_refresher = NodeRefresher::new(
            &self.config,
            query_client.clone_query_client(),
            self.storage.clone(),
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 3. build the http server
        let http_router = build_router(
            app_state,
            self.agents_http_auth_token.clone(),
            self.metrics_and_results_http_auth_token.clone(),
        );

        // 4. build task for evicting stale test run results
        let stale_results_eviction = StaleResultsEviction::new(
            self.storage.clone(),
            self.config.testrun_eviction_age,
            self.config.aggregate_retention,
            self.config.test_timeout,
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 5. build the materialiser, which drives both the windowed probe aggregates and the
        //    config-score snapshot off one interval reading per tick, and submits them to the
        //    performance contract when enabled
        let submitter = self.config.performance_submission_enabled.then(|| {
            PerformanceSubmitter::new(
                self.client.clone(),
                self.storage.clone(),
                address.clone(),
                self.config.performance_submission_batch_size,
                self.config.performance_submission_coverage_threshold,
            )
        });
        let materialiser = Materialiser::new(
            MaterialiserConfig {
                windows: self.config.aggregation_windows,
                testrun_retention: self.config.testrun_eviction_age,
                minimum_balance: self.config.minimum_on_chain_balance.clone(),
                chain_interactions_penalty: self.config.chain_interactions_penalty,
            },
            self.storage.clone(),
            materialiser_query_client.clone_query_client(),
            materialiser_query_client,
            submitter,
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 5b. build the chain-capability refresher: keeps each node's on-chain standing (balance +
        //     feegrant) warm in its own cache so config-score materialisation only ever reads it. Its
        //     balances are queried in the denom of the minimum-balance config.
        let chain_capability_refresher = ChainCapabilityRefresher::new(
            self.config.chain_capability_config(),
            query_client,
            self.storage.clone(),
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 6. build task for submitting accumulated results to the nym-api
        let result_submitter = ResultSubmitter::new(
            self.client.read().await.nym_api.clone(),
            self.storage.clone(),
            self.identity_keys.clone(),
            self.config.result_submission_interval,
            self.config.result_submission_batch_size,
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 7. evict stale data before starting anything else so any test runs
        //    left "in progress" by a prior crashed/restarted orchestrator are
        //    freed up before agents start polling for work. Note: this is a
        //    blocking call — a hung DB at start-up will prevent the
        //    orchestrator from serving, which is the desired fail-fast here.
        stale_results_eviction
            .evict_stale_results()
            .await
            .context("failed to evict stale data")?;

        // 8. start all the tasks
        // http server
        let http_server_fut = run_http_server(
            http_router,
            self.config.http_server_bind_address,
            self.shutdown_manager.clone_shutdown_token(),
        );
        self.shutdown_manager
            .try_spawn_named(http_server_fut, "http-server");
        // node refresher
        self.shutdown_manager.try_spawn_named(
            async move {
                node_refresher.run().await;
            },
            "node-refresher",
        );
        // stale results eviction
        self.shutdown_manager.try_spawn_named(
            async move { stale_results_eviction.run().await },
            "stale-data-eviction",
        );
        // nym-api result submitter
        self.shutdown_manager.try_spawn_named(
            async move { result_submitter.run().await },
            "result-submitter",
        );
        // chain-capability cache refresher
        self.shutdown_manager.try_spawn_named(
            async move { chain_capability_refresher.run().await },
            "chain-capability-refresher",
        );
        // per-epoch materialisation (probe aggregates + config-score snapshot) and submission
        self.shutdown_manager
            .try_spawn_named(async move { materialiser.run().await }, "materialiser");

        self.shutdown_manager.run_until_shutdown().await;
        Ok(())
    }
}
