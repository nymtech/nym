// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The gateway liveness probe: one indivisible run measuring two interfaces over one client session.
//!
//! Named for the mixnet capability it measures, because `GatewayProbe` already means something else in
//! this repository: the prober that walks a gateway's VPN and exit path. Nothing here touches that.

use crate::agent::config::{NodeTesterConfig, ProbeProfile};
use crate::agent::gateway::result::GATEWAY_EXERCISED_INTERFACES;
use crate::agent::result::{LatencyDistribution, PacketDelivery, TestRunResult};
use crate::agent::tested_node::{TestedGatewayDetails, TestedNodeDetails};
use crate::mixnet::client_session::events::ReceivedPayload;
use crate::mixnet::client_session::inbox::GatewaySessionInbox;
use crate::mixnet::client_session::{GatewaySession, GatewaySessionTarget};
use crate::mixnet::egress::EgressConnection;
use crate::mixnet::inbox::TargetInbox;
use crate::mixnet::sphinx::helpers::{as_sphinx_node, create_test_sphinx_packet_header};
use crate::mixnet::sphinx::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::Context;
use humantime::format_duration;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::{ExercisedInterface, TestKind};
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::PacketType;
use nym_sphinx_types::{NymPacket, SphinxPacket};
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

pub(crate) mod result;
pub(crate) mod wave;

/// Neither leg of a gateway probe has a sphinx delay applied to it, so its packets ask for none.
///
/// A mixnode probe's two-hop route has the node apply the delay before forwarding, which is why it
/// subtracts one delay from every measured round trip. Here there is nothing to subtract: on the
/// ingest leg the gateway forwards the packet verbatim without any sphinx processing at all, and on
/// the delivery leg it is the FINAL hop, which carries no delay to apply. Asking for a delay and then
/// not subtracting it, or subtracting one that was never applied, would bias every latency figure by
/// its length.
const GATEWAY_PACKET_DELAY: Duration = Duration::ZERO;

/// A probe of ONE gateway: everything it needs before it starts, and the two phases it then runs.
///
/// The run proceeds in three ordered steps (see [`run`](Self::run)):
///
/// 1. Establish the client session, which is the only failure that abandons the whole run.
/// 2. Measure client ingest: forward packets through the session whose next hop is this agent, and
///    count what reaches the agent's own mixnet listener.
/// 3. Measure client delivery: send final-hop packets to the gateway's mixnet listener addressed to
///    this agent's client session, and count what the gateway pushes into that session.
///
/// Neither phase may abort the other. A gateway needs both capabilities to be useful, so both are
/// always measured and a phase that produced nothing is reported as a zero.
pub(crate) struct GatewayMixnetLivenessProbe {
    /// Tester configuration controlling timeouts and addressing.
    config: NodeTesterConfig,

    /// The liveness sending knobs, resolved once at construction. A gateway probe has no stress
    /// profile: there is no gateway stress assignment, and the two would not resemble each other.
    profile: ProbeProfile,

    /// The identity this agent presents when registering, which is also the destination the delivery
    /// phase addresses its packets to. It is the ANNOUNCED one: a freshly generated key would
    /// register fine and then be metered, since the gateway's exemption is a set membership test on
    /// what the contract holds.
    client_identity: Arc<ed25519::KeyPair>,

    /// The agent's Noise key, used by the delivery phase's egress connection.
    noise_key: Arc<x25519::KeyPair>,

    /// Header for the INGEST leg: a one-hop route to this agent, which the gateway forwards verbatim.
    ///
    /// A one-hop route makes the outbound and returning forms of a packet identical, since no earlier
    /// hop peels a layer off it: the payload is wrapped with the route's only key, which is also the
    /// key [`TestPacketHeader::recover_payload`] unwraps with. That is what lets the shared inbox
    /// recover these with an ordinary reusable-header strategy.
    ingest_header: TestPacketHeader,

    /// Header for the DELIVERY leg: a one-hop route to the gateway, addressed to this agent's client
    /// session.
    delivery_header: TestPacketHeader,

    /// The gateway under test.
    target: TestedGatewayDetails,
}

