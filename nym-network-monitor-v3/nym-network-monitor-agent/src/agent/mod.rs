// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::gateway::wave::GatewayLivenessWave;
use crate::agent::helpers::derive_client_identity;
use crate::agent::tested_node::{TestedGatewayDetails, TestedNodeDetails};
use crate::agent::wave::{MixnetWave, ProbeReport};
use anyhow::{Context, bail};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use nym_network_monitor_orchestrator_requests::models::{
    AgentAnnounceRequest, AgentMixAddresses, GatewayProbeTarget, MixnetProbeTarget, TestKind,
    TestRunAssignment, TestRunAssignmentRequest, TestRunResultSubmissionRequest,
};
use nym_noise::LATEST_NOISE_VERSION;
use nym_sphinx_types::DestinationAddressBytes;
use std::sync::Arc;
use tracing::info;

pub(crate) mod config;
pub(crate) mod gateway;
pub(crate) mod helpers;
pub(crate) mod result;
pub(crate) mod tested_node;
pub(crate) mod tester;
pub(crate) mod wave;

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

    /// The ed25519 identity this agent presents when opening a gateway client session, derived from
    /// [`Self::noise_key`]. It is announced on chain, so it must stay stable for as long as the
    /// noise key does.
    ///
    /// Shared rather than owned because a gateway wave registers one session per target, all of them
    /// concurrently and all under this one identity.
    client_identity: Arc<ed25519::KeyPair>,
}

impl NetworkMonitorAgent {
    /// Creates a new agent with the given tester configuration, pre-loaded noise key,
    /// and orchestrator client.
    pub(crate) fn new(
        tester_config: NodeTesterConfig,
        noise_key: Arc<x25519::KeyPair>,
        orchestrator_client: OrchestratorClient,
    ) -> anyhow::Result<Self> {
        let client_identity = Arc::new(derive_client_identity(&noise_key)?);

        Ok(NetworkMonitorAgent {
            tester_config,
            orchestrator_client,
            noise_key,
            client_identity,
        })
    }

    /// This agent's client address, which every test packet carries as its sphinx destination and
    /// which a gateway resolves a delivered final-hop packet by.
    ///
    /// Derived from the ANNOUNCED identity, so it is the address a gateway that granted this agent a
    /// session holds a live entry under.
    fn client_address(&self) -> DestinationAddressBytes {
        self.client_identity
            .public_key()
            .derive_destination_address()
    }

    /// The addresses this agent announces to the orchestrator, and thus the ones the nodes it
    /// tests will see its traffic coming from.
    fn mix_addresses(&self) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: self.tester_config.external_mixnet_address_v4,
            v6: self.tester_config.external_mixnet_address_v6,
        }
    }

    /// Announces this agent's details (mixnet address, noise key, protocol version, client identity)
    /// to the orchestrator so they can be registered in the smart contract.
    pub(crate) async fn announce_agent(&self) -> anyhow::Result<()> {
        self.orchestrator_client
            .announce_agent(&AgentAnnounceRequest {
                mix_addresses: self.mix_addresses(),
                x25519_noise_key: *self.noise_key.public_key(),
                // we're always using the latest noise version available
                noise_version: LATEST_NOISE_VERSION.into(),
                ed25519_identity: *self.client_identity.public_key(),
            })
            .await?;
        Ok(())
    }

    /// Probes every target of a mixnet wave, submitting each result the moment that target finishes.
    ///
    /// Both mixnode kinds come through here: a stress assignment is a wave of exactly one, so the two
    /// share a path rather than drifting apart. The kind selects the profile and the per-target
    /// deadline, and is echoed onto every result.
    async fn run_mixnet_wave(
        &self,
        kind: TestKind,
        targets: Vec<MixnetProbeTarget>,
    ) -> anyhow::Result<()> {
        let targets = targets
            .into_iter()
            .map(TestedNodeDetails::from_probe_target)
            .collect();

        MixnetWave::new(
            self.tester_config,
            kind,
            self.client_address(),
            self.noise_key.clone(),
            targets,
        )?
        .run(|report| self.submit(report))
        .await
    }

    /// Probes every target of a gateway liveness wave, submitting each result as that target finishes.
    ///
    /// A separate path from [`run_mixnet_wave`](Self::run_mixnet_wave) rather than a third kind
    /// flowing through it: the two share a listener and a reporting shape, but a gateway target is a
    /// client session measuring two interfaces where a mixnode target is a Noise connection measuring
    /// one, so folding them together would mean a probe that is two probes behind one name.
    async fn run_gateway_liveness_wave(
        &self,
        targets: Vec<GatewayProbeTarget>,
    ) -> anyhow::Result<()> {
        let targets = targets
            .into_iter()
            .map(TestedGatewayDetails::from_probe_target)
            .collect();

        GatewayLivenessWave::new(
            self.tester_config,
            self.client_identity.clone(),
            self.noise_key.clone(),
            targets,
        )?
        .run(|report| self.submit(report))
        .await
    }

    /// Submits one target's result.
    ///
    /// A node-level failure is submitted even when zeroed: submission is what advances that pairing's
    /// staleness, so withholding it would leave the node maximally overdue and reassigned the moment
    /// its lease expired.
    async fn submit(&self, report: ProbeReport) -> anyhow::Result<()> {
        let ProbeReport {
            node_id,
            tested_address,
            result,
        } = report;

        // every target of an assignment carries one; only the manual `test-node` command probes a
        // node it has no id for, and that path never reaches here
        let Some(node_id) = node_id else {
            bail!("cannot submit a result for {tested_address}: the target carries no node id")
        };

        self.orchestrator_client
            .submit_test_run_result(&TestRunResultSubmissionRequest {
                node_id,
                tested_address,
                result: result.into(),
            })
            .await
            .context("failed to submit test run result")?;
        Ok(())
    }

    /// Requests a work assignment from the orchestrator and, if one is available,
    /// performs appropriate test against the assigned node and submits the results.
    pub(crate) async fn perform_work_assignment(&self) -> anyhow::Result<()> {
        let request = TestRunAssignmentRequest {
            mix_addresses: self.mix_addresses(),
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

        // 3. run the assignment. each kind submits its own results as its targets finish, rather than
        // the results being collected and submitted here: a wave's targets have to release their
        // in-flight locks independently of one another
        match work_assignment {
            TestRunAssignment::MixnodeStress(target) => {
                self.run_mixnet_wave(TestKind::Stress, vec![*target]).await
            }
            TestRunAssignment::MixnodeLiveness(targets) => {
                self.run_mixnet_wave(TestKind::Liveness, targets).await
            }
            TestRunAssignment::GatewayLiveness(targets) => {
                self.run_gateway_liveness_wave(targets).await
            }
        }
    }
}
