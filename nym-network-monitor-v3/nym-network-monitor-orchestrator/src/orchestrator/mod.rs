// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// to be used in subsequent PRs
#![allow(dead_code)]

use crate::orchestrator::config::Config;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{
    NewNymNode, NewTestRun, NymNode, TestRun, TestRunInProgress, TestType,
};
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownManager;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::nyxd::bip39;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::error;
use zeroize::Zeroizing;

pub(crate) mod config;
pub(crate) mod testruns;

pub(crate) struct NetworkMonitorOrchestrator {
    /// Runtime configuration for the orchestrator.
    pub(crate) config: Config,

    /// Validator client used to:
    /// - submit test results to the nym-api
    /// - query node information from the chain
    /// - send authorisation transactions to the network monitors contract
    pub(crate) client: DirectSigningHttpRpcValidatorClient,

    /// Ed25519 key pair used to sign result submissions to the nym-api.
    pub(crate) identity_keys: Arc<ed25519::KeyPair>,

    /// Bearer token presented by agents when requesting work assignments and submitting results.
    pub(crate) http_auth_token: Arc<Zeroizing<String>>,

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
        http_auth_token: Zeroizing<String>,
        mnemonic: bip39::Mnemonic,
    ) -> anyhow::Result<Self> {
        let storage = NetworkMonitorStorage::init(&config.database_path).await?;

        let client_config = config.try_build_validator_client_config()?;
        let client = DirectSigningHttpRpcValidatorClient::new_signing(client_config, mnemonic)?;

        let this = NetworkMonitorOrchestrator {
            config,
            client,
            identity_keys,
            http_auth_token: Arc::new(http_auth_token),
            storage,
            shutdown_manager: ShutdownManager::build_new_default()?,
        };
        this.verify_orchestrator_chain_authorisation().await?;
        this.verify_orchestrator_nym_api_authorisation().await?;

        Ok(this)
    }

    async fn verify_orchestrator_chain_authorisation(&self) -> anyhow::Result<()> {
        // ensure our address is authorised to send transactions
        // to the network monitors contract to authorise the agents
        error!("unimplemented");
        Ok(())
    }

    async fn verify_orchestrator_nym_api_authorisation(&self) -> anyhow::Result<()> {
        // ensure our key is authorised to submit test results to the nym-api
        error!("unimplemented");
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        let _ = &self.config;
        error!("unimplemented");
        self.make_clippy_happy().await?;
        self.shutdown_manager.run_until_shutdown().await;
        Ok(())
    }

    // a placeholder to make sure to use all types within the storage
    // without having to mark the whole module with allow(dead_code)
    pub(crate) async fn make_clippy_happy(&self) -> anyhow::Result<()> {
        let dummy_node = NewNymNode {
            node_id: 0,
            identity_key: "".to_string(),
            last_seen_bonded: OffsetDateTime::now_utc(),
            mixnet_socket_address: "".to_string(),
            noise_key: None,
            sphinx_key: None,
            key_rotation_id: None,
        };
        let dummy_testrun = NewTestRun {
            test_type: TestType::Mixnode,
            test_timestamp: OffsetDateTime::now_utc(),
            ingress_noise_handshake_us: None,
            egress_noise_handshake_us: None,
            packets_sent: 0,
            packets_received: 0,
            approximate_latency_us: None,
            packets_rtt_min_us: None,
            packets_rtt_mean_us: None,
            packets_rtt_max_us: None,
            packets_rtt_std_dev_us: None,
            sending_latency_min_us: None,
            sending_latency_mean_us: None,
            sending_latency_max_us: None,
            sending_latency_std_dev_us: None,
            received_duplicates: false,
            error: None,
        };

        self.storage.insert_or_update_nym_node(&dummy_node).await?;
        self.storage.insert_test_run(&dummy_testrun, 123).await?;
        self.storage
            .clear_timed_out_testruns_in_progress(self.config.test_timeout)
            .await?;
        self.storage
            .assign_next_testrun(self.config.test_interval)
            .await?;
        self.storage
            .evict_old_testruns(self.config.testrun_eviction_age)
            .await?;

        let nn = NymNode {
            inner: dummy_node,
            last_testrun: None,
        };
        let _ = nn.last_testrun;

        let tr = TestRun {
            id: 0,
            inner: dummy_testrun,
        };
        let _ = tr.id;
        let _ = tr.inner;

        let tr = TestRunInProgress {
            node_id: 0,
            started_at: OffsetDateTime::now_utc(),
        };
        let _ = tr.node_id;
        let _ = tr.started_at;

        Ok(())
    }
}
