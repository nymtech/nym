// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::NewTestRun;
use axum::extract::FromRef;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::{
    AgentMixAddresses, NymNodeData, NymNodeWithTestRun, PagedResult, Pagination, TestRunAssignment,
    TestRunData, TestRunInProgressData, TestRunResult,
};
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, warn};

/// Thread-safe cache of all agents known to this orchestrator, keyed by the agent's IPv4 mixnet
/// address. Used to short-circuit the contract txs for already-announced agents.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Looks up an agent by the address pair it announced. Returns `None` if nothing is registered
    /// under its IPv4 address, or if the registered entry belongs to a different IPv6 address, i.e.
    /// this is not the agent we authorised.
    pub(crate) async fn get_agent(&self, addresses: AgentMixAddresses) -> Option<KnownAgent> {
        let guard = self.inner.lock().await;
        let agent = guard.agents.get(&addresses.v4)?;

        if agent.mix_v6 != addresses.v6 {
            return None;
        }

        Some(*agent)
    }

    /// Records an announcement from the agent at `addresses`. The cache entry is upserted: a
    /// missing entry is inserted, and if the cached noise key or IPv6 address differs from the
    /// announced one it is overwritten and the agent is treated as not-yet-announced so the caller
    /// re-runs the contract txs with the new details.
    ///
    /// Returns the current `announced` flag: `true` means the agent was already announced to the
    /// contract and the caller should skip the contract txs; `false` means the caller should submit
    /// them and call [`Self::mark_announced`] on success.
    pub(crate) async fn try_announce_agent(
        &self,
        addresses: AgentMixAddresses,
        noise_key: x25519::PublicKey,
    ) -> bool {
        let mut guard = self.inner.lock().await;

        match guard.agents.entry(addresses.v4) {
            Entry::Occupied(mut entry) => {
                let agent = entry.get_mut();
                agent.last_active_at = OffsetDateTime::now_utc();

                if agent.noise_key == noise_key && agent.mix_v6 == addresses.v6 {
                    return agent.announced;
                }

                // the addresses and the noise key are all meant to be stable for the lifetime of
                // an agent, so this is either a re-provisioned agent reusing its IPv4 address or a
                // live one whose configuration changed. we can't distinguish those (announcements
                // are bearer-token authenticated, not key authenticated), so the announcement is
                // accepted - but a superseded IPv6 address stays authorised in the contract, which
                // is worth knowing about.
                warn!(
                    "agent at {} announced details differing from the cached ones (cached: {} / {}, announced: {} / {}) - re-announcing it to the contract",
                    addresses.v4,
                    agent.mix_v6,
                    agent.noise_key.to_base58_string(),
                    addresses.v6,
                    noise_key.to_base58_string(),
                );
                PROMETHEUS_METRICS.inc(PrometheusMetric::AgentDetailsChanged);

                agent.mix_v6 = addresses.v6;
                agent.noise_key = noise_key;
                agent.announced = false;
            }
            Entry::Vacant(entry) => {
                entry.insert(KnownAgent {
                    mix_v6: addresses.v6,
                    last_active_at: OffsetDateTime::now_utc(),
                    noise_key,
                    announced: false,
                });
            }
        }

        guard.publish_gauges();
        false
    }

    /// Marks the agent at `addresses` as announced. Should be called once every contract
    /// transaction for that agent has succeeded.
    pub(crate) async fn mark_announced(&self, addresses: AgentMixAddresses) {
        let mut guard = self.inner.lock().await;
        if let Some(agent) = guard.agents.get_mut(&addresses.v4)
            && agent.mix_v6 == addresses.v6
        {
            agent.announced = true;
        }
        guard.publish_gauges();
    }
}

/// Rebuilds the agent cache from on-chain data. Used at orchestrator startup to
/// restore state for agents that were authorised before a restart.
///
/// The contract holds one entry per socket address and nothing ties together the two entries
/// belonging to a single agent, so the pairs are recovered by grouping on the noise key, which is
/// unique per agent. Records that don't form exactly one IPv4/IPv6 pair are dropped: they are
/// either authorisations predating the IPv6 announcement, or leftovers from an agent that has since
/// changed one of its addresses. Dropping them is safe because this cache exists purely to skip
/// redundant contract transactions - agents always announce before requesting work, which
/// re-creates the entry at the cost of one extra transaction.
impl TryFrom<Vec<AuthorisedNetworkMonitor>> for KnownAgents {
    type Error = anyhow::Error;

