// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::FromRef;
use nym_crypto::asymmetric::x25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

#[derive(Clone)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    pub(crate) async fn assign_agent_port(
        &self,
        host_ip: IpAddr,
        agent_pubkey: x25519::PublicKey,
    ) -> u16 {
        let mut guard = self.inner.lock().await;
        let host_agents = guard.agents.entry(host_ip).or_insert_with(Vec::new);

        // if this agent existed before, return the existing information
        if let Some(existing_agent) = host_agents.iter().find(|a| a.noise_key == agent_pubkey) {
            info!("reusing existing agent port for agent at {host_ip} with key {agent_pubkey}");
            return existing_agent.mixnet_port;
        }

        // assign a new port to the agent

        // 1. figure out used ports
        let mut occupied_ports = host_agents
            .iter()
            .map(|a| a.mixnet_port)
            .collect::<Vec<_>>();
        occupied_ports.sort();

        // 2. choose the next available port (or fallback to default for the first agent with given host ip)
        let next_port = occupied_ports
            .last()
            .map(|p| *p + 1)
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT);

        // insert agent information into the cache
        host_agents.push(KnownAgent {
            host_ip,
            mixnet_port: next_port,
            last_active_at: OffsetDateTime::now_utc(),
            noise_key: agent_pubkey,
        });

        next_port
    }

    // due to port assignment, socket address is guaranteed to uniquely identify an agent
    // (noise key would have also worked)
    pub(crate) async fn touch_agent(&self, mix_listener: SocketAddr) -> bool {
        let mut guard = self.inner.lock().await;
        let Some(host_agents) = guard.agents.get_mut(&mix_listener.ip()) else {
            return false;
        };

        let agent = host_agents
            .iter_mut()
            .find(|agent| agent.mixnet_port == mix_listener.port());
        if let Some(agent) = agent {
            agent.last_active_at = OffsetDateTime::now_utc();
            return true;
        }

        false
    }
}

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
                });
        }

        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(KnownAgentsInner { agents: agents_map })),
        })
    }
}

struct KnownAgentsInner {
    // map of agents, based on the host ip address, to their known state
    agents: HashMap<IpAddr, Vec<KnownAgent>>,
}

#[allow(dead_code)]
struct KnownAgent {
    host_ip: IpAddr,
    mixnet_port: u16,
    last_active_at: OffsetDateTime,
    noise_key: x25519::PublicKey,
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