impl GatewayMixnetLivenessProbe {
    pub(crate) fn new(
        config: NodeTesterConfig,
        client_identity: Arc<ed25519::KeyPair>,
        noise_key: Arc<x25519::KeyPair>,
        target: TestedGatewayDetails,
    ) -> anyhow::Result<Self> {
        let profile = config.profile_for(TestKind::Liveness);

        // an ephemeral sphinx key, needed only to build the ingest header below and deliberately not
        // retained: the header carries the payload key derived from it, and that is what recovers a
        // returning packet, so nothing afterwards needs the private half
        let sphinx_key: x25519::KeyPair = x25519::PrivateKey::new(&mut OsRng).into();

        debug!("probing gateway {} under {profile:?}", target.identity);
        debug!("{target:#?}");

        let client_address = client_identity.public_key().derive_destination_address();

        // both headers are built ONCE here rather than per packet, for the same reason the mixnode
        // probe does it: a header costs a full sphinx construction plus the payload key derivation,
        // while stamping a fresh payload into an existing one costs an encapsulation
        let ingest_header = create_test_sphinx_packet_header(
            &[as_sphinx_node(
                config.return_address_for(target.mixnet.address),
                *sphinx_key.public_key(),
            )],
            client_address,
            GATEWAY_PACKET_DELAY,
        )
        .context("failed to build the ingest leg's sphinx header")?;

        let delivery_header = create_test_sphinx_packet_header(
            &[target.mixnet.as_sphinx_node()],
            client_address,
            GATEWAY_PACKET_DELAY,
        )
        .context("failed to build the delivery leg's sphinx header")?;

        Ok(GatewayMixnetLivenessProbe {
            config,
            profile,
            client_identity,
            noise_key,
            ingest_header,
            delivery_header,
            target,
        })
    }

    /// The node this probe measures, as the wave's shared ingress knows it.
    pub(crate) fn tested_node(&self) -> &TestedNodeDetails {
        &self.target.mixnet
    }

    /// Builds the inbox for the INGEST phase's arrivals.
    ///
    /// Handed out rather than built inside [`run`](Self::run) for the same reason the mixnode probe
    /// does it: the wave's ingress has to be assembled from every target's inbox before any probe
    /// starts, since one listener serves them all and can only route to channels that exist.
    ///
    /// Those arrivals are attributed to this target by the GATEWAY's source address, because a
    /// forwarded packet reaches us over the gateway's own outbound mixnet connection. The existing
    /// per-target routing therefore needs nothing added for this phase.
    pub(crate) fn build_ingest_inbox(&self) -> TargetInbox {
        TargetInbox::new(
            self.ingest_header.clone().into(),
            self.profile.waiting_duration,
        )
    }

    /// How many packets ONE phase sends.
    ///
    /// The profile's count in FULL, not split between the two phases. The count is chosen for score
    /// granularity, so halving it would make each of a gateway's interfaces coarser than a mixnode's
    /// single one, and the volume a gateway wave costs is already held level with a mixnode wave's
    /// from the other side: the orchestrator hands out half as many gateway targets per wave.
    fn packets_per_phase(&self) -> usize {
        self.profile.expected_packets
    }

    /// Runs both phases against the target and returns what they measured.
    ///
    /// Returns `Err` only for a critical failure on OUR side. A gateway that refuses the session or
    /// swallows every packet is a zero-scoring result, not an error, so that it still gets submitted
    /// and still advances that pairing's staleness.
    pub(crate) async fn run(
        self,
        ingest_inbox: TargetInbox,
        deadline: Option<Duration>,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<TestRunResult> {
        let identity = self.target.identity;
        info!("beginning the gateway probe of {identity}");

        // stamped BEFORE the session is established, so the reported duration includes getting one.
        // seeded with both interfaces, so it is already submittable as a pair of zeros
        let mut result = TestRunResult::new(
            TestKind::Liveness,
            GATEWAY_PACKET_DELAY,
            GATEWAY_EXERCISED_INTERFACES,
        );

        let (session, delivery_arrivals) = match GatewaySession::establish(
            self.session_target(),
            &self.client_identity,
            self.config.gateway_session_config(),
            shutdown,
        )
        .await
        {
            Ok(established) => established,
            Err(err) => {
                // the ONE failure that zeroes both measurements, and the only one that makes the node
                // unreachable rather than merely unmeasured
                warn!("could not establish a client session with {identity}: {err:#}");
                result.set_error(format!(
                    "{:#}",
                    err.context("failed to establish the gateway client session")
                ));
                return Ok(result);
            }
        };

        let mut run = GatewayRun::new(self, session, delivery_arrivals, ingest_inbox, result);
        run.run_to_deadline(deadline).await;
        let result = run.finish().await;

        info!("finished the gateway probe of {identity}");
        Ok(result)
    }

    /// This gateway as a client session's target.
    fn session_target(&self) -> GatewaySessionTarget {
        self.target.session_target()
    }
}

/// One gateway run in flight: the live resources, plus the state that only exists while it runs.
///
/// Separate from [`GatewayMixnetLivenessProbe`] for the same reason `ProbeRun` is separate from
/// `NodeProbe`: the session is established during the run and can fail, so it cannot be a field
/// without becoming an `Option` that every send path has to unwrap.
struct GatewayRun {
    probe: GatewayMixnetLivenessProbe,