    fn try_from(agents: Vec<AuthorisedNetworkMonitor>) -> Result<Self, Self::Error> {
        let mut by_noise_key: HashMap<String, Vec<SocketAddr>> = HashMap::new();
        for agent in agents {
            by_noise_key
                .entry(agent.bs58_x25519_noise)
                .or_default()
                .push(agent.mixnet_address);
        }

        let mut agents_map = HashMap::new();
        for (bs58_noise_key, addresses) in by_noise_key {
            let (v4_addresses, v6_addresses): (Vec<_>, Vec<_>) =
                addresses.iter().copied().partition(|addr| addr.is_ipv4());

            let ([mix_v4], [mix_v6]) = (v4_addresses.as_slice(), v6_addresses.as_slice()) else {
                error!(
                    "the agent using noise key {bs58_noise_key} has {} authorised address(es) on chain ({addresses:?}) rather than a single ipv4/ipv6 pair - ignoring it until it re-announces itself",
                    addresses.len()
                );
                continue;
            };

            let noise_key = x25519::PublicKey::from_base58_string(&bs58_noise_key)?;
            agents_map.insert(
                *mix_v4,
                KnownAgent {
                    mix_v6: *mix_v6,
                    // the on-chain authorisation timestamp says nothing about liveness,
                    // so treat a restored entry as freshly active
                    last_active_at: OffsetDateTime::now_utc(),
                    noise_key,
                    announced: true,
                },
            );
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
    /// Map from the agent's IPv4 mixnet address to its cached state. The port is part of the key
    /// because several agents may share a host IP.
    agents: HashMap<SocketAddr, KnownAgent>,
}

impl KnownAgentsInner {
    /// Recomputes and publishes the `known_agents_*` gauges. Called from every mutation of
    /// the inner map — we recount rather than incrementally adjust so the gauges stay correct
    /// even if a future code path mutates state without going through a dedicated helper.
    fn publish_gauges(&self) {
        let total = self.agents.len() as i64;
        let announced = self.agents.values().filter(|a| a.announced).count() as i64;

        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsTotal, total);
        PROMETHEUS_METRICS.set(PrometheusMetric::KnownAgentsAnnounced, announced);
    }
}

/// Cached state of a single known agent, i.e. of the pair of contract authorisations it holds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownAgent {
    /// The IPv6 mixnet address announced alongside the IPv4 address this entry is keyed by.
    pub(crate) mix_v6: SocketAddr,

    pub(crate) last_active_at: OffsetDateTime,
    pub(crate) noise_key: x25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract, under both of
    /// its addresses. Set to `true` when restored from the chain at startup, or after a successful
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

        let Some(assigned) = node_to_test else {
            return Ok(None);
        };
        let node_ips = assigned.node.announced_ips();
        let tested_ip = assigned.tested_ip;
        let node = assigned.node.inner;

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

        // the stored socket address only contributes the mix port - the address to test comes from
        // the rotation over everything the node announced
        let Ok(node_address) = address.parse::<SocketAddr>() else {
            return Err(ApiError::MalformedStoredData);
        };
        let node_address = SocketAddr::new(tested_ip, node_address.port());

        let Ok(noise_key) = noise_key.parse() else {
            return Err(ApiError::MalformedStoredData);
        };

        let Ok(sphinx_key) = sphinx_key.parse() else {
            return Err(ApiError::MalformedStoredData);
        };

        Ok(Some(TestRunAssignment {
            node_id: node.node_id as u32,
            node_address,
            node_ips,
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
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        // currently all testruns are mixnode results
        let testrun = NewTestRun::from_mixnode_result(node_id, tested_address, result);
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
        tested_address: SocketAddr,
    ) -> Result<(), ApiError> {
        self.testrun_manager
            .submit_testrun_result(&self.storage, result, node_id, tested_address)
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr, Timestamp};
    use nym_test_utils::helpers::seeded_rng;

    fn noise_key(seed: u8) -> x25519::PublicKey {
        x25519::PublicKey::from(&x25519::PrivateKey::new(&mut seeded_rng([seed; 32])))
    }

    fn addresses(port: u16) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: format!("1.1.1.1:{port}").parse().unwrap(),
            v6: format!("[aaaa::{port}]:{port}").parse().unwrap(),
        }
    }

    fn authorisation(
        address: SocketAddr,
        noise_key: x25519::PublicKey,
    ) -> AuthorisedNetworkMonitor {
        AuthorisedNetworkMonitor {
            mixnet_address: address,
            authorised_by: Addr::unchecked("n1foomp"),
            authorised_at: Timestamp::from_seconds(42),
            bs58_x25519_noise: noise_key.to_base58_string(),
            noise_version: 1,
        }
    }

