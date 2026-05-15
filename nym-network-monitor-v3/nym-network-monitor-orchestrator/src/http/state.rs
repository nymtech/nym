// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::NewTestRun;
use axum::extract::FromRef;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::{
    NymNodeData, NymNodeWithTestRun, PagedResult, Pagination, TestRunAssignment, TestRunData,
    TestRunInProgressData, TestRunResult,
};
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

/// Thread-safe cache of all agents known to this orchestrator, keyed by host IP.
/// Used to short-circuit the contract tx for already-announced agents.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Looks up an agent by its full mixnet socket address (host IP + port).
    /// Returns `None` if no agent is registered at that address.
    pub(crate) async fn get_agent(&self, address: SocketAddr) -> Option<KnownAgent> {
        let guard = self.inner.lock().await;
        let host_agents = guard.agents.get(&address.ip())?;

        host_agents
            .iter()
            .find(|a| a.mixnet_port == address.port())
            .copied()
    }

    /// Records an announcement from the agent at `mix_listener`. The cache entry
    /// is upserted: a missing entry is inserted, and if the cached noise key differs
    /// from the announced one it is overwritten and the agent is treated as
    /// not-yet-announced so the caller re-runs the contract tx with the new key.
    ///
    /// Returns the current `announced` flag: `true` means the agent was already
    /// announced to the contract and the caller should skip the contract tx;
    /// `false` means the caller should submit the tx and call [`Self::mark_announced`]
    /// on success.
    pub(crate) async fn try_announce_agent(
        &self,
        mix_listener: SocketAddr,
        noise_key: x25519::PublicKey,
    ) -> bool {
        let mut guard = self.inner.lock().await;
        let host_agents = guard.agents.entry(mix_listener.ip()).or_default();

        if let Some(agent) = host_agents
            .iter_mut()
            .find(|agent| agent.mixnet_port == mix_listener.port())
        {
            agent.last_active_at = OffsetDateTime::now_utc();
            if agent.noise_key == noise_key {
                return agent.announced;
            }
            agent.noise_key = noise_key;
            agent.announced = false;
            guard.publish_gauges();
            return false;
        }

        host_agents.push(KnownAgent {
            mixnet_port: mix_listener.port(),
            last_active_at: OffsetDateTime::now_utc(),
            noise_key,
            announced: false,
        });
        guard.publish_gauges();
        false
    }

    /// Marks the agent at `mix_listener` as announced. Should be called after a
    /// successful contract transaction.
    pub(crate) async fn mark_announced(&self, mix_listener: SocketAddr) {
        let mut guard = self.inner.lock().await;
        let Some(host_agents) = guard.agents.get_mut(&mix_listener.ip()) else {
            return;
        };
        if let Some(agent) = host_agents
            .iter_mut()
            .find(|a| a.mixnet_port == mix_listener.port())
        {
            agent.announced = true;
        }
        guard.publish_gauges();
    }
}

/// Rebuilds the agent cache from on-chain data. Used at orchestrator startup to
/// restore state for agents that were authorised before a restart.
impl TryFrom<Vec<AuthorisedNetworkMonitor>> for KnownAgents {
    type Error = anyhow::Error;

    fn try_from(agents: Vec<AuthorisedNetworkMonitor>) -> Result<Self, Self::Error> {
        let mut agents_map = HashMap::new();

        for agent in agents {
            let host_ip = agent.mixnet_address.ip();
            let noise_key = x25519::PublicKey::from_base58_string(&agent.bs58_x25519_noise)?;
            agents_map
                .entry(host_ip)
                .or_insert_with(Vec::new)
                .push(KnownAgent {
                    mixnet_port: agent.mixnet_address.port(),
                    // or should we use the authorisation ts?
                    last_active_at: OffsetDateTime::now_utc(),
                    noise_key,
                    announced: true,
                });
        }

        let inner = KnownAgentsInner { agents: agents_map };
        inner.publish_gauges();
        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(inner)),
        })
    }
}

/// Inner state behind the [`KnownAgents`] mutex.
#[derive(Default)]
struct KnownAgentsInner {
    /// Map from host IP to the list of agents running on that host.
    agents: HashMap<IpAddr, Vec<KnownAgent>>,
}

impl KnownAgentsInner {
    /// Recomputes and publishes the `known_agents_*` gauges. Called from every mutation of
    /// the inner map — we recount rather than incrementally adjust so the gauges stay correct
    /// even if a future code path mutates state without going through a dedicated helper.
    fn publish_gauges(&self) {
        let (total, announced) =
            self.agents
                .values()
                .fold((0i64, 0i64), |(total, announced), agents| {
                    let t = total + agents.len() as i64;
                    let a = announced + agents.iter().filter(|a| a.announced).count() as i64;
                    (t, a)
                });
        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsTotal, total);
        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsAnnounced, announced);
    }
}