    /// The live session. Both phases share it, and it stays open across both and their drain windows.
    session: GatewaySession,

    /// The DELIVERY phase's arrivals, pushed into the session by the gateway.
    delivery_arrivals: GatewaySessionInbox,

    /// The INGEST phase's arrivals, forwarded by the gateway to this agent's mixnet listener.
    ingest_arrivals: TargetInbox,

    /// The run-level frame, seeded with both interfaces. Handed in rather than created here, because
    /// its start time has to predate the session.
    result: TestRunResult,

    /// What the INGEST phase is measuring, folded into [`Self::result`] by
    /// [`finish`](Self::finish). Held beside it for the same reason `ProbeRun` does: the send path
    /// updates counters without an `Option` lookup per write.
    ingest_measured: PacketDelivery,

    /// What the DELIVERY phase is measuring.
    delivery_measured: PacketDelivery,

    /// Monotonically increasing id stamped into each outgoing packet, shared by both phases so that
    /// no id is issued twice within a run.
    ///
    /// Deliberately NOT split into a range per phase. A range would only buy the detection of a
    /// packet arriving on the wrong leg, and the two legs cannot be confused: an ingest packet is
    /// addressed to this agent's ephemeral sphinx key and arrives over Noise on the mixnet listener,
    /// while a delivery packet is addressed to the gateway's key and arrives already unwrapped over
    /// the websocket. Nothing can deliver one as the other.
    packet_counter: u64,
}

impl GatewayRun {
    fn new(
        probe: GatewayMixnetLivenessProbe,
        session: GatewaySession,
        delivery_arrivals: GatewaySessionInbox,
        ingest_arrivals: TargetInbox,
        result: TestRunResult,
    ) -> Self {
        GatewayRun {
            probe,
            session,
            delivery_arrivals,
            ingest_arrivals,
            result,
            ingest_measured: PacketDelivery::default(),
            delivery_measured: PacketDelivery::default(),
            packet_counter: 0,
        }
    }

    /// Runs both phases, cut off at `deadline` if one is set.
    ///
    /// The deadline wraps the sequence HERE rather than the caller wrapping the whole probe, because a
    /// dropped future would take its partial result with it: a run cut off during its delivery phase
    /// still has to report the ingest phase it completed.
    async fn run_to_deadline(&mut self, deadline: Option<Duration>) {
        let Some(deadline) = deadline else {
            self.execute().await;
            return;
        };

        let identity = self.probe.target.identity;
        if timeout(deadline, self.execute()).await.is_err() {
            let deadline = format_duration(deadline);
            warn!("the gateway probe of {identity} did not complete within {deadline}");
            // run-level, because a deadline cuts off whatever was in flight rather than implicating
            // one interface. whichever phases had already finished keep their own figures
            self.result
                .set_error(format!("the probe did not complete within {deadline}"));
        }
    }

