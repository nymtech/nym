// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::tested_node::TestedNodeDetails;
use crate::agent::tester::NodeStressTester;
use anyhow::Context;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, TestRunAssignmentRequest, TestRunResultSubmissionRequest,
};
use nym_noise::LATEST_NOISE_VERSION;
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

    /// Client used to communicate with the orchestrator API (port requests, announcements,
    /// work assignments, result submissions).
    orchestrator_client: OrchestratorClient,

    /// The tester's own Noise key pair, used to authenticate the egress connection.
    noise_key: Arc<x25519::KeyPair>,
}

impl NetworkMonitorAgent {
    /// Creates a new agent with the given tester configuration, pre-loaded noise key,
    /// and orchestrator client.
    pub(crate) fn new(
        tester_config: NodeTesterConfig,
        noise_key: Arc<x25519::KeyPair>,
        orchestrator_client: OrchestratorClient,
    ) -> Self {
        NetworkMonitorAgent {
            tester_config,
            orchestrator_client,
            noise_key,
        }
    }

    /// Announces this agent's details (mixnet address, noise key, protocol version)
    /// to the orchestrator so they can be registered in the smart contract.
    pub(crate) async fn announce_agent(&self) -> anyhow::Result<()> {
        self.orchestrator_client
            .announce_agent(&AgentAnnounceRequest {
                agent_mix_socket_address: self.tester_config.external_mixnet_address,
                x25519_noise_key: *self.noise_key.public_key(),
                // we're always using the latest noise version available
                noise_version: LATEST_NOISE_VERSION.into(),
            })
            .await?;
        Ok(())
    }

    /// Requests a work assignment from the orchestrator and, if one is available,
    /// performs a stress test against the assigned node and submits the results.
    pub(crate) async fn run_stress_test(&self) -> anyhow::Result<()> {
        let request = TestRunAssignmentRequest {
            agent_mix_socket_address: self.tester_config.external_mixnet_address,
            x25519_noise_key: *self.noise_key.public_key(),
        };

        // 1. query the orchestrator for a work assignment
        let Some(work_assignment) = self
            .orchestrator_client
            .request_work_assignment(&request)
            .await?
            .assignment
        else {
            // 2. if no work is available - exit immediately
            info!("no work available, exiting...");
            return Ok(());
        };

        info!("retrieved the following work assignment: {work_assignment:?}");
        let node_id = work_assignment.node_id;

        // 3. otherwise construct the tester and attempt to perform the measurements
        let tested_node = TestedNodeDetails::from_testrun_assignment(work_assignment);
        let mut stress_tester =
            NodeStressTester::new(self.tester_config, self.noise_key.clone(), tested_node)?;

        // attempt to perform the measurements within the configured timeouts
        // note: the only errors we're possibly exiting on are critical failures like
        // theoretically impossible sphinx packet creations or failing to join on tasks.
        // any sending/receiving errors are included as part of an `Ok(result)` response.
        let result = stress_tester.run_stress_test().await?;

        // 4. after that has concluded - submit the results back to the orchestrator
        self.orchestrator_client
            .submit_test_run_result(&TestRunResultSubmissionRequest {
                node_id,
                result: result.into(),
            })
            .await
            .context("failed to submit test run result")?;
        Ok(())
    }
}
