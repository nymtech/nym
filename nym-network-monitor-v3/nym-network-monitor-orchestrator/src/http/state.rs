// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::NewTestRun;
use axum::extract::FromRef;
use nym_crypto::asymmetric::x25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_network_monitor_orchestrator_requests::models::{TestRunAssignment, TestRunResult};
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

/// Thread-safe cache of all agents known to this orchestrator, keyed by host IP.
/// Used to coordinate port assignments and validate announcements.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Returns a mixnet port for the agent identified by `host_ip` and `agent_pubkey`.
    /// If the agent was seen before, the previously assigned port is returned.
    /// Otherwise the first available port in the range
    /// `[DEFAULT_MIX_LISTENING_PORT, u16::MAX]` on that host is allocated.
    pub(crate) async fn assign_agent_port(
        &self,
        host_ip: IpAddr,
        agent_pubkey: x25519::PublicKey,
    ) -> Option<u16> {
        let mut guard = self.inner.lock().await;
        let host_agents = guard.agents.entry(host_ip).or_insert_with(Vec::new);

        // if this agent existed before, return the existing information
        if let Some(existing_agent) = host_agents.iter().find(|a| a.noise_key == agent_pubkey) {
            info!("reusing existing agent port for agent at {host_ip} with key {agent_pubkey}");
            return Some(existing_agent.mixnet_port);
        }

        // find the first available port in the valid range
        let occupied_ports: BTreeSet<u16> = host_agents.iter().map(|a| a.mixnet_port).collect();

        let next_port =
            (DEFAULT_MIX_LISTENING_PORT..=u16::MAX).find(|p| !occupied_ports.contains(p))?;

        // insert agent information into the cache
        host_agents.push(KnownAgent {
            mixnet_port: next_port,
            last_active_at: OffsetDateTime::now_utc(),
            noise_key: agent_pubkey,
            announced: false,
        });

        Some(next_port)
    }

    pub(crate) async fn get_agent(&self, address: SocketAddr) -> Option<KnownAgent> {
        let guard = self.inner.lock().await;
        let host_agents = guard.agents.get(&address.ip())?;

        host_agents
            .iter()
            .find(|a| a.mixnet_port == address.port())
            .copied()
    }

    /// Validates and marks the agent at `mix_listener` as announced.
    ///
    /// Returns:
    /// - `Err` if no agent with that address exists (orchestrator may have restarted).
    /// - `Ok(true)` if the agent was already announced (caller should skip the contract tx).
    /// - `Ok(false)` if the agent exists but hasn't been announced yet (caller should
    ///   proceed with the contract tx and call [`mark_announced`] on success).
    ///
    /// Also verifies that the provided `noise_key` matches the one stored during port
    /// assignment — returns `Err` on mismatch.
    pub(crate) async fn try_announce_agent(
        &self,
        mix_listener: SocketAddr,
        noise_key: x25519::PublicKey,
    ) -> Result<bool, AgentAnnounceError> {
        let mut guard = self.inner.lock().await;
        let host_agents = guard
            .agents
            .get_mut(&mix_listener.ip())
            .ok_or(AgentAnnounceError::NotFound)?;

        let agent = host_agents
            .iter_mut()
            .find(|agent| agent.mixnet_port == mix_listener.port())
            .ok_or(AgentAnnounceError::NotFound)?;

        if agent.noise_key != noise_key {
            return Err(AgentAnnounceError::NoiseKeyMismatch);
        }

        agent.last_active_at = OffsetDateTime::now_utc();

        if agent.announced {
            return Ok(true);
        }

        Ok(false)
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
    }
}

#[derive(Debug)]
pub(crate) enum AgentAnnounceError {
    /// No agent with the given socket address exists in the cache.
    NotFound,
    /// The noise key in the request doesn't match the one from port assignment.
    NoiseKeyMismatch,
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

        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(KnownAgentsInner { agents: agents_map })),
        })
    }
}

