// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::{NodeTesterConfig, ProbeProfile};
use crate::agent::result::{LatencyDistribution, TestRunResult};
use crate::agent::tested_node::TestedNodeDetails;
use crate::mixnet::egress::EgressConnection;
use crate::mixnet::inbox::TargetInbox;
use crate::mixnet::sphinx::helpers::{
    as_sphinx_node, build_test_sphinx_packet, create_test_sphinx_packet_header,
};
use crate::mixnet::sphinx::payload::ProcessedPacket;
use crate::mixnet::sphinx::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::Context;
use humantime::format_duration;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_sphinx_types::SphinxPacket;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::time::{Instant, sleep, timeout};
use tracing::{debug, error, info, warn};

/// A probe of ONE node: everything it needs before it starts, and the sequence it then runs.
///
/// A probe proceeds in five ordered steps (see [`run`](Self::run)):
///
/// 1. Establish an outbound (egress) Noise-encrypted TCP connection to the node.
/// 2. Bind a local TCP listener (ingress) that receives sphinx packets the node sends back.
/// 3. Send a single probe packet to verify basic connectivity and record baseline latency.
/// 4. Replay the same packet (when `reuse_header` is enabled) to confirm the node's
///    bloomfilter bypass is correctly configured.
/// 5. Send packets at the profile's rate for its duration, then collect and summarise the results.
///
/// The same sequence serves both test kinds; only the [`ProbeProfile`] applied differs. Only critical
/// failures (e.g. failing to bind a port) are returned as `Err`; node-level failures (e.g. the node
/// not responding) are captured inside the returned [`TestRunResult`] so the caller can still inspect
/// partial data, and can still submit it.
pub(crate) struct NodeProbe {
    /// Tester configuration controlling timeouts and addressing.
    config: NodeTesterConfig,

    /// What this probe measures. Selects the profile and is echoed onto the result.
    kind: TestKind,

    /// The sending knobs of that kind, resolved once at construction so the two cannot drift.
    profile: ProbeProfile,

    /// Pre-built sphinx packet header reused across all packets when `config.reuse_header`
    /// is set. Allows the node's bloomfilter bypass to be exercised. `None` means a fresh
    /// header is built for every packet.
    reusable_test_header: Option<TestPacketHeader>,

    /// The agent's own Noise key pair, used to authenticate the egress connection.
    noise_key: Arc<x25519::KeyPair>,

    /// An ephemeral sphinx key pair generated at construction time. Used both to build the
    /// return-route sphinx header (so packets come back to this agent) and to decrypt
    /// returning packets when `reuse_header` is disabled.
    sphinx_key: Arc<x25519::KeyPair>,

    /// Identity and addressing information for the node being probed.
    tested_node: TestedNodeDetails,
}

/// How far a probe's sequence got, which is what decides whether there are connection statistics to
/// fold into its result.
enum ProbeOutcome {
    /// Nothing ever came back, so there is neither an ingress handshake nor egress statistics: the
    /// node either refused the connection or never returned a packet.
    NothingReturned,

    /// At least one packet came back, so the node connected to us, completed the handshake as our
    /// responder, and both sets of statistics exist.
    Measured,

    /// Cut off by the per-target deadline. Whatever was gathered stands, but nothing about the
    /// connection can be assumed: the probe may have been interrupted before or after its first
    /// packet came back, so the handshake is folded in only if it actually arrived.
    DeadlineExceeded,
}

impl NodeProbe {
    /// Builds a probe of `tested_node` under `profile`, generating a fresh ephemeral sphinx key. If
    /// `config.reuse_header` is set, the sphinx packet header is pre-built here so it can be reused
    /// across all test packets.
    pub(crate) fn new(
        config: NodeTesterConfig,
        kind: TestKind,
        noise_key: Arc<x25519::KeyPair>,
        tested_node: TestedNodeDetails,
    ) -> anyhow::Result<Self> {
        let profile = config.profile_for(kind);

        debug!("using the following tester config");
        debug!("{config:#?}");

        debug!("probing the following node for {kind} under {profile:?}");
        debug!("{tested_node:#?}");

        let sphinx_key = x25519::PrivateKey::new(&mut OsRng);

        let reusable_test_header = if config.reuse_header {
            debug!("reusing sphinx header for tests");
            // Route: tested node → this agent (so packets come back to us).
            let route = [
                tested_node.as_sphinx_node(),
                as_sphinx_node(
                    config.return_address_for(tested_node.address),
                    sphinx_key.public_key(),
                ),
            ];
            let delay = config.packet_delay;
            Some(create_test_sphinx_packet_header(route, delay)?)
        } else {
            debug!("new sphinx header will be generated for each new test packet");
            None
        };

        Ok(Self {
            config,
            kind,
            profile,
            reusable_test_header,
            noise_key,
            sphinx_key: Arc::new(sphinx_key.into()),
            tested_node,
        })
    }