    /// Runs the two phases in order.
    ///
    /// The ingest phase runs first and its failure is recorded WITHOUT returning: the phases test
    /// independent capabilities, so a gateway with a broken session path may still deliver perfectly
    /// and has to be measured doing it.
    async fn execute(&mut self) {
        if let Err(err) = self.measure_client_ingest().await {
            warn!(
                "the client ingest phase against {} failed: {err:#}",
                self.probe.target.identity
            );
            self.ingest_measured.set_error(format!("{err:#}"));
        }

        if let Err(err) = self.measure_client_delivery().await {
            warn!(
                "the client delivery phase against {} failed: {err:#}",
                self.probe.target.identity
            );
            self.delivery_measured.set_error(format!("{err:#}"));
        }
    }

    /// Measures client ingest: forward through the session, receive on the mixnet listener.
    ///
    /// Nothing here exercises the gateway's sphinx layer at all, since the envelope names the next hop
    /// explicitly and the gateway forwards the packet verbatim. A loss on this phase therefore
    /// implicates the session, the bandwidth path or the outbound forwarder, which is exactly what
    /// makes the two phases worth separating.
    async fn measure_client_ingest(&mut self) -> anyhow::Result<()> {
        let expected = self.probe.packets_per_phase();
        debug!("measuring client ingest with {expected} packet(s)");

        // reported as the intended count rather than what was pushed, so a gateway that throttles us
        // is penalised rather than flattered
        self.ingest_measured.packets_sent = expected;

        let batches = self.plan_batches(expected);
        let mut interval = self.dispatch_interval();

        for batch_size in batches {
            interval.tick().await;

            let mut batch = Vec::with_capacity(batch_size);
            for _ in 0..batch_size {
                batch.push(self.next_ingest_packet()?);
            }
            self.session.forward_batch(batch).await?;
        }

        let received = self.drain_ingest_arrivals(expected).await;
        summarise(&mut self.ingest_measured, received);
        self.report_session_treatment();

        Ok(())
    }

    /// Logs whether the gateway would take our forwards at all.
    ///
    /// The only thing worth distinguishing, and the reason it is worth logging: a session refused for
    /// want of bandwidth means this gateway does not hold our announced identity, so the phase's zero
    /// is ours to explain rather than the gateway's. HOW a gateway came to accept a packet is not
    /// examined, because a liveness probe measures only whether the packet got through.
    fn report_session_treatment(&self) {
        let identity = self.probe.target.identity;

        // reported alongside the verdict below rather than instead of it. we do not close the session
        // until both phases are done, so a close observed HERE is the gateway's doing, and it means
        // anything still in flight was never going to arrive - including the delivery phase, which
        // has not run yet and now has nowhere to be delivered to
        if let Some(reason) = self.delivery_arrivals.closed() {
            match reason {
                Some(reason) => warn!("{identity} ended our session early: {reason}"),
                None => warn!("{identity} ended our session early without giving a reason"),
            }
        }

        if self.delivery_arrivals.accepted() > 0 {
            debug!(
                "{identity} accepted {} of our forwards",
                self.delivery_arrivals.accepted()
            );
            return;
        }

        match self.delivery_arrivals.refusal() {
            Some(refusal) => warn!(
                "{identity} accepted none of our forwards, refusing with: {refusal}. an out-of-bandwidth refusal means this session was METERED, so the gateway is not treating us as an authorised monitor: it either predates the monitor-session exemption or has not ingested our announced identity"
            ),
            None => warn!("{identity} acknowledged none of our forwards and gave no reason"),
        }
    }

    /// Measures client delivery: send over Noise to the mixnet listener, receive on the session.
    ///
    /// Opening the egress connection is part of THIS phase rather than of the run, so a gateway whose
    /// mixnet listener refuses us still reports the ingest phase it passed.
    async fn measure_client_delivery(&mut self) -> anyhow::Result<()> {
        let expected = self.probe.packets_per_phase();
        debug!("measuring client delivery with {expected} packet(s)");

        let mut egress = self
            .establish_egress_connection()
            .await
            .context("failed to establish the egress connection to the gateway")?;

        self.delivery_measured.packets_sent = expected;

        let batches = self.plan_batches(expected);
        let mut interval = self.dispatch_interval();

        for batch_size in batches {
            interval.tick().await;

            let mut batch = Vec::with_capacity(batch_size);
            for _ in 0..batch_size {
                batch.push(self.next_delivery_packet()?);
            }
            egress.send_packet_batch(batch).await?;
        }

        let received = self.drain_delivery_arrivals(expected).await;
        summarise(&mut self.delivery_measured, received);

        // this leg DOES speak Noise, so it has an egress handshake to report. the ingest leg has
        // neither figure, since a websocket authenticated by the registration handshake involves no
        // Noise at all
        self.delivery_measured
            .set_egress_connection_statistics(egress.connection_statistics);

        Ok(())
    }