/// Inner state behind the [`KnownAgents`] mutex.
#[derive(Default)]
struct KnownAgentsInner {
    /// Map from host IP to the list of agents running on that host.
    agents: HashMap<IpAddr, Vec<KnownAgent>>,
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

#[derive(Clone)]
pub(crate) struct TestrunManager {
    storage: NetworkMonitorStorage,
    testrun_staleness_age: Duration,
}

impl TestrunManager {
    pub(crate) async fn assign_next_testrun(&self) -> Result<Option<TestRunAssignment>, ApiError> {
        let node_to_test = match self
            .storage
            .assign_next_testrun(self.testrun_staleness_age)
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

    pub(crate) async fn submit_testrun_result(
        &self,
        result: TestRunResult,
        node_id: NodeId,
    ) -> Result<(), ApiError> {
        // currently all testruns are mixnode results
        let testrun = NewTestRun::from_mixnode_result(result);
        if let Err(err) = self.storage.insert_test_run(&testrun, node_id).await {
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
            testrun_manager: TestrunManager {
                storage,
                testrun_staleness_age,
            },
            validator_client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::x25519;
    use rand::rngs::OsRng;

    fn random_pubkey() -> x25519::PublicKey {
        *x25519::KeyPair::new(&mut OsRng).public_key()
    }

    const HOST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1));
    const HOST_B: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2));

    #[tokio::test]
    async fn first_agent_gets_default_port() {
        let agents = KnownAgents::default();
        let port = agents.assign_agent_port(HOST, random_pubkey()).await;
        assert_eq!(port, Some(DEFAULT_MIX_LISTENING_PORT));
    }

    #[tokio::test]
    async fn second_agent_same_host_gets_next_port() {
        let agents = KnownAgents::default();
        let key_a = random_pubkey();
        let key_b = random_pubkey();

        let port_a = agents.assign_agent_port(HOST, key_a).await.unwrap();
        let port_b = agents.assign_agent_port(HOST, key_b).await.unwrap();

        assert_eq!(port_a, DEFAULT_MIX_LISTENING_PORT);
        assert_eq!(port_b, DEFAULT_MIX_LISTENING_PORT + 1);
    }

    #[tokio::test]
    async fn same_key_returns_same_port() {
        let agents = KnownAgents::default();
        let key = random_pubkey();

        let first = agents.assign_agent_port(HOST, key).await.unwrap();
        let second = agents.assign_agent_port(HOST, key).await.unwrap();

        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn different_hosts_get_independent_ports() {
        let agents = KnownAgents::default();
        let key_a = random_pubkey();
        let key_b = random_pubkey();

        let port_a = agents.assign_agent_port(HOST, key_a).await.unwrap();
        let port_b = agents.assign_agent_port(HOST_B, key_b).await.unwrap();

        assert_eq!(port_a, DEFAULT_MIX_LISTENING_PORT);
        assert_eq!(port_b, DEFAULT_MIX_LISTENING_PORT);
    }

    #[tokio::test]
    async fn try_announce_unknown_agent_returns_not_found() {
        let agents = KnownAgents::default();
        let addr: SocketAddr = "10.0.0.1:1789".parse().unwrap();

        let result = agents.try_announce_agent(addr, random_pubkey()).await;
        assert!(matches!(result, Err(AgentAnnounceError::NotFound)));
    }

    #[tokio::test]
    async fn try_announce_wrong_key_returns_mismatch() {
        let agents = KnownAgents::default();
        let real_key = random_pubkey();
        let wrong_key = random_pubkey();

        let port = agents.assign_agent_port(HOST, real_key).await.unwrap();
        let addr = SocketAddr::new(HOST, port);

        let result = agents.try_announce_agent(addr, wrong_key).await;
        assert!(matches!(result, Err(AgentAnnounceError::NoiseKeyMismatch)));
    }

    #[tokio::test]
    async fn try_announce_returns_false_then_true_after_mark() {
        let agents = KnownAgents::default();
        let key = random_pubkey();

        let port = agents.assign_agent_port(HOST, key).await.unwrap();
        let addr = SocketAddr::new(HOST, port);

        // first announce: not yet announced
        let already = agents.try_announce_agent(addr, key).await.unwrap();
        assert!(!already);

        // mark as announced
        agents.mark_announced(addr).await;

        // second announce: already announced
        let already = agents.try_announce_agent(addr, key).await.unwrap();
        assert!(already);
    }

    #[tokio::test]
    async fn port_reuse_after_gap() {
        // Simulate: agent on default port is known, next port is assigned,
        // then verify a third agent gets default+2
        let agents = KnownAgents::default();
        let key_a = random_pubkey();
        let key_b = random_pubkey();
        let key_c = random_pubkey();

        let p1 = agents.assign_agent_port(HOST, key_a).await.unwrap();
        let p2 = agents.assign_agent_port(HOST, key_b).await.unwrap();
        let p3 = agents.assign_agent_port(HOST, key_c).await.unwrap();

        assert_eq!(p1, DEFAULT_MIX_LISTENING_PORT);
        assert_eq!(p2, DEFAULT_MIX_LISTENING_PORT + 1);
        assert_eq!(p3, DEFAULT_MIX_LISTENING_PORT + 2);
    }
}
