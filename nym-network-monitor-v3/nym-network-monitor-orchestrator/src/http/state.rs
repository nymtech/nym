// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::FromRef;
use nym_crypto::asymmetric::x25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

/// Thread-safe cache of all agents known to this orchestrator, keyed by host IP.
/// Used to coordinate port assignments and validate announcements.
#[derive(Clone)]
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
            host_ip,
            mixnet_port: next_port,
            last_active_at: OffsetDateTime::now_utc(),
            noise_key: agent_pubkey,
            announced: false,
        });

        Some(next_port)
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
                    host_ip,
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
struct KnownAgentsInner {
    /// Map from host IP to the list of agents running on that host.
    agents: HashMap<IpAddr, Vec<KnownAgent>>,
}

/// Cached state of a single known agent on a particular host.
#[allow(dead_code)]
struct KnownAgent {
    host_ip: IpAddr,
    mixnet_port: u16,
    last_active_at: OffsetDateTime,
    noise_key: x25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract.
    /// Set to `true` when restored from the chain at startup, or after a successful
    /// `/announce` contract transaction.
    announced: bool,
}

/// Shared application state available to all axum request handlers.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub(crate) agents: KnownAgents,

    pub(crate) validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
}

impl AppState {
    pub(crate) fn new(
        agents: KnownAgents,
        validator_client: Arc<RwLock<DirectSigningHttpRpcValidatorClient>>,
    ) -> Self {
        AppState {
            agents,
            validator_client,
        }
    }
}
