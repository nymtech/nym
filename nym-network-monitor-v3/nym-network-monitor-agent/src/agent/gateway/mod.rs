// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The gateway liveness probe: one indivisible run measuring two interfaces over one client session.
//!
//! Named for the mixnet capability it measures, because `GatewayProbe` already means something else in
//! this repository: the prober that walks a gateway's VPN and exit path. Nothing here touches that.

// SCAFFOLD: the bodies land as group 9's tasks are worked through, at which point both allows come off
#![allow(dead_code, unused_variables)]

use crate::agent::config::{NodeTesterConfig, ProbeProfile};
use crate::agent::gateway::result::GATEWAY_EXERCISED_INTERFACES;
use crate::agent::result::{PacketDelivery, TestRunResult};
use crate::agent::tested_node::{TestedGatewayDetails, TestedNodeDetails};
use crate::mixnet::client_session::inbox::GatewaySessionInbox;
use crate::mixnet::client_session::{GatewaySession, GatewaySessionConfig};
use crate::mixnet::egress::EgressConnection;
use crate::mixnet::inbox::TargetInbox;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_types::{DestinationAddressBytes, SphinxPacket};
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod result;
pub(crate) mod wave;

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

    /// An ephemeral sphinx key pair. The ingest phase's packets name this agent as their final hop, so
    /// they come back to be opened with this.
    sphinx_key: Arc<x25519::KeyPair>,

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
        todo!()
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
        todo!()
    }

    /// The timeouts this probe holds its session to.
    // TODO: these want their own knobs on `NodeTesterConfig` and the CLI, rather than being derived
    // from the mixnet ones, since a websocket upgrade and a Noise handshake are not the same wait
    fn session_config(&self) -> GatewaySessionConfig {
        todo!()
    }

    /// How many packets ONE phase sends.
    ///
    /// OPEN: the profile's count is per TARGET and a gateway target has two phases. Splitting it keeps
    /// a gateway wave's volume level with a mixnode wave's; sending it in full twice keeps each
    /// interface's score granularity level with a mixnode's. Undecided, so it sits behind one accessor
    /// rather than being spread across both phases' call sites.
    fn packets_per_phase(&self) -> usize {
        todo!()
    }

    /// This agent's client address, which is both what the gateway resolves a delivered packet to and
    /// what it looked our exemption up by.
    fn client_address(&self) -> DestinationAddressBytes {
        self.client_identity
            .public_key()
            .derive_destination_address()
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
    ) -> anyhow::Result<TestRunResult> {
        // 1. stamp the result BEFORE establishing the session, so the reported duration includes it.
        //    seeded with both interfaces, so it is already submittable as a pair of zeros
        let _ = TestRunResult::new(
            TestKind::Liveness,
            self.config.packet_delay,
            GATEWAY_EXERCISED_INTERFACES,
        );
        // 2. establish the session; on failure set the RUN-level error and return, which is the one
        //    failure that makes the node unreachable rather than merely unmeasured
        // 3. hand the live resources to `GatewayRun` and run both phases under the deadline
        todo!()
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

    /// Monotonically increasing id stamped into each outgoing packet.
    // TODO: decide whether the two phases share this counter or take disjoint ranges. Sharing is
    // simpler; disjoint ranges make a packet that came back on the wrong leg a detectable defect
    // rather than an invisible one
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
        todo!()
    }

    /// Runs both phases, cut off at `deadline` if one is set.
    ///
    /// The deadline wraps the sequence HERE rather than the caller wrapping the whole probe, because a
    /// dropped future would take its partial result with it: a run cut off during its delivery phase
    /// still has to report the ingest phase it completed.
    async fn run_to_deadline(&mut self, deadline: Option<Duration>) -> anyhow::Result<()> {
        todo!()
    }

    /// Runs the two phases in order.
    ///
    /// The ingest phase runs first and its failure is recorded WITHOUT returning: the phases test
    /// independent capabilities, so a gateway with a broken session path may still deliver perfectly
    /// and has to be measured doing it.
    async fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    /// Measures client ingest: forward through the session, receive on the mixnet listener.
    ///
    /// Nothing here exercises the gateway's sphinx layer at all, since the envelope names the next hop
    /// explicitly and the gateway forwards the packet verbatim. A loss on this phase therefore
    /// implicates the session, the bandwidth path or the outbound forwarder, which is exactly what
    /// makes the two phases worth separating.
    async fn measure_client_ingest(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    /// Measures client delivery: send over Noise to the mixnet listener, receive on the session.
    ///
    /// Opening the egress connection is part of THIS phase rather than of the run, so a gateway whose
    /// mixnet listener refuses us still reports the ingest phase it passed.
    async fn measure_client_delivery(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    /// Builds the next ingest-phase packet: a sphinx packet whose only hop is this agent, wrapped in
    /// an envelope naming this agent's mixnet address as the next hop.
    ///
    // TODO: this is a ONE-hop route, which `build_test_sphinx_packet` and
    // `create_test_sphinx_packet_header` cannot express: both are fixed at `[Node; 2]` and the latter
    // asserts two payload keys. Generalising them over route length is part of this group's work
    fn next_ingest_packet(&mut self) -> anyhow::Result<MixPacket> {
        todo!()
    }

    /// Builds the next delivery-phase packet: a sphinx packet whose only hop is the gateway, addressed
    /// to this agent's own client session.
    ///
    /// Ack-sized on purpose, and not merely to stay small: a final hop of that size carries no
    /// SURB-Ack, so the node hands the payload to the session whole instead of trying to recover an
    /// ack from the front of it.
    ///
    // TODO: needs a real destination, where the mixnode probe uses `dummy_destination()`. The gateway
    // resolves a delivered packet by exactly this address, so a zeroed one is dropped
    fn next_delivery_packet(&mut self) -> anyhow::Result<SphinxPacket> {
        todo!()
    }

    /// Opens the delivery phase's outbound Noise connection to the gateway's mixnet listener.
    async fn establish_egress_connection(&self) -> anyhow::Result<EgressConnection> {
        todo!()
    }

    /// Folds both phases into the result.
    ///
    /// Whatever happened, this yields two measurements: the seeded slots are what makes a phase that
    /// never ran a zero rather than an absence, so nothing here has to remember to fill one in.
    fn finish(self) -> TestRunResult {
        todo!()
    }
}
