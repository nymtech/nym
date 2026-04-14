// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::helpers::load_noise_key;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use nym_network_monitor_orchestrator_requests::models::AgentAnnounceRequest;
use nym_noise::LATEST_NOISE_VERSION;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

pub(crate) mod config;
pub(crate) mod helpers;
pub(crate) mod result;
pub(crate) mod tested_node;
pub(crate) mod tester;

/// A network monitor agent that receives test assignments from the orchestrator,
/// stress-tests individual nym-nodes, and reports results back.
pub(crate) struct NetworkMonitorAgent {
    /// Tester configuration controlling rates, timeouts, and addressing.
    tester_config: NodeTesterConfig,

    orchestrator_client: OrchestratorClient,

    /// The tester's own Noise key pair, used to authenticate the egress connection.
    noise_key: Arc<x25519::KeyPair>,
}

impl NetworkMonitorAgent {
    /// Creates a new agent, loading the Noise key from disk and wrapping the
    /// orchestrator token in a zeroizing container.
    pub(crate) fn new<P: AsRef<Path>>(
        tester_config: NodeTesterConfig,
        noise_key_path: P,
        orchestrator_client: OrchestratorClient,
    ) -> anyhow::Result<Self> {
        Ok(NetworkMonitorAgent {
            tester_config,
            orchestrator_client,
            noise_key: load_noise_key(noise_key_path)?,
        })
    }

    // TODO: orchestrator will have to check if this combination of key/address already exists
    pub(crate) async fn announce_agent(&self) -> anyhow::Result<()> {
        self.orchestrator_client
            .announce_agent(&AgentAnnounceRequest {
                agent_mix_socket_address: self.tester_config.mixnet_address,
                x25519_noise_key: *self.noise_key.public_key(),
                // we're always using the latest noise version available
                noise_version: LATEST_NOISE_VERSION.into(),
            })
            .await?;
        Ok(())
    }

    pub(crate) async fn run_stress_test(&self) -> anyhow::Result<()> {
        // 1. query the orchestrator for a work assignment
        let Some(work_assignment) = self
            .orchestrator_client
            .request_work_assignment()
            .await?
            .assignment
        else {
            // 2. if no work is available - exit immediately
            info!("no work available, exiting...");
            return Ok(());
        };

        info!("retrieved the following work assignment: {work_assignment:?}");

        // TODO:
        // 3. otherwise construct the tester and attempt to perform the measurements
        let _ = &self.tester_config;

        // 4. after that has concluded - submit the results back to the orchestrator
        Ok(())
    }
}
