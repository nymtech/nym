// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::gateway::GatewayMixnetLivenessProbe;
use crate::agent::tested_node::TestedGatewayDetails;
use crate::agent::wave::{ProbeReport, WaveListener, report_outcome};
use crate::mixnet::inbox::TargetInbox;
use crate::mixnet::targets::WaveTarget;
use anyhow::bail;
use futures::future::join_all;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_task::ShutdownToken;
use std::future::Future;
use std::sync::Arc;
use tracing::info;

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

    noise_key: Arc<x25519::KeyPair>,

    /// The probes, each already holding its own handle on the announced client identity.
    ///
    /// One identity across the whole wave is safe only because a wave's targets are DISTINCT
    /// gateways: a gateway keys its active sessions by client address, so two concurrent sessions to
    /// the SAME gateway under one identity would collide and the second would be dropped. That is
    /// checked in [`new`](Self::new) rather than assumed.
    probes: Vec<WaveProbe>,

    /// This wave's shutdown token, a child of the agent's. Every session of the wave derives its own
    /// child from it, so one target closing stops only its own reader while a cancel from above
    /// reaches every one of them.
    shutdown: ShutdownToken,
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
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        // an empty wave is a bug on the orchestrator's side rather than something to run vacuously:
        // "no work" is an absent assignment
        if targets.is_empty() {
            bail!("was handed an empty gateway liveness wave, which is not a valid assignment")
        }

        // one identity across the wave is only safe because its targets are distinct gateways, and
        // the orchestrator is what guarantees that. checked here rather than trusted: a repeated
        // gateway would have its second session dropped on `insert_remote` and score as dead, which
        // is a confusing way to learn of an assignment bug
        let mut identities = targets
            .iter()
            .map(|target| target.identity)
            .collect::<Vec<_>>();
        identities.sort_unstable_by_key(|identity| identity.to_bytes());
        identities.dedup();
        if identities.len() != targets.len() {
            bail!(
                "was handed a gateway liveness wave listing the same gateway more than once, which cannot be probed under one client identity"
            )
        }

        let probes = targets
            .into_iter()
            .map(|target| {
                let probe = GatewayMixnetLivenessProbe::new(
                    config,
                    client_identity.clone(),
                    noise_key.clone(),
                    target,
                )?;
                let ingest_inbox = probe.build_ingest_inbox();
                Ok(WaveProbe {
                    probe,
                    ingest_inbox,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(GatewayLivenessWave {
            config,
            noise_key,
            probes,
            shutdown,
        })
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
        let GatewayLivenessWave {
            config,
            noise_key,
            probes,
            shutdown,
            ..
        } = self;
        info!(
            "beginning a gateway liveness wave of {} target(s)",
            probes.len()
        );

        // 1 and 2. the ingress and the one listener serving it. this wave needs both for exactly the
        // reason the mixnode wave does: the INGEST phase's packets are forwarded by the gateway to
        // this agent's mixnet listener, and are attributed to their target by the gateway's own source
        // address, so nothing about that routing is gateway-specific
        let wave_targets = probes
            .iter()
            .map(|probed| WaveTarget {
                node: probed.probe.tested_node().clone(),
                events: probed.ingest_inbox.events_sender(),
            })
            .collect::<Vec<_>>();
        let listener =
            WaveListener::start(&config, noise_key, &wave_targets, shutdown.clone()).await?;

        // 3. every target at once, each opening its own client session and reporting as it finishes
        let deadline = config.per_target_timeout(TestKind::Liveness);
        join_all(probes.into_iter().map(|probed| {
            let report = &report;
            // a child per target, so one session closing at the end of its run stops only its own
            // reader while a cancel from above still reaches every session in the wave
            let session_shutdown = shutdown.child_token();
            async move {
                let WaveProbe {
                    probe,
                    ingest_inbox,
                } = probed;
                let node_id = probe.tested_node().node_id;
                let tested_address = probe.tested_node().address;

                let outcome = probe.run(ingest_inbox, deadline, session_shutdown).await;
                report_outcome(node_id, tested_address, outcome, report).await;
            }
        }))
        .await;

        // 4. drain the ingress, which finishes the connection handlers
        listener.drain().await;

        info!("finished the gateway liveness wave");
        Ok(())
    }
}
