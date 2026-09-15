// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::result::TestRunResult;
use crate::agent::tested_node::TestedNodeDetails;
use crate::agent::tester::NodeProbe;
use crate::mixnet::connection_handler::SharedHandlerData;
use crate::mixnet::inbox::TargetInbox;
use crate::mixnet::listener::MixnetListener;
use crate::mixnet::targets::{WaveIngress, WaveTarget};
use anyhow::bail;
use futures::future::join_all;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_task::ShutdownToken;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{debug, error, info};

/// One target's outcome, handed over the moment that target finishes.
pub(crate) struct ProbeReport {
    pub(crate) node_id: Option<u32>,

    /// The address that was actually probed. A node may announce several and only some may be
    /// healthy, so a result is meaningless without it.
    pub(crate) tested_address: SocketAddr,

    pub(crate) result: TestRunResult,
}

/// One assignment's targets, probed CONCURRENTLY against a single shared ingress.
///
/// One wave is one concurrent batch, never split into sub-waves: that is what bounds the assignment's
/// lease by the worst case of one target rather than by the sum over all of them, and it is the only
/// reason a multi-target lease is safe.
///
/// A stress assignment goes through here too, as a wave of exactly one. Keeping one path means the
/// stress test exercises the same machinery a liveness wave does rather than the two drifting apart.
pub(crate) struct MixnetWave {
    config: NodeTesterConfig,
    kind: TestKind,
    noise_key: Arc<x25519::KeyPair>,
    probes: Vec<WaveProbe>,
}

/// A probe and the inbox the wave's ingress will route to it.
struct WaveProbe {
    probe: NodeProbe,
    inbox: TargetInbox,
}

impl MixnetWave {
    pub(crate) fn new(
        config: NodeTesterConfig,
        kind: TestKind,
        noise_key: Arc<x25519::KeyPair>,
        targets: Vec<TestedNodeDetails>,
    ) -> anyhow::Result<Self> {
        // "no work" is an absent assignment, so an empty wave is a bug on the orchestrator's side
        // rather than something to run vacuously
        if targets.is_empty() {
            bail!("was handed an empty {kind} wave, which is not a valid assignment")
        }

        let probes = targets
            .into_iter()
            .map(|node| {
                let probe = NodeProbe::new(config, kind, noise_key.clone(), node)?;
                let inbox = probe.build_inbox();
                Ok(WaveProbe { probe, inbox })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(MixnetWave {
            config,
            kind,
            noise_key,
            probes,
        })
    }

    /// Probes every target at once, handing each report to `report` the moment that target finishes.
    ///
    /// Reporting per target rather than at the end of the wave is what releases each target's
    /// in-flight lock independently, and what limits an agent that dies mid-wave to losing only the
    /// targets it had not yet reported.
    ///
    /// One target's outcome never aborts the wave: a probe that fails critically is logged and its
    /// target left unreported, so the orchestrator's lease expires and the node keeps its turn rather
    /// than being scored zero for our own fault.
    pub(crate) async fn run<F, Fut>(self, report: F) -> anyhow::Result<()>
    where
        F: Fn(ProbeReport) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let MixnetWave {
            config,
            kind,
            noise_key,
            probes,
        } = self;
        info!("beginning a {kind} wave of {} target(s)", probes.len());

        // 1. one ingress table over every target of the wave, which has to exist before any probe
        // starts: the listener can only route to channels that are already registered
        let wave_targets = probes
            .iter()
            .map(|probed| WaveTarget {
                node: probed.probe.tested_node().clone(),
                events: probed.inbox.events_sender(),
            })
            .collect::<Vec<_>>();
        let ingress = Arc::new(WaveIngress::new(&wave_targets));

        // 2. ONE listener for the whole wave
        debug!("creating mixnet listener on {}", config.mixnet_bind_address);
        let shutdown_token = ShutdownToken::new();
        let listener = MixnetListener::new(
            config.mixnet_bind_address,
            SharedHandlerData::new(
                ingress,
                noise_key,
                config.noise_handshake_timeout,
                shutdown_token.clone(),
            ),
            shutdown_token.clone(),
        )
        .await?;

        let listener_on_start = Arc::new(Notify::new());
        let listener_on_start_clone = listener_on_start.clone();
        let listener_join =
            tokio::spawn(async move { listener.run(listener_on_start_clone).await });

        // wait for the listener task to properly begin
        listener_on_start.notified().await;

        // 3. every target at once, each reporting as it finishes. the wave's duration is therefore
        // the slowest single target rather than the sum over them
        let deadline = config.per_target_timeout(kind);
        join_all(
            probes
                .into_iter()
                .map(|probed| probe_and_report(probed, deadline, &report)),
        )
        .await;

        // 4. shut the ingress down, which drains the connection handlers
        debug!("shutting down the mixnet listener and finishing the wave");
        shutdown_token.cancel();
        let _ = listener_join.await;

        info!("finished the {kind} wave");
        Ok(())
    }
}

/// Runs one target of a wave and reports it, swallowing its failures so the rest of the wave is
/// unaffected.
async fn probe_and_report<F, Fut>(
    probed: WaveProbe,
    deadline: Option<std::time::Duration>,
    report: &F,
) where
    F: Fn(ProbeReport) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let WaveProbe { probe, inbox } = probed;
    let node_id = probe.tested_node().node_id;
    let tested_address = probe.tested_node().address;

    let result = match probe.run(inbox, deadline).await {
        Ok(result) => result,
        Err(err) => {
            // a critical failure on OUR side, so nothing is reported for this target: it keeps its
            // turn through the lease expiring rather than being scored zero for our fault
            error!(
                "the probe of {tested_address} failed critically, so it will not be reported: {err:#}"
            );
            return;
        }
    };

    if let Err(err) = report(ProbeReport {
        node_id,
        tested_address,
        result,
    })
    .await
    {
        error!("failed to report the result for {tested_address}: {err:#}");
    }
}