    /// The node this probe measures.
    pub(crate) fn tested_node(&self) -> &TestedNodeDetails {
        &self.tested_node
    }

    /// Builds this target's inbox.
    ///
    /// Handed out rather than built inside [`run`](Self::run) because a wave's ingress has to be
    /// assembled from every target's inbox BEFORE any of them starts: one listener serves the whole
    /// wave, and it can only route to channels that already exist.
    pub(crate) fn build_inbox(&self) -> TargetInbox {
        let packet_recovery = match &self.reusable_test_header {
            Some(header) => header.clone().into(),
            None => self.sphinx_key.clone().into(),
        };
        TargetInbox::new(packet_recovery, self.profile.waiting_duration)
    }

    /// Opens the outbound Noise-encrypted TCP connection to the node under test.
    async fn establish_egress_connection(&self) -> anyhow::Result<EgressConnection> {
        EgressConnection::establish(
            self.tested_node.address,
            self.config.egress_connection_timeout,
            self.tested_node.key_rotation,
            &self.egress_noise_config(),
        )
        .await
    }

    /// The Noise config for the EGRESS connection to the node under test.
    ///
    /// This is the one direction that needs the node's static key: the initiator looks the responder
    /// up by the address it dialled (`get_noise_key`), whereas the responder side only gates on
    /// whether it recognises a source and authenticates with our own key. Scoped to the address
    /// being dialled for the same reason the ingress scopes its own configs per connection.
    fn egress_noise_config(&self) -> NoiseConfig {
        let nodes = HashMap::from([(
            self.tested_node.address.ip().to_canonical(),
            self.tested_node.as_noise_node(),
        )]);

        NoiseConfig::new(
            self.noise_key.clone(),
            NoiseNetworkView::new(nodes),
            self.config.noise_handshake_timeout,
        )
    }

    /// Returns a sphinx node representation of this agent's own mixnet listener address,
    /// used as the final hop in the packet route so packets are delivered back here.
    fn as_sphinx_node(&self) -> nym_sphinx_types::Node {
        as_sphinx_node(
            self.config.return_address_for(self.tested_node.address),
            *self.sphinx_key.public_key(),
        )
    }

    /// Runs the probe against its target and returns the collected results.
    ///
    /// `inbox` is the one this probe's [`build_inbox`](Self::build_inbox) produced, already registered
    /// with the wave's shared ingress. `deadline`, when set, cuts the probe off and keeps whatever it
    /// had gathered, so one unresponsive target cannot hold up the wave it belongs to.
    ///
    /// The ingress is NOT torn down here: one listener serves the whole wave, so its lifetime belongs
    /// to whoever assembled it.
    pub(crate) async fn run(
        self,
        inbox: TargetInbox,
        deadline: Option<Duration>,
    ) -> anyhow::Result<TestRunResult> {
        let node_address = self.tested_node.address;
        let node_id = self.tested_node.node_id;
        if let Some(node_id) = node_id {
            info!("beginning probe of node {node_id} ({node_address})");
        } else {
            info!("beginning probe of node {node_address}");
        }

        // started HERE rather than once the connection is up: the result's elapsed time is defined to
        // include establishing the connections, so stamping it any later would quietly drop the
        // egress connect and its handshake out of every run's reported duration
        let mut result = TestRunResult::new(self.kind, self.config.packet_delay);

        // 1. establish the egress connection — abort immediately if it fails
        debug!("attempting to establish egress connection to the tested node");
        let egress = match self.establish_egress_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                // a node that would not take our connection is measured as zero rather than being an
                // agent-side error, so the caller still has a result to submit
                result.set_error(format!(
                    "{:#}",
                    err.context("failed to establish egress node connection")
                ));
                return Ok(result);
            }
        };

        // 2 to 6. everything from here on owns the live resources, so it needs no arguments threaded
        // through it. that is what stops a wave's targets being able to write into each other's
        // results
        let mut run = ProbeRun::new(self, egress, inbox, result);
        let outcome = run.run_to_deadline(deadline).await?;
        let result = run.finish(outcome)?;

        if let Some(node_id) = node_id {
            info!("finished probe of node {node_id} ({node_address})");
        } else {
            info!("finished probe of node {node_address}");
        }
        Ok(result)
    }
}