/// Cached state of a single known agent on a particular host.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownAgent {
    pub(crate) mixnet_port: u16,
    pub(crate) last_active_at: OffsetDateTime,
    pub(crate) noise_key: x25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract.
    /// Set to `true` when restored from the chain at startup, or after a successful
    /// `/announce` contract transaction.
    pub(crate) announced: bool,
}

/// Coordinates test run assignment and result storage.
///
/// Wraps the underlying [`NetworkMonitorStorage`] and applies the configured
/// `testrun_staleness_age` when deciding which nodes are eligible for testing.
#[derive(Clone)]
pub(crate) struct TestrunManager {
    /// Minimum time that must elapse after a node's last test before it becomes
    /// eligible for another one. Passed to the storage layer as a staleness gate.
    testrun_staleness_age: Duration,
}

impl TestrunManager {
    /// Selects the most stale idle mixnode and atomically marks it as having a test
    /// in progress. Returns `None` if no mixnode is currently eligible.
    async fn assign_next_mixnode_testrun(
        &self,
        storage: &NetworkMonitorStorage,
    ) -> Result<Option<TestRunAssignment>, ApiError> {
        let node_to_test = match storage
            .assign_next_mixnode_testrun(self.testrun_staleness_age)
            .await
        {
            Ok(node) => node,
            Err(err) => {
                error!("testrun assignment storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
        };

        let Some(node) = node_to_test.map(|n| n.inner) else {
            return Ok(None);
        };

        let (Some(address), Some(noise_key), Some(sphinx_key), Some(key_rotation)) = (
            node.mixnet_socket_address,
            node.noise_key,
            node.sphinx_key,
            node.key_rotation_id,
        ) else {
            // this should never happen as the db query should ignore entries where those fields are set to NULL
            error!(
                "database inconsistency - attempted to assign node {} for stress testing, but we don't have its complete data",
                node.node_id
            );
            return Err(ApiError::StorageFailure);
        };

        let Ok(node_address) = address.parse() else {
            return Err(ApiError::MalformedStoredData);
        };

        let Ok(noise_key) = noise_key.parse() else {
            return Err(ApiError::MalformedStoredData);
        };

        let Ok(sphinx_key) = sphinx_key.parse() else {
            return Err(ApiError::MalformedStoredData);
        };

        Ok(Some(TestRunAssignment {
            node_id: node.node_id as u32,
            node_address,
            noise_key,
            sphinx_key,
            key_rotation_id: key_rotation as u32,
        }))
    }

    /// Persists a completed test run result to the database and updates the
    /// node's `last_testrun` pointer.
    async fn submit_testrun_result(
        &self,
        storage: &NetworkMonitorStorage,
        result: TestRunResult,
        node_id: NodeId,
    ) -> Result<(), ApiError> {
        // currently all testruns are mixnode results
        let testrun = NewTestRun::from_mixnode_result(node_id, result);
        if let Err(err) = storage.insert_test_run(&testrun).await {
            error!("testrun result storage failure: {err}");
            return Err(ApiError::StorageFailure);
        }
        Ok(())
    }
}

/// Shared application state available to all axum request handlers.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub(crate) agents: KnownAgents,

    pub(crate) testrun_manager: TestrunManager,

    pub(crate) storage: NetworkMonitorStorage,

    pub(crate) validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
}

impl AppState {
    pub(crate) fn new(
        agents: KnownAgents,
        storage: NetworkMonitorStorage,
        testrun_staleness_age: Duration,
        validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
    ) -> Self {
        AppState {
            agents,
            storage,
            testrun_manager: TestrunManager {
                testrun_staleness_age,
            },
            validator_client,
        }
    }

    /// Selects the most stale idle mixnode and atomically marks it as having a test
    /// in progress. Returns `None` if no mixnode is currently eligible.
    pub(crate) async fn assign_next_mixnode_testrun(
        &self,
    ) -> Result<Option<TestRunAssignment>, ApiError> {
        self.testrun_manager
            .assign_next_mixnode_testrun(&self.storage)
            .await
    }

    /// Persists a completed test run result to the database and updates the
    /// node's `last_testrun` pointer.
    pub(crate) async fn submit_testrun_result(
        &self,
        result: TestRunResult,
        node_id: NodeId,
    ) -> Result<(), ApiError> {
        self.testrun_manager
            .submit_testrun_result(&self.storage, result, node_id)
            .await
    }

