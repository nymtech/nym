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
use nym_sphinx_types::DestinationAddressBytes;
use nym_task::ShutdownToken;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// One target's outcome, handed over the moment that target finishes.
pub(crate) struct ProbeReport {
    pub(crate) node_id: Option<u32>,

    /// The address that was actually probed. A node may announce several and only some may be
    /// healthy, so a result is meaningless without it.
    pub(crate) tested_address: SocketAddr,

    pub(crate) result: TestRunResult,
}

/// The one mixnet listener that serves a whole wave, and the table routing what it accepts to the
/// target it belongs to.
///
/// Shared by both waves rather than written out twice, because the ORDERING is the load-bearing part:
/// the ingress table has to exist before any probe starts, since the listener can only route to
/// channels that are already registered; the listener has to be accepting before the first packet can
/// come back; and it has to be drained afterwards so its handlers finish. A copy of that sequence
/// with one step out of place would present as unattributed packets rather than as a failure.
pub(crate) struct WaveListener {
    /// This wave's token, cancelled by [`drain`](Self::drain). A child of the agent's, so cancelling
    /// it ends the listener without ending the agent.
    shutdown: ShutdownToken,

    join: JoinHandle<()>,
}

impl WaveListener {
    /// Builds the ingress over every target and starts the listener, returning once it is accepting.
    pub(crate) async fn start(
        config: &NodeTesterConfig,
        noise_key: Arc<x25519::KeyPair>,
        targets: &[WaveTarget],
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        let ingress = Arc::new(WaveIngress::new(targets));

        debug!("creating mixnet listener on {}", config.mixnet_bind_address);
        let listener = MixnetListener::new(
            config.mixnet_bind_address,
            SharedHandlerData::new(
                ingress,
                noise_key,
                config.noise_handshake_timeout,
                shutdown.clone(),
            ),
            shutdown.clone(),
        )
        .await?;

        let on_start = Arc::new(Notify::new());
        let listener_on_start = on_start.clone();
        let join = tokio::spawn(async move { listener.run(listener_on_start).await });

        // wait for the listener task to properly begin, so no probe sends into a socket that is not
        // yet accepting
        on_start.notified().await;

        Ok(WaveListener { shutdown, join })
    }

    /// Shuts the listener down, which drains its connection handlers.
    pub(crate) async fn drain(self) {
        debug!("shutting down the mixnet listener and finishing the wave");
        self.shutdown.cancel();
        let _ = self.join.await;
    }
}

/// Hands one target's outcome to `report`, swallowing failures so the rest of the wave is unaffected.
///
/// Shared by both waves because the SEMANTICS are subtle in the same way for each: a probe that
/// failed critically is left unreported rather than submitted as a zero, so the orchestrator's lease
/// expires and the node keeps its turn instead of being scored down for our own fault. A failure to
/// report is likewise logged rather than propagated, since the wave's other targets are unaffected by
/// it.
pub(crate) async fn report_outcome<F, Fut>(
    node_id: Option<u32>,
    tested_address: SocketAddr,
    outcome: anyhow::Result<TestRunResult>,
    report: &F,
) where
    F: Fn(ProbeReport) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let result = match outcome {
        Ok(result) => result,
        Err(err) => {
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

    /// This wave's shutdown token, a child of the agent's. Cancelled at the end of the wave to drain
    /// the listener, which is why it must be a child: cancelling it must not take the agent with it.
    shutdown: ShutdownToken,
}

/// A probe and the inbox the wave's ingress will route to it.
struct WaveProbe {
    probe: NodeProbe,
    inbox: TargetInbox,
}

impl MixnetWave {
    /// `client_address` is this agent's own, derived once by the caller rather than per probe: it is a
    /// property of the agent's announced identity rather than of any target, so deriving it inside
    /// each probe would run the same HKDF once per target of the wave.
    pub(crate) fn new(
        config: NodeTesterConfig,
        kind: TestKind,
        client_address: DestinationAddressBytes,
        noise_key: Arc<x25519::KeyPair>,
        targets: Vec<TestedNodeDetails>,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        // "no work" is an absent assignment, so an empty wave is a bug on the orchestrator's side
        // rather than something to run vacuously
        if targets.is_empty() {
            bail!("was handed an empty {kind} wave, which is not a valid assignment")
        }

        let probes = targets
            .into_iter()
            .map(|node| {
                let probe = NodeProbe::new(config, kind, client_address, noise_key.clone(), node)?;
                let inbox = probe.build_inbox();
                Ok(WaveProbe { probe, inbox })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(MixnetWave {
            config,
            kind,
            noise_key,
            probes,
            shutdown,
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
            shutdown,
        } = self;
        info!("beginning a {kind} wave of {} target(s)", probes.len());

        // 1 and 2. the ingress over every target, and the one listener serving them all
        let wave_targets = probes
            .iter()
            .map(|probed| WaveTarget {
                node: probed.probe.tested_node().clone(),
                events: probed.inbox.events_sender(),
            })
            .collect::<Vec<_>>();
        let listener = WaveListener::start(&config, noise_key, &wave_targets, shutdown).await?;

        // 3. every target at once, each reporting as it finishes. the wave's duration is therefore
        // the slowest single target rather than the sum over them
        let deadline = config.per_target_timeout(kind);
        join_all(probes.into_iter().map(|probed| {
            let report = &report;
            async move {
                let WaveProbe { probe, inbox } = probed;
                let node_id = probe.tested_node().node_id;
                let tested_address = probe.tested_node().address;

                let outcome = probe.run(inbox, deadline).await;
                report_outcome(node_id, tested_address, outcome, report).await;
            }
        }))
        .await;

        // 4. drain the ingress, which finishes the connection handlers
        listener.drain().await;

        info!("finished the {kind} wave");
        Ok(())
    }
}