    /// How many packets each dispatch of a phase sends.
    ///
    /// Both phases pace identically, so the plan is derived once from the profile rather than each
    /// phase deciding: a burst is the one thing a liveness probe must not do, since it measures
    /// delivery and would then be measuring the target under load.
    fn plan_batches(&self, expected: usize) -> Vec<usize> {
        let batch = self.probe.profile.sending_batch_size.max(1);
        let mut batches = Vec::new();
        let mut planned = 0;

        while planned < expected {
            let size = batch.min(expected - planned);
            batches.push(size);
            planned += size;
        }

        batches
    }

    /// The ticker that paces a phase's dispatches, matching the profile's target rate.
    fn dispatch_interval(&self) -> tokio::time::Interval {
        let mut interval = tokio::time::interval(self.probe.profile.batch_interval());
        // if we fall behind, don't try to catch up with burst sends
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval
    }

    /// Drains the ingest phase's arrivals, waiting out the straggler window for the ones still in
    /// flight.
    async fn drain_ingest_arrivals(&mut self, expected: usize) -> Vec<Measured> {
        let mut received = self
            .ingest_arrivals
            .all_available()
            .into_iter()
            .filter_map(|packet| match packet {
                Ok(packet) => Some(Measured {
                    id: packet.id,
                    latency: packet.rtt,
                }),
                Err(err) => {
                    debug!("a forwarded packet was malformed: {err:#}");
                    None
                }
            })
            .collect::<Vec<_>>();

        if received.len() < expected {
            let straggler_wait = sleep(self.probe.profile.waiting_duration);
            pin!(straggler_wait);

            loop {
                tokio::select! {
                    _ = &mut straggler_wait => break,
                    next = self.ingest_arrivals.next_packet() => match next {
                        Ok(packet) => {
                            received.push(Measured { id: packet.id, latency: packet.rtt });
                            if received.len() >= expected {
                                break;
                            }
                        }
                        Err(err) => {
                            debug!("stopped draining forwarded packets: {err:#}");
                            break;
                        }
                    },
                }
            }
        }

        received
    }

    /// Drains the delivery phase's arrivals, waiting out the straggler window.
    ///
    /// No sphinx recovery here: the gateway was the packet's final hop and unwrapped the payload
    /// before pushing it, so what arrives is the test content's bytes and only its round trip has to
    /// be worked out.
    async fn drain_delivery_arrivals(&mut self, expected: usize) -> Vec<Measured> {
        let mut received = self
            .delivery_arrivals
            .all_available()
            .into_iter()
            .filter_map(Measured::from_payload)
            .collect::<Vec<_>>();

        if received.len() < expected {
            let straggler_wait = sleep(self.probe.profile.waiting_duration);
            pin!(straggler_wait);

            loop {
                tokio::select! {
                    _ = &mut straggler_wait => break,
                    next = self.delivery_arrivals.next_payload() => match next {
                        Ok(payload) => {
                            if let Some(measured) = Measured::from_payload(payload) {
                                received.push(measured);
                                if received.len() >= expected {
                                    break;
                                }
                            }
                        }
                        Err(err) => {
                            debug!("stopped draining delivered payloads: {err:#}");
                            break;
                        }
                    },
                }
            }
        }

        received
    }

    /// Builds the next ingest-phase packet: a sphinx packet whose only hop is this agent, wrapped in
    /// an envelope naming this agent's mixnet address as the next hop.
    fn next_ingest_packet(&mut self) -> anyhow::Result<MixPacket> {
        // the id is taken before the header is touched: the builder borrows the probe immutably,
        // which cannot overlap the mutable borrow bumping the counter
        let content = self.next_content();
        let packet = self
            .probe
            .ingest_header
            .create_test_packet(content)
            .context("failed to build an ingest packet")?;

        // the gateway forwards this verbatim to whatever the envelope names, and what it names is the
        // agent's own listener in the family this run is being conducted over
        let next_hop = NymNodeRoutingAddress::from(
            self.probe
                .config
                .return_address_for(self.probe.target.mixnet.address),
        );

        Ok(MixPacket::new(
            next_hop,
            NymPacket::Sphinx(packet),
            PacketType::Mix,
            self.probe.target.mixnet.key_rotation,
        ))
    }