/// One probe in flight: the live resources, plus the state that only exists while it runs.
///
/// Separate from [`NodeProbe`] because the egress connection is established during the run and can
/// fail, so it cannot be a field of the probe without becoming an `Option` that every send path has
/// to unwrap. Owning the probe rather than borrowing it keeps this free of lifetimes.
struct ProbeRun {
    probe: NodeProbe,

    /// The outbound connection every test packet is sent over.
    egress: EgressConnection,

    /// This target's receiving side.
    inbox: TargetInbox,

    /// Accumulated as the run proceeds, and returned whether or not the run completed. Handed in
    /// rather than created here, because its start time has to predate the egress connection.
    result: TestRunResult,

    /// Monotonically increasing counter embedded in each outgoing packet as its ID. Per RUN rather
    /// than per probe, since ids are only meaningful within the run that issued them.
    packet_counter: u64,
}

impl ProbeRun {
    fn new(
        probe: NodeProbe,
        egress: EgressConnection,
        inbox: TargetInbox,
        result: TestRunResult,
    ) -> Self {
        ProbeRun {
            probe,
            egress,
            inbox,
            result,
            packet_counter: 0,
        }
    }

    /// Runs the sequence, cut off at `deadline` if one is set.
    ///
    /// The deadline wraps the sequence HERE rather than the caller wrapping the whole probe, because a
    /// dropped future would take its partial result with it: a target cut off mid-flight still has to
    /// report the packets it did get back.
    async fn run_to_deadline(
        &mut self,
        deadline: Option<Duration>,
    ) -> anyhow::Result<ProbeOutcome> {
        let Some(deadline) = deadline else {
            return self.execute().await;
        };

        let address = self.probe.tested_node.address;
        // bound to a local so the borrow the timeout holds on `self` ends before the arms run
        let outcome = timeout(deadline, self.execute()).await;

        match outcome {
            Ok(outcome) => outcome,
            Err(_) => {
                let deadline = format_duration(deadline);
                warn!("the probe of {address} did not complete within {deadline}");
                self.result
                    .set_error(format!("the probe did not complete within {deadline}"));
                Ok(ProbeOutcome::DeadlineExceeded)
            }
        }
    }

    /// Runs steps 3 to 6 of the sequence, reporting how far it got.
    ///
    /// Returns `Err` only for critical failures; a node that misbehaves leaves its error on the
    /// result instead.
    async fn execute(&mut self) -> anyhow::Result<ProbeOutcome> {
        // 3. probe: send a single packet to confirm the node responds
        debug!("sending initial node connectivity probe");
        if !self.send_connectivity_probe().await? {
            return Ok(ProbeOutcome::NothingReturned);
        }

        // 4. probe: replay the packet to verify bloomfilter bypass is configured
        debug!("sending bloomfilter probe");
        if self.probe.config.reuse_header && !self.send_bloomfilter_probe().await? {
            return Ok(ProbeOutcome::Measured);
        }

        // 5. send packets at the profile's rate for its duration
        debug!(
            "beginning the proper load testing. going to send at rate {}/s for {}",
            self.probe.profile.target_rate,
            format_duration(self.probe.profile.sending_duration)
        );
        self.send_load_test().await?;

        // 6. collect and summarise results
        debug!("waiting for final packets to arrive");
        self.collect_test_results().await;

        Ok(ProbeOutcome::Measured)
    }

