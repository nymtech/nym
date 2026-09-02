// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// SCAFFOLD: the bodies land as group 9's tasks are worked through, at which point both allows come off
#![allow(dead_code, unused_variables)]

use crate::agent::config::NodeTesterConfig;
use crate::agent::gateway::GatewayMixnetLivenessProbe;
use crate::agent::tested_node::TestedGatewayDetails;
use crate::agent::wave::ProbeReport;
use crate::mixnet::inbox::TargetInbox;
use nym_crypto::asymmetric::{ed25519, x25519};
use std::future::Future;
use std::sync::Arc;

/// One gateway liveness assignment's targets, probed CONCURRENTLY.
///
/// The same bargain as [`MixnetWave`](crate::agent::wave::MixnetWave): one wave is one concurrent
/// batch, so the lease the orchestrator stamps is bounded by the slowest single target rather than by
/// their sum. A gateway target costs more to hold open than a mixnode one, which is why the
/// orchestrator sizes gateway waves smaller rather than why this batches them differently.
///
/// It shares the mixnet listener machinery with the mixnode wave, because the ingest phase's packets
/// arrive over exactly that listener, attributed by the forwarding gateway's source address.
pub(crate) struct GatewayLivenessWave {
    config: NodeTesterConfig,

    /// The announced client identity every session in the wave registers with.
    ///
    /// One identity across the whole wave is safe only because a wave's targets are DISTINCT
    /// gateways: a gateway keys its active sessions by client address, so two concurrent sessions to
    /// the SAME gateway under one identity would collide and the second would be dropped. A wave that
    /// listed a gateway twice would therefore score one of the two as dead.
    client_identity: Arc<ed25519::KeyPair>,

    noise_key: Arc<x25519::KeyPair>,

    probes: Vec<WaveProbe>,
}

/// A probe and the inbox the wave's ingress will route its ingest-phase arrivals to.
struct WaveProbe {
    probe: GatewayMixnetLivenessProbe,
    ingest_inbox: TargetInbox,
}

impl GatewayLivenessWave {
    pub(crate) fn new(
        config: NodeTesterConfig,
        client_identity: Arc<ed25519::KeyPair>,
        noise_key: Arc<x25519::KeyPair>,
        targets: Vec<TestedGatewayDetails>,
    ) -> anyhow::Result<Self> {
        // an empty wave is a bug on the orchestrator's side rather than something to run vacuously:
        // "no work" is an absent assignment
        todo!()
    }

    /// Probes every target at once, handing each report to `report` the moment that target finishes.
    ///
    /// One target's outcome never aborts the wave, and a probe that fails critically is left
    /// unreported so the orchestrator's lease expires and the node keeps its turn rather than being
    /// scored zero for our own fault.
    pub(crate) async fn run<F, Fut>(self, report: F) -> anyhow::Result<()>
    where
        F: Fn(ProbeReport) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        // 1. one ingress table over every target, built before any probe starts
        // 2. ONE mixnet listener for the whole wave, receiving every target's ingest phase
        // 3. every target at once, each establishing its own session and reporting as it finishes
        // 4. shut the ingress down, which drains the connection handlers
        //
        // TODO: steps 1, 2 and 4 are `MixnetWave::run` verbatim. Once both waves exist and the
        // duplication is visible in one place, decide whether the shared part becomes a helper the two
        // call or whether the two waves collapse into one type parameterised by its probe
        todo!()
    }
}