    /// Builds the next delivery-phase packet: a sphinx packet whose only hop is the gateway, addressed
    /// to this agent's own client session.
    ///
    /// Ack-sized on purpose, and not merely to stay small: a final hop of that size carries no
    /// SURB-Ack, so the node hands the payload to the session whole instead of trying to recover an
    /// ack from the front of it.
    fn next_delivery_packet(&mut self) -> anyhow::Result<SphinxPacket> {
        let content = self.next_content();
        self.probe
            .delivery_header
            .create_test_packet(content)
            .context("failed to build a delivery packet")
    }

    /// The next packet's content, taking the run's next id.
    fn next_content(&mut self) -> TestPacketContent {
        let content = TestPacketContent::new(self.packet_counter);
        self.packet_counter += 1;
        content
    }

    /// Opens the delivery phase's outbound Noise connection to the gateway's mixnet listener.
    async fn establish_egress_connection(&self) -> anyhow::Result<EgressConnection> {
        let target = &self.probe.target.mixnet;

        // scoped to the one address being dialled, exactly as the mixnode probe's is: the initiator
        // looks the responder up by the address it dialled
        let nodes = HashMap::from([(target.address.ip().to_canonical(), target.as_noise_node())]);
        let noise_config = NoiseConfig::new(
            self.probe.noise_key.clone(),
            NoiseNetworkView::new(nodes),
            self.probe.config.noise_handshake_timeout,
        );

        EgressConnection::establish(
            target.address,
            self.probe.config.egress_connection_timeout,
            target.key_rotation,
            &noise_config,
        )
        .await
    }