    /// Backs `GET /v1/results/testrun/{id}`. `Ok(None)` means the row doesn't
    /// exist (the handler maps this to a 404); storage errors are logged and
    /// collapsed to [`ApiError::StorageFailure`].
    pub(crate) async fn get_testrun_by_id(&self, id: i64) -> Result<Option<TestRunData>, ApiError> {
        let result = match self.storage.get_testrun_by_id(id).await {
            Err(err) => {
                error!("get_testrun_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(testrun)) => testrun,
        };

        Ok(Some(result.into()))
    }

    /// Backs `GET /v1/results/nym-node/{node_id}`. If the node is known, its
    /// snapshot is returned along with the most recent completed test run
    /// (fetched in a second query via [`Self::get_testrun_by_id`]);
    /// `latest_test_run` is `None` when no such run exists.
    ///
    /// Malformed stored data (e.g. an unparsable base58 key) is surfaced as
    /// [`ApiError::MalformedStoredData`]; this should never happen in practice
    /// because the orchestrator writes these fields itself.
    pub(crate) async fn get_nym_node_by_id(
        &self,
        node_id: NodeId,
    ) -> Result<Option<NymNodeWithTestRun>, ApiError> {
        let nym_node = match self.storage.get_nym_node_by_id(node_id).await {
            Err(err) => {
                error!("get_nym_node_by_id storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(None) => return Ok(None),
            Ok(Some(nym_node)) => nym_node,
        };

        let latest_test_run = match nym_node.last_testrun {
            None => None,
            Some(testrun_id) => self.get_testrun_by_id(testrun_id).await?,
        };

        Ok(Some(NymNodeWithTestRun {
            node: nym_node.try_into().map_err(|err| {
                error!("get_nym_node_by_id malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?,
            latest_test_run,
        }))
    }

    /// Backs `GET /v1/results/testruns-in-progress`. Returns a page of rows
    /// from `testrun_in_progress` ordered oldest `started_at` first so stale
    /// runs surface at the top.
    pub(crate) async fn get_testruns_in_progress_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunInProgressData>, ApiError> {
        let (in_progress, total) = match self
            .storage
            .get_testruns_in_progress_paginated(pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_in_progress_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(result) => result,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: in_progress.len(),
            total,
            items: in_progress.into_iter().map(Into::into).collect(),
        })
    }

    /// Backs `GET /v1/results/testruns`. Returns a single page of completed
    /// runs ordered newest first, together with the total row count at the
    /// time the page was read (fetched in the same transaction as the page
    /// itself for consistency).
    pub(crate) async fn get_testruns_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self.storage.get_testruns_paginated(pagination).await {
            Err(err) => {
                error!("get_testruns_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok(testruns) => testruns,
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns.into_iter().map(Into::into).collect(),
        })
    }

    /// Backs `GET /v1/results/nym-nodes`. Returns a page of nodes ordered by
    /// `node_id` ascending. Each row is converted to [`NymNodeData`] via the
    /// fallible `TryFrom` impl that decodes stored base58 keys; a failure
    /// anywhere in the page produces [`ApiError::MalformedStoredData`].
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PagedResult<NymNodeData>, ApiError> {
        let (nym_nodes, total) = match self.storage.get_nym_nodes_paginated(pagination).await {
            Err(err) => {
                error!("get_nym_nodes_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((nym_nodes, total)) => (nym_nodes, total),
        };

        let mut items = Vec::with_capacity(nym_nodes.len());
        for node in nym_nodes {
            items.push(node.try_into().map_err(|err| {
                error!("get_nym_nodes_paginated malformed stored data: {err}");
                ApiError::MalformedStoredData
            })?);
        }

        Ok(PagedResult {
            page: pagination.page(),
            per_page: items.len(),
            total,
            items,
        })
    }

    /// Backs `GET /v1/results/nym-node/{node_id}/testruns`. Returns a page of
    /// completed runs for a single node ordered newest first. Unknown or
    /// never-tested nodes produce a valid empty page (`total: 0`) rather than
    /// a 404.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        node_id: NodeId,
        pagination: Pagination,
    ) -> Result<PagedResult<TestRunData>, ApiError> {
        let (testruns, total) = match self
            .storage
            .get_testruns_for_node_paginated(node_id, pagination)
            .await
        {
            Err(err) => {
                error!("get_testruns_for_node_paginated storage failure: {err}");
                return Err(ApiError::StorageFailure);
            }
            Ok((testruns, total)) => (testruns, total),
        };

        Ok(PagedResult {
            page: pagination.page(),
            per_page: testruns.len(),
            total,
            items: testruns.into_iter().map(Into::into).collect(),
        })
    }
}