    /// Folds in the statistics that only exist once a packet has come back, and yields the result.
    fn finish(self, outcome: ProbeOutcome) -> anyhow::Result<TestRunResult> {
        let ProbeRun {
            egress,
            inbox,
            mut result,
            ..
        } = self;

        match outcome {
            // nothing came back, so neither statistic exists
            ProbeOutcome::NothingReturned => (),

            ProbeOutcome::Measured => {
                // absence is a hard error rather than a missing measurement: this is only reached
                // once a packet has come back, and a packet coming back means the node connected to
                // us and completed the handshake as our responder, so a `None` is a defect in the
                // per-target plumbing rather than a node that behaved badly
                let ingress_handshake = inbox
                    .ingress_handshake()
                    .context("missing ingress noise duration after completing entire test run!")?;

                result.set_ingress_noise_handshake(ingress_handshake);
                result.set_egress_connection_statistics(egress.connection_statistics);
            }

            ProbeOutcome::DeadlineExceeded => {
                // that invariant does NOT hold for a probe that was cut off part way, so this takes
                // the handshake if it arrived and says nothing if it did not
                if let Some(ingress_handshake) = inbox.ingress_handshake() {
                    result.set_ingress_noise_handshake(ingress_handshake);
                }
                result.set_egress_connection_statistics(egress.connection_statistics);
            }
        }

        Ok(result)
    }

    /// Builds the next test sphinx packet, incrementing the internal packet counter.
    /// Reuses the pre-built header when available; otherwise builds a fresh header and
    /// encrypts it with a new sphinx key each time.
    fn create_test_sphinx_packet(&mut self) -> anyhow::Result<SphinxPacket> {
        let content = TestPacketContent::new(self.packet_counter);
        self.packet_counter += 1;

        match &self.probe.reusable_test_header {
            Some(header) => header.create_test_packet(content),
            None => {
                let route = [
                    self.probe.tested_node.as_sphinx_node(),
                    self.probe.as_sphinx_node(),
                ];
                build_test_sphinx_packet(
                    &route,
                    self.probe.config.packet_delay,
                    None,
                    &content.to_bytes(),
                )
            }
        }
    }

