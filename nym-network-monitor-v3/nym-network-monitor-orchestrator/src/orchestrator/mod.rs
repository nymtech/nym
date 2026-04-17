// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// to be used in subsequent PRs
#![allow(dead_code)]

use crate::http::api::{build_router, run_http_server};
use crate::http::state::{AppState, KnownAgents};
use crate::orchestrator::config::Config;
use crate::orchestrator::node_refresher::NodeRefresher;
use crate::orchestrator::stale_results_eviction::StaleResultsEviction;
use crate::storage::NetworkMonitorStorage;
use anyhow::Context;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::nyxd::contract_traits::PagedNetworkMonitorsQueryClient;
use nym_validator_client::nyxd::{AccountId, bip39};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;
use zeroize::Zeroizing;

pub(crate) mod config;
mod node_refresher;
mod stale_results_eviction;
pub(crate) mod testruns;

pub(crate) struct NetworkMonitorOrchestrator {
    /// Runtime configuration for the orchestrator.
    pub(crate) config: Config,

    /// Validator client used to:
    /// - submit test results to the nym-api
    /// - query node information from the chain
    /// - send authorisation transactions to the network monitors contract
    pub(crate) client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,

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
    /// and verifies that the orchestrator is authorised on both the chain and the nym-api.
    pub(crate) async fn new(
        config: Config,
        identity_keys: Arc<ed25519::KeyPair>,
        agents_http_auth_token: Zeroizing<String>,
        metrics_and_results_http_auth_token: Zeroizing<String>,
        mnemonic: bip39::Mnemonic,
    ) -> anyhow::Result<Self> {
        let storage = NetworkMonitorStorage::init(&config.database_path).await?;

        let client_config = config.try_build_validator_client_config()?;
        let client = Arc::new(RwLock::new(
            DirectSigningHttpRpcValidatorClient::new_signing(client_config, mnemonic)?,
        ));

        let this = NetworkMonitorOrchestrator {
            config,
            client,
            identity_keys,
            agents_http_auth_token: Arc::new(agents_http_auth_token),
            metrics_and_results_http_auth_token: Arc::new(metrics_and_results_http_auth_token),
            storage,
            shutdown_manager: ShutdownManager::build_new_default()?,
        };
        this.verify_orchestrator_chain_authorisation().await?;
        this.verify_orchestrator_nym_api_authorisation().await?;

        Ok(this)
    }

    /// Returns the on-chain bech32 address of the orchestrator's signing account.
    async fn address(&self) -> AccountId {
        self.client.read().await.nyxd.address()
    }

    /// Verifies that the orchestrator's account is authorised to send transactions
    /// to the network monitors contract (i.e. to authorise agents on-chain).
    async fn verify_orchestrator_chain_authorisation(&self) -> anyhow::Result<()> {
        // ensure our address is authorised to send transactions
        // to the network monitors contract to authorise the agents
        error!("unimplemented");
        Ok(())
    }

    /// Verifies that the orchestrator's identity key is authorised to submit
    /// test results to the nym-api.
    async fn verify_orchestrator_nym_api_authorisation(&self) -> anyhow::Result<()> {
        // ensure our key is authorised to submit test results to the nym-api
        error!("unimplemented");
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
            self.config.test_interval,
            self.client.clone(),
        );

        // 2. build node information refresher
        let node_refresher = NodeRefresher::new(
            &self.config,
            query_client,
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
            self.config.test_timeout,
            self.shutdown_manager.clone_shutdown_token(),
        );

        // 5. evict stale data before starting anything else so any test runs
        //    left "in progress" by a prior crashed/restarted orchestrator are
        //    freed up before agents start polling for work. Note: this is a
        //    blocking call — a hung DB at start-up will prevent the
        //    orchestrator from serving, which is the desired fail-fast here.
        stale_results_eviction
            .evict_stale_results()
            .await
            .context("failed to evict stale data")?;

        // 6. start all the tasks
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

        self.shutdown_manager.run_until_shutdown().await;
        Ok(())
    }
}