    /// Closes the session and folds both phases into the result.
    ///
    /// The session is closed HERE and nowhere earlier: it is held open across both phases and their
    /// drain windows, because the delivery phase needs a live session at the moment its packets reach
    /// the gateway and one closed early turns a delivered packet into a dropped one.
    ///
    /// Whatever happened, this yields two measurements: the seeded slots are what makes a phase that
    /// never ran a zero rather than an absence, so nothing here has to remember to fill one in.
    async fn finish(self) -> TestRunResult {
        let GatewayRun {
            session,
            mut result,
            ingest_measured,
            delivery_measured,
            ..
        } = self;

        session.close().await;

        result
            .measurements
            .record(ExercisedInterface::ClientIngest, ingest_measured);
        result
            .measurements
            .record(ExercisedInterface::ClientDelivery, delivery_measured);

        result
    }
}

/// One packet that came back, reduced to what scoring needs.
///
/// The two legs measure the same two things by different means, so they are reduced to a common shape
/// before being counted: the ingest leg's inbox has already computed a round trip from the sphinx
/// payload, while the delivery leg's payload arrives raw and its timestamp has to be read out.
struct Measured {
    id: u64,
    latency: Duration,
}

impl Measured {
    /// Reads a delivered payload's id and round trip out of its content.
    ///
    /// `None` for anything that is not one of our test packets, which is not counted at all: a
    /// gateway may push a payload some other client sent us, and counting it would credit the gateway
    /// with a delivery we never asked for.
    fn from_payload(payload: ReceivedPayload) -> Option<Self> {
        let content = TestPacketContent::from_bytes(&payload.payload)
            .inspect_err(|err| debug!("a delivered payload was not one of ours: {err:#}"))
            .ok()?;

        Some(Measured {
            id: content.id,
            latency: (payload.received_at - content.sending_timestamp).unsigned_abs(),
        })
    }
}

/// Counts what came back into `measured`, collapsing duplicates.
///
/// A duplicate is recorded rather than merely dropped: an honest node never replays a packet, and the
/// orchestrator scores an interface that saw one as zero, since the ratio alone cannot tell a node
/// that forwarded everything from one that echoed a single packet many times.
fn summarise(measured: &mut PacketDelivery, received: Vec<Measured>) {
    let mut by_id = HashMap::new();
    for packet in received {
        if by_id.insert(packet.id, packet.latency).is_some() {
            error!(
                "‼️ received a duplicate packet for id {} - something nasty is going on!",
                packet.id
            );
            measured.received_duplicates = true;
        }
    }

    let latencies = by_id.values().copied().collect::<Vec<_>>();
    measured.packets_received = by_id.len();
    measured.packets_statistics = Some(LatencyDistribution::compute(&latencies));

    // `approximate_latency` is deliberately left empty. It means the round trip of a single packet
    // sent in ISOLATION before any load, which is a baseline neither of these phases establishes:
    // there is no connectivity probe to abort on, since a phase producing nothing has to be measured
    // as a zero rather than cut short. Anything else put here (a minimum, a first arrival) would be a
    // figure under load wearing the name of one taken without it.

    debug!(
        sent = measured.packets_sent,
        received = measured.packets_received,
        recv_pct = format!("{:.1}%", measured.received_percentage()),
        "phase complete"
    );
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use nym_network_monitor_orchestrator_requests::models as api;
    use nym_test_utils::helpers::deterministic_rng;

    /// A gateway at an address nothing is listening on.
    ///
    /// Port 1 on loopback refuses immediately rather than hanging, so this exercises the failure
    /// without waiting out a timeout and without binding anything of its own.
    fn unreachable_gateway() -> TestedGatewayDetails {
        let mut rng = deterministic_rng();

        TestedGatewayDetails {
            mixnet: TestedNodeDetails::new_test(
                "127.0.0.1:1".parse().expect("bad test address"),
                &["127.0.0.1".parse().expect("bad test ip")],
            ),
            identity: *ed25519::KeyPair::new(&mut rng).public_key(),
            clients_ws_port: 1,
        }
    }

    fn probe(target: TestedGatewayDetails) -> GatewayMixnetLivenessProbe {
        let mut rng = deterministic_rng();

        GatewayMixnetLivenessProbe::new(
            NodeTesterConfig::new_test(),
            Arc::new(ed25519::KeyPair::new(&mut rng)),
            Arc::new(x25519::KeyPair::new(&mut rng)),
            target,
        )
        .expect("failed to build the probe")
    }

    // failing to establish the session is the ONE failure that zeroes both measurements, and it must
    // still produce a submittable result: an `Err` here would leave the target unreported, so the
    // orchestrator would hold its lease and the gateway would keep its turn instead of being scored
    #[tokio::test]
    async fn a_session_that_cannot_be_established_yields_two_zeroed_measurements() {
        let probe = probe(unreachable_gateway());
        let ingest_inbox = probe.build_ingest_inbox();

        let result = probe
            .run(ingest_inbox, None, ShutdownToken::new())
            .await
            .expect("a gateway that would not take a session was reported as our own failure");

        // run-level, because it is the whole run that could not happen. this is what the orchestrator
        // reads `was_reachable` off
        assert!(result.error.is_some(), "{result:#?}");

        let wire: api::TestRunResult = result.into();
        assert_eq!(wire.measurements.len(), 2);
        assert!(
            wire.measurements
                .iter()
                .all(|measurement| measurement.received_ratio() == 0.0),
            "{:#?}",
            wire.measurements
        );
    }

    // the two interfaces are fixed by the kind rather than by what a run managed to measure, so even
    // a run that never reached the gateway reports them both, in a stable order
    #[tokio::test]
    async fn both_interfaces_are_reported_even_by_a_run_that_measured_nothing() {
        let probe = probe(unreachable_gateway());
        let ingest_inbox = probe.build_ingest_inbox();

        let result = probe
            .run(ingest_inbox, None, ShutdownToken::new())
            .await
            .expect("the probe failed on our side");

        let wire: api::TestRunResult = result.into();
        assert_eq!(
            wire.measurements
                .iter()
                .map(|measurement| measurement.interface)
                .collect::<Vec<_>>(),
            vec![
                ExercisedInterface::ClientIngest,
                ExercisedInterface::ClientDelivery
            ]
        );
    }
}