    /// Builds a batch of `batch_size` test sphinx packets with consecutive IDs.
    fn create_packet_batch(&mut self, batch_size: usize) -> anyhow::Result<Vec<SphinxPacket>> {
        let mut packets = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let packet = self.create_test_sphinx_packet()?;
            packets.push(packet);
        }
        Ok(packets)
    }

    /// Computes the network latency for a received packet by subtracting the configured
    /// sphinx delay from its measured round-trip time.
    fn packet_latency(&self, received: ProcessedPacket) -> Duration {
        received.rtt - self.probe.config.packet_delay
    }

    /// Creates and sends a single test sphinx packet.
    /// On send failure, records an error on the result and returns `false`.
    async fn send_test_packet(&mut self) -> anyhow::Result<bool> {
        let packet = self
            .create_test_sphinx_packet()
            .context("sphinx packet creation failure!")?;
        if let Err(err) = self.egress.send_packet(packet).await {
            self.result
                .set_error(format!("{:#}", err.context("failed to send test packet")));
            return Ok(false);
        };
        Ok(true)
    }

    /// Creates and sends a batch of `batch_size` test packets.
    /// On send failure, records an error on the result and returns `false`.
    async fn send_test_packet_batch(&mut self, batch_size: usize) -> anyhow::Result<bool> {
        let batch = self
            .create_packet_batch(batch_size)
            .context("sphinx packet batch creation failure!")?;

        if let Err(err) = self.egress.send_packet_batch(batch).await {
            self.result
                .set_error(format!("{:#}", err.context("failed to send test packet")));
            return Ok(false);
        };
        Ok(true)
    }

    /// Sends a single packet and waits for it to come back.
    /// On success, sets `approximate_latency` on the result and returns `true`.
    /// On failure, sets an error on the result and returns `false` (caller should abort).
    async fn send_connectivity_probe(&mut self) -> anyhow::Result<bool> {
        if !self.send_test_packet().await? {
            return Ok(false);
        }

        match self.inbox.next_packet().await {
            Ok(res) => {
                let latency = self.packet_latency(res);
                self.result.set_approximate_latency(latency);
                Ok(true)
            }
            Err(err) => {
                let err = err.context("failed to receive a valid initial packet back");

                // the node not answering at all and the node answering with an unusable connection
                // are different diagnoses - a dead node against, say, a stale noise key - so when its
                // return connection reported a failure, say so rather than only that nothing arrived
                self.result.set_error(match self.inbox.ingress_failure() {
                    Some(failure) => {
                        format!("{err:#} - the node's return connection failed: {failure}")
                    }
                    None => format!("{err:#}"),
                });
                Ok(false)
            }
        }
    }

    /// Replays a packet to verify that the node's bloomfilter bypass is correctly configured.
    /// Returns `true` if the packet was returned, `false` if the node failed the check (caller should abort).
    /// Should only be called when `config.reuse_header` is set.
    async fn send_bloomfilter_probe(&mut self) -> anyhow::Result<bool> {
        info!("repeating the packet to check bloomfilter bypass configuration");
        if !self.send_test_packet().await? {
            return Ok(false);
        }

        match self.inbox.next_packet().await {
            Ok(res) => {
                info!("received {res}");
                Ok(true)
            }
            Err(err) => {
                self.result.set_error(format!(
                    "{:#}",
                    err.context("failed to receive a valid secondary packet back - the node might not have a working chain subscriber (or the agent might be misconfigured)"))
                );
                Ok(false)
            }
        }
    }

    /// Sends packets at the profile's rate for its duration.
    /// Dispatches one batch every `batch_interval` seconds; if the egress falls behind,
    /// ticks are delayed rather than bunched up to avoid unintended bursts.
    /// Updates `result.packets_sent` after every batch and returns `false` on send failure.
    async fn send_load_test(&mut self) -> anyhow::Result<bool> {
        // one batch every (sending_batch_size / target_rate) seconds keeps us at the target rate
        let batch_interval = self.probe.profile.batch_interval();
        let mut interval = tokio::time::interval(batch_interval);
        // if we fall behind, don't try to catch up with burst sends
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let start = Instant::now();
        let mut sent = 0;
        let total_packets = self.probe.profile.expected_packets;
        let sending_duration = self.probe.profile.sending_duration;
        let sending_batch_size = self.probe.profile.sending_batch_size;

        loop {
            if start.elapsed() >= sending_duration {
                break;
            }
            if sent >= total_packets {
                break;
            }
            interval.tick().await;

            // the last batch may be smaller than other batches
            let remaining = total_packets - sent;
            let batch_size = sending_batch_size.min(remaining);
            if !self.send_test_packet_batch(batch_size).await? {
                return Ok(false);
            }

            sent += batch_size;
            // update send count after each batch so partial results are visible on early exit
            self.result.set_packets_sent(sent);
        }

        if sent < total_packets {
            warn!(
                "did not manage to send all required packets within the sending window. sent {sent}/{total_packets}"
            );
        }
        // Report `total_packets` (= expected) rather than `sent` so the orchestrator's
        // `received / sent` score formula effectively becomes `received / expected` -
        // a node that throttled us via TCP back-pressure into not pushing all packets
        // through is correctly penalised. Per-batch `set_packets_sent(sent)` updates
        // above remain in place for the `Ok(false)` early-exit (send error) path, so
        // partial-progress visibility is preserved when the test aborts mid-run.
        self.result.set_packets_sent(total_packets);
        Ok(true)
    }

    /// Drains all received packets from the inbox (waiting up to `waiting_duration` for
    /// stragglers), deduplicates by ID, computes RTT statistics, and populates the result.
    async fn collect_test_results(&mut self) {
        // drain whatever arrived immediately, then wait for stragglers
        let mut received = self.inbox.all_available();
        if received.len() < self.result.packets_sent {
            let deadline = sleep(self.probe.profile.waiting_duration);
            pin!(deadline);
            loop {
                tokio::select! {
                    _ = &mut deadline => break,
                    next = self.inbox.next_packet() => {
                        received.push(next);
                        if received.len() >= self.result.packets_sent {
                            break;
                        }
                    }
                }
            }
        }

        // deduplicate by packet ID; duplicates indicate possible node misbehaviour
        let mut valid_received = HashMap::new();
        for packet in received {
            let Ok(packet) = packet else {
                debug!("received packet was malformed");
                continue;
            };
            if valid_received.insert(packet.id, packet).is_some() {
                error!(
                    "‼️ received duplicate packet for id {} - something nasty is going on!",
                    packet.id
                );
                self.result.set_received_duplicates();
            }
        }

        let latencies = valid_received
            .values()
            .map(|p| self.packet_latency(*p))
            .collect::<Vec<_>>();

        let received_count = valid_received.len();
        self.result.set_packets_received(received_count);
        self.result
            .set_packets_statistics(LatencyDistribution::compute(&latencies));

        debug!(
            sent = self.result.packets_sent,
            received = received_count,
            recv_pct = format!("{:.1}%", self.result.received_percentage()),
            "load test complete"
        );
    }
}