    #[tokio::test]
    async fn agent_is_not_announced_until_marked() {
        let agents = KnownAgents::default();
        let addresses = addresses(1789);
        let key = noise_key(1);

        assert!(!agents.try_announce_agent(addresses, key).await);
        assert!(!agents.get_agent(addresses).await.unwrap().announced);

        agents.mark_announced(addresses).await;
        assert!(agents.get_agent(addresses).await.unwrap().announced);

        // a repeated announcement is now a no-op, so the caller skips the contract txs
        assert!(agents.try_announce_agent(addresses, key).await);
    }

    // agents deployed on the same host share its ipv4 address and are only told apart by the port,
    // so an announcement from one must not disturb the other
    #[tokio::test]
    async fn agents_sharing_a_host_ip_are_tracked_separately() {
        let agents = KnownAgents::default();
        let first = addresses(1789);
        let second = addresses(1790);
        assert_eq!(first.v4.ip(), second.v4.ip());

        agents.try_announce_agent(first, noise_key(1)).await;
        agents.mark_announced(first).await;
        agents.try_announce_agent(second, noise_key(2)).await;

        assert!(agents.get_agent(first).await.unwrap().announced);
        assert!(!agents.get_agent(second).await.unwrap().announced);
    }

    // the entry belongs to the announced pair, not to the ipv4 address alone
    #[tokio::test]
    async fn lookup_with_a_different_v6_address_finds_nothing() {
        let agents = KnownAgents::default();
        let announced = addresses(1789);
        let key = noise_key(1);

        agents.try_announce_agent(announced, key).await;
        agents.mark_announced(announced).await;

        let other_v6 = AgentMixAddresses {
            v6: "[bbbb::1]:1789".parse().unwrap(),
            ..announced
        };
        assert!(agents.get_agent(other_v6).await.is_none());
    }

    #[tokio::test]
    async fn changed_details_require_a_re_announcement() {
        for changed in [
            AgentMixAddresses {
                v6: "[bbbb::1]:1789".parse().unwrap(),
                ..addresses(1789)
            },
            addresses(1789),
        ] {
            let agents = KnownAgents::default();
            let announced = addresses(1789);
            agents.try_announce_agent(announced, noise_key(1)).await;
            agents.mark_announced(announced).await;

            // either the v6 address or the noise key differs from what we have cached
            let key = if changed.v6 == announced.v6 {
                noise_key(2)
            } else {
                noise_key(1)
            };
            assert!(!agents.try_announce_agent(changed, key).await);

            let agent = agents.get_agent(changed).await.unwrap();
            assert!(!agent.announced);
            assert_eq!(agent.mix_v6, changed.v6);
            assert_eq!(agent.noise_key, key);
        }
    }

    #[tokio::test]
    async fn on_chain_pairs_are_recovered_via_the_noise_key() {
        let first = addresses(1789);
        let second = addresses(1790);
        let (first_key, second_key) = (noise_key(1), noise_key(2));

        let restored = KnownAgents::try_from(vec![
            authorisation(second.v6, second_key),
            authorisation(first.v4, first_key),
            authorisation(second.v4, second_key),
            authorisation(first.v6, first_key),
        ])
        .unwrap();

        let restored_first = restored.get_agent(first).await.unwrap();
        assert_eq!(restored_first.mix_v6, first.v6);
        assert_eq!(restored_first.noise_key, first_key);
        assert!(restored_first.announced);

        let restored_second = restored.get_agent(second).await.unwrap();
        assert_eq!(restored_second.mix_v6, second.v6);
        assert_eq!(restored_second.noise_key, second_key);
        assert!(restored_second.announced);
    }

    // on-chain records that don't form an ipv4/ipv6 pair (authorisations predating the ipv6
    // announcement, or an address the agent no longer announces) are dropped, so the agent
    // re-announces itself and pays for one extra contract tx
    #[tokio::test]
    async fn unpaired_on_chain_records_are_dropped() {
        let agent = addresses(1789);
        let key = noise_key(1);

        let v4_only = KnownAgents::try_from(vec![authorisation(agent.v4, key)]).unwrap();
        assert!(v4_only.get_agent(agent).await.is_none());

        let stale_v6 = KnownAgents::try_from(vec![
            authorisation(agent.v4, key),
            authorisation(agent.v6, key),
            authorisation("[bbbb::1]:1789".parse().unwrap(), key),
        ])
        .unwrap();
        assert!(stale_v6.get_agent(agent).await.is_none());
    }
}
