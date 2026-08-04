## Context

Network monitor v3 today performs exactly one kind of measurement: a high-rate two-hop stress test against a single mixnode, routed `[tested_node, agent]` so the node relays each packet straight back. The orchestrator assigns work lazily from a staleness-ordered node table guarded by a `testrun_in_progress` lock set, agents are one-shot jobs that test exactly one node and exit, and results are submitted to nym-api in signed monotonic batches that feed the stress component of node performance.

Liveness scoring meanwhile lives in the nym-api-internal v1 monitor, which measures delivery over five-node routes and attributes the result to one substituted node. Its bias is documented in its own design record and is the motivation for this change (see proposal.md).

The relevant constraints are:

- **The `network-monitors` contract cannot be redeployed.** Nothing here may require a new contract field, message, or query. Agent authorisation, and therefore the monitor identification that the node-side changes depend on, must reuse the existing socket-address plus x25519-noise-key entries.
- **Agent and orchestrator redeploy together**, so wire changes between them are free. nym-node and nym-api do not, so anything they must learn is a compatibility surface.
- **Nodes learn the authorised-agent set only through a nyxd websocket event subscription** with no periodic reconciliation. A node that misses an event only re-syncs on restart. This is the load-bearing risk for a liveness score, because a node that has not ingested its agents' authorisations fails every gate and is indistinguishable from a dead node.
- **Node IP addresses are unique across the node population** (nodes attribute each other's Noise keys by IP), whereas several agents may share one IP disambiguated by port.
- Announced address sets are canonicalised, deduplicated and sorted, and test runs rotate through them by position.

## Goals / Non-Goals

**Goals:**

- Produce a per-node liveness score that depends on that node and the probing agent's own link, and on nothing else in the network.
- Cover a gateway's two real jobs: forwarding client traffic into the mixnet, and delivering mixnet traffic to a client.
- Sweep the whole testable population at a cadence comparable to v1's 15 minutes.
- Ship in a way that cannot damage rewarding while the node and gateway fleets are still upgrading, and produce the evidence needed to decide when liveness may replace the v1 routing score.
- Leave the existing stress test's behaviour and scores unchanged.

**Non-Goals:**

- Removing or altering the v1 monitor. It keeps running and keeps feeding the routing component; the cutover is a later change gated on the evidence this one produces.
- Testing a gateway's wss / hostname ingress path. Recorded as a future test kind.
- Testing wireguard, SOCKS5, the authenticator, or anything else `nym-gateway-probe` and the node-status agent cover.
- Latency-derived scoring. Latency is recorded, not scored (Decision 11).
- Any change to the `network-monitors` contract, or to the source-IP basis of the node-side authorisation gates. Moving those gates onto the Noise-authenticated static key remains the separately-identified follow-on.
- Removing gateway on-disk message storage for ordinary clients.

## Decisions

### Decision 1: Liveness is a second test kind inside the same subsystem

**Choice.** Liveness is a `test_kind` handled by the same orchestrator, the same agents, the same chain authorisation, and the same submission path as stress testing. Work becomes keyed by `(node_id, test_kind)` rather than by `node_id`.

**Why.** Every actor, credential and lifecycle stage is already in place; the only genuinely new things are the probe profile, the gateway phases, and per-kind scheduling. A separate subsystem would duplicate the authorisation model, the node registry, and the submission machinery for no gain.

**Alternative considered.** A distinct capability and daemon. Rejected as duplication.

**Consequence.** Several structures that assume one test per node need widening (the staleness pointer, the rotation cursor, the submission watermark), and the assignment API grows a kind discriminator. The `testrun_in_progress` primary key on `node_id` alone is deliberately NOT widened: it is what keeps two kinds from measuring one node simultaneously.

### Decision 2: Mixnode liveness is the existing probe at a low-volume profile, still replaying one header

**Choice.** The mixnode liveness probe is the existing two-hop self-loop with its own profile (order 100 packets, lower rate, shorter straggler wait), keeping `reuse_header` enabled.

**Why.** The measurement being replaced is a delivery ratio, and the existing probe already produces one with correct attribution. Keeping `reuse_header` keeps a single code path through the node (Noise responder, routing filter, replay bypass, forward hop) so a liveness result and a stress result describe the same machinery, and it avoids rebuilding a header per packet across a whole wave.

**Alternative considered.** Fresh headers per packet, which would decouple liveness from the replay/bloomfilter bypass and make wave attribution trivial (Decision 6). Rejected because it introduces no new independence in practice: the bypass is gated on the same authorised-agent set as the Noise and routing gates, so a node that fails one fails all three.

**Consequence.** A node whose chain subscriber is broken fails liveness for a reason unrelated to its forwarding capability. That is the central risk of the change and is handled by shipping at weight zero (Decision 12), not by the probe design.

### Decision 3: Gateway liveness is one indivisible test with two phases over one client session

**Choice.** A gateway liveness assignment is a single unit of work performed by a single agent within a single client session, in two phases:

```
phase 1, ingress                          phase 2, egress
agent(client) --ws:ForwardSphinx--> GW     agent(as mixnode) --mixnet--> GW
  GW forwards verbatim to next hop           GW: Noise responder, final-hop unwrap,
  --> agent's own mixnet listener            destination resolution
                                             --> push into the live client session
tests: session, bandwidth path,           tests: mixnet ingress, sphinx unwrap,
       outbound forwarder + Noise                client delivery
```

The run produces two signals. The score denominator is fixed by the kind at two signals, so a phase that produces nothing scores zero rather than being dropped from the average. A phase-1 failure MUST NOT abort the run. Only failure to establish the session aborts, in which case both signals are zero.

**Why.** The two phases test independent capabilities and a gateway needs both to be useful, so both must always be measured and a missing signal must never be more favourable than a zero one. This mirrors v1, which seeds every tested node at zero received so that an unreachable node scores 0 rather than being omitted. Sharing one session is required anyway, because final-hop delivery needs a live session at the moment the packet arrives.

**Alternative considered.** A single combined loop (`client -> GW -> agent-as-mix -> GW -> client`) exercising both directions per packet. Rejected because it halves the packet budget but destroys direction attribution, which is the entire reason for going minimal-hop. Also considered and rejected: separate assignments per phase, which would allow two agents to measure half a gateway each and produce results nobody can compose.

**Consequence.** An agent that dies after phase 1 abandons the whole run rather than resuming it. Phase 1 involves no sphinx processing at the gateway at all (the client supplies an explicit next hop and the gateway forwards verbatim), so a phase-1 failure implicates the session, the bandwidth path, or the outbound forwarder, never the sphinx layer. Neither phase exercises `forward_hop_processing_enabled`, which is derived from `modes.mixnode` and is off on a pure gateway, so the tests can never accidentally depend on a gateway performing mix forwarding.

### Decision 4: Gateway client sessions are established over `ws://<announced-ip>` only

**Choice.** The agent connects to `ws://{announced_ip}:{clients_ws_port}`, constructed directly from the refresher's stored data. Announced hostnames and wss entries are ignored. Testing the wss ingress is a future test kind.

**Why.** Three properties fall out at once: the source IP the gateway observes is genuinely the agent's, so the monitor-session identification of Decision 5 is sound by construction rather than dependent on the operator's ingress topology; there is no DNS dependency, matching the minimal-hop design of every other probe; and there is no certificate handling. Authenticity is unaffected, because the registration handshake authenticates the gateway's ed25519 identity against the identity key the orchestrator holds from the mixnet contract. TLS would only protect a transport that the gateway protocol already protects with a handshake-derived shared key.

**Alternative considered.** Reusing `NymNode::ws_entry_address()`. Rejected: it returns `wss://{hostname}` whenever the gateway announces one, and even its no-TLS variant prefers a hostname over an IP. Using it would reintroduce both DNS and TLS, and would put an operator-owned reverse proxy in the middle of a measurement whose scoring assumes the observed source IP is the agent's.

**Consequence.** This is a deliberate divergence from v1, which does probe the wss path when announced. A gateway with a broken TLS ingress therefore scores well under v3 liveness and badly under v1, systematically. Accepted, and made legible by bucketing the divergence gauge on whether the gateway announces a wss entry (Decision 12). In the entry model `clients_ws_port` is mandatory while `clients_wss_port` is optional, so plain ws is the base case and this leaves no coverage hole at the announcement level.

### Decision 5: The node delivers monitor final-hop packets in-session or drops them, and treats a monitor client session as ephemeral and unmetered

**Choice.** Two changes in nym-node, both keyed on the already-propagated authorised-agent set:

1. A final-hop packet from an authorised monitor IP is processed rather than dropped, delivered to a live client session if there is one, and **never** written to disk storage.
2. A client websocket session from an authorised monitor IP is unmetered and persists nothing: no shared-keys row, no bandwidth row, no inbox.

**Why.** (1) is the enabling change for gateway egress testing, which is refused outright today. Suppressing the disk fallback is not an optimisation but a correctness and hygiene requirement: it gives the agent exactly the semantics it wants (a packet that did not arrive on the socket was not delivered, full stop), and it stops monitor traffic from accruing undeliverable rows on every gateway in the network at liveness cadence. (2) removes the need for the agent to hold ecash ticketbooks, which for a one-shot process would mean an external credential-provisioning pipeline for a probe that transfers a few kilobytes.

**Alternative considered.** Real ticketbooks for agents, needing no node change. Rejected as disproportionate. Also considered: unmetered but still persisted, which is a smaller change but leaves a row per (agent, gateway) pair and keeps the inbox path live.

**Consequence.** An authorised agent IP gains free unmetered gateway transit. Accepted, because an IP that is compromised can be de-authorised by the orchestrator, and because monitor-sourced packets are already confined to monitor destinations by the existing routing filter. Deliberately NOT hardened further (Decision 10). The `client_id` that the authenticated handler threads into `BandwidthStorageManager` is storage-assigned, so a non-persisting session needs that identity to become optional; this is the seam the change touches, not merely a skipped check.

### Decision 6: An agent tests a wave of targets concurrently, and one wave is one concurrent batch

**Choice.** An assignment hands the agent N targets which it establishes and probes concurrently. The lease stamped on the in-progress rows is `now + kind_budget + slack`, independent of N, because the agent runs the whole wave at once rather than in sub-waves. Aggregate send rate is configured as a total budget with the per-target rate derived from it, never the other way round.

**Why.** Concurrency turns the lease bound from the sum of per-target worst cases into the maximum of them, which is what makes a full-network sweep at v1's cadence possible and what makes batch assignment safe at all. Sequentially, a 50-target batch of unresponsive nodes runs past any reasonable timeout, the orchestrator evicts the leases while the agent is still working, and the nodes get reassigned to a second agent, producing exactly the concurrent double-measurement the per-node mutex exists to prevent. Defining a wave as one concurrent batch keeps the orchestrator from having to know the agent's concurrency in order to size the lease.

**Alternative considered.** One target per request in a loop until the assignment comes back empty. It needs no lease arithmetic at all and bounds a crash to one node, at the cost of one round trip per target (a low single-digit percentage of a multi-second test). Rejected only because concurrency raises sweep throughput by an order of magnitude, which is the binding requirement; the loop remains the fallback if wave concurrency proves troublesome.

**Consequence.** The agent's tester is built around exactly one target today: one listener bound per run, a `NoiseNetworkView` holding one node's key, source-address acceptance gated on one node's addresses, and a packet processor built from one reusable header. A wave needs a shared listener, a Noise view holding every target's key, and per-target attribution. Attribution is by the source IP of the node's return connection, which is sound because node IPs are unique across the population; the agent's known-source set and Noise view for a wave are the union of every target's announced addresses, and that union is collision-free for the same reason. Measurements within a wave share the agent's NIC, CPU and scheduler, so latency figures from a wave are not comparable with stress-test latency figures (Decision 11). The existing "10k packets per second" figure was measured as batched writes down one connection and does not transfer to many sockets each carrying a low rate; the aggregate budget must be measured for the fan-out shape.

### Decision 7: Per-kind staleness and rotation, retained per-node mutex, leases materialised on the row

**Choice.** A new `node_test_state (node_id, test_kind)` table holds `last_tested_at`, `last_testrun_id` and `last_tested_ip`, replacing `nym_node.last_testrun` and `nym_node.last_tested_ip`. `testrun_in_progress` keeps its `node_id` primary key and gains `expires_at` plus a `test_kind` column for observability. Eviction becomes `DELETE FROM testrun_in_progress WHERE expires_at < ?`. Liveness eligibility additionally requires that the node's stress-kind `last_tested_at` is older than a cooldown.

**Why.** Per-kind state is what stops a 15-minute liveness cadence and a 2-hour stress cadence from fighting over one staleness pointer and one rotation cursor. Materialising the deadline on the row means the eviction sweep never needs to learn about kinds, so a future expensive kind that runs for minutes needs no eviction change; today's sweep takes a single cutoff derived from a global `test_timeout` and cannot express two budgets. Keeping the in-progress key on `node_id` alone gives the "one test at a time per node, across kinds" property for free. The cooldown covers the case the mutex cannot: a liveness test handed out the instant a stress test's row clears measures a node whose queues are still draining.

**Alternative considered.** A per-kind in-progress table. Rejected: it would permit simultaneous stress and liveness measurement of one node, which biases both.

**Consequence.** Denormalising `last_tested_at` also fixes an existing defect. Today staleness is read through `JOIN testrun tr ON tr.id = n.last_testrun` with `ON DELETE SET NULL`, so when eviction removes a node's last run the node reads as never-tested and jumps the queue.

### Decision 8: Results carry per-signal rows under a run-level row

**Choice.** `testrun` keeps run-level facts (kind, node, tested address, timing, error) and a new `testrun_signal (testrun_id, signal)` child table carries the counts and latency distributions per measured signal. A mixnode liveness or stress run has one signal; a gateway liveness run has two (`gw_ingress`, `gw_egress`). The score reported downstream is the average over the kind's fixed signal set.

**Why.** Downstream consumers want one number per node, but an operator needs to tell "perfect ingress, dead egress" from "uniformly half-lossy", and averaging destroys that distinction. This is the same argument that put the tested address on each result during the dual-stack work: without it a per-address failure is indistinguishable from a dead node. A child table also stops `testrun` from growing a column group per future test kind.

**Alternative considered.** Nullable per-direction columns on `testrun`. Simpler to query, worse to extend, and the proposal explicitly anticipates further test kinds.

### Decision 9: Per-kind submission streams

**Choice.** Each test kind has its own submission watermark and its own nym-api endpoint, and each endpoint keeps its own per-signer replay high-water mark.

**Why.** The existing single `metadata.last_submitted_testrun_id` cannot serve two destinations; the first liveness submission would drag the stress watermark past unsubmitted rows. Separate per-signer high-water marks are equally load-bearing: nym-api rejects any batch whose timestamp is not strictly greater than the signer's last accepted one, so two interleaved streams signed by the same orchestrator identity and validated against one shared map would reject each other indefinitely.

**Consequence.** The liveness ingest path repeats the staleness, membership, monotonicity and signature checks rather than sharing the stress path's state.

### Decision 10: No further hardening of the IP-keyed monitor trust

**Choice.** Monitor identification, for both the replay bypass and the new client-session exemption, stays keyed on the authorised agent's source IP. No same-IP forwarding restriction is added.

**Why.** An IP that is compromised can be de-authorised by the orchestrator, which bounds the exposure without new mechanism, and monitor-sourced packets are already confined to monitor destinations by the existing routing filter. Moving both gates onto the Noise-authenticated x25519 static key is already an identified follow-on and is the right place for this hardening, since it fixes the mixnet gate and the session gate in one coherent step.

**Consequence.** Keeps `agent1 -> node -> agent2` possible, which is the only way to attribute a mixnode's loss to its inbound versus outbound direction, and which a same-IP restriction would have foreclosed.

### Decision 11: Liveness scores delivery ratio only; latency is recorded, not scored

**Choice.** The liveness score is the delivery ratio averaged over the kind's signals. The full RTT distribution keeps being recorded and submitted, but carries no weight.

**Why.** Two confounds make a latency-derived score untrustworthy today. Nodes defer replay checking in batches bounded by `maximum_replay_detection_deferral` (50ms) and `maximum_replay_detection_pending_packets` (100), and only defer when the bloomfilter lock is contended, so a low-volume probe measures a busy node as slower than an idle one in a step function, penalising exactly the nodes that are carrying traffic. And measurements inside a wave include the agent's own queueing. Recording the distribution first means the weighting decision can be made against real data.

**Consequence.** v1's absence of any latency signal is preserved for now, so the migration changes attribution without also changing what the score means.

### Decision 12: Liveness ships as a third performance component at weight zero, with a divergence gauge

**Choice.** Liveness enters node performance as a component alongside the v1 routing score and the v3 stress score, initially weighted zero, mirroring the existing `use_stress_testing_data` / `minimum_available_*` / `*_score_weight` gating trio. Alongside it, a gauge reports per-node divergence between v3 liveness and v1 routing, bucketed by whether the node announces a wss entry.

**Why.** Two independent populations will score zero on liveness for reasons unrelated to their forwarding: nodes that have not ingested their agents' authorisations, and gateways not yet carrying the Decision 5 node changes. Shipping at weight zero makes both harmless, and the divergence gauge turns the eventual cutover into a measurement rather than a judgement call. Bucketing on the wss announcement separates the divergence this design knowingly introduces (Decision 4) from the divergence that indicates a real problem.

**Consequence.** A liveness failure whose cause is a rejected Noise handshake still scores zero, because a node that will not accept a connection is not routable and "unmeasurable" must never score better than "measurably broken". During shadow mode the distinct failure outcomes are recorded so that the divergence population can be explained rather than merely counted.

### Decision 13: The agent's client identity is derived from its noise key rather than provisioned

**Choice.** If the fully ephemeral session of Decision 5 lands, the agent generates a random ed25519 client identity per test. If it does not, the identity is derived deterministically from the agent's existing x25519 noise private key via HKDF with a domain-separation label, and used as the ed25519 seed directly.

**Why.** Neither option adds on-disk key material to provision, and a stable derived identity keeps registration rows bounded at one per (agent, gateway) pair if persistence cannot be avoided. A labelled KDF rather than seeding a CSPRNG with the raw private key gives domain separation, so any future derivation from the same secret stays independent; this is key separation, which is what HKDF is for, not key reuse.

**Consequence.** The noise key becomes a root secret, which is not an escalation because holding it already allows impersonating the agent to every node. Rotating the noise key rotates the client identity. The agent's client identity is permanently recognisable to a gateway, which costs nothing: the agent's IP is published on-chain, so liveness traffic is not covert regardless.

## Risks / Trade-offs

- **A node that has not ingested its agent authorisations scores zero on liveness.** This is the migration's central risk, because unlike v1 the probe requires the tested node to have explicitly allowlisted the prober. → Ship at weight zero with a divergence gauge (Decision 12), and treat the already-identified node-side periodic reconciliation follow-on as a prerequisite for the cutover rather than as optional hardening.
- **Liveness traffic is not covert at the mixnode, where v1's was.** A dishonest node can forward monitor traffic perfectly and drop everything else, because the authorised agent set is public on-chain. → Accepted deliberately: the current risk is negligible and the route-conflation bias being fixed is real and observed. The mitigation, if it is ever needed, is many agents across many providers, or an occasional multi-hop client-path run reusing the gateway-client capability this change introduces.
- **Un-upgraded gateways score zero on the egress phase**, since the final-hop drop is the current behaviour. → Weight zero until the fleet has upgraded; the divergence gauge measures when that is true.
- **A gateway with a broken wss ingress scores well on liveness and badly under v1.** → Known and intended (Decision 4); made legible by the wss bucket in the divergence gauge, and closed later by a dedicated wss test kind.
- **A gateway whose ws listener is not dual-stack scores zero on its ipv6 rotation.** → Reported as a genuine finding rather than suppressed, since it means the gateway does not serve ipv6 clients, but it will present as a population of half-scoring gateways on day one and should be expected.
- **Free unmetered gateway transit for an authorised agent IP.** → Bounded by revocation and by the existing monitor-to-monitor routing confinement (Decision 10).
- **Wave concurrency shares the agent's link across targets**, so one saturated target's back-pressure and the agent's own scheduling contaminate its neighbours' latency figures. → Aggregate rate budget rather than per-target times width, and latency excluded from scoring (Decision 11).
- **A crashed agent locks a whole wave** until its lease expires. → Leases are per-kind and sized to a single concurrent wave, so the exposure is tens of seconds rather than the current global five-minute timeout; results submitted per target release their own locks as the wave progresses.
- **The gateway session exemption assumes no proxy in front of the plain ws port.** → Made structural by connecting to `ws://ip` (Decision 4); an out-of-bandwidth failure must be recorded as a distinguishable outcome so a proxied gateway is diagnosable rather than silently scored zero.

## Migration Plan

1. Land the orchestrator schema migration and per-kind scheduling with liveness assignment disabled, so the stress test keeps running on the new tables. The migration moves `nym_node.last_testrun` and `last_tested_ip` into `node_test_state` under the stress kind, and backfills `expires_at` on any live in-progress rows.
2. Land the nym-node changes (final-hop delivery policy, ephemeral unmetered monitor session) and let them propagate through the fleet. They are inert until an agent exercises them.
3. Land the agent's liveness profile and wave concurrency, and enable mixnode liveness. This needs no gateway-side change and validates the wave machinery on the larger population.
4. Enable gateway liveness once enough of the fleet carries step 2.
5. Land nym-api ingest and the shadow-weighted component at weight zero, with the divergence gauge.
6. Decide the weighting, and separately the v1 cutover, from the divergence data. Both are later changes.

Rollback at any step is a config change rather than a revert: liveness assignment can be disabled in the orchestrator, and the component weight is already zero. The nym-node changes are the only step that is not trivially reversible in a running fleet, which is why they carry no behaviour for anyone but authorised monitors.

## Open Questions

1. **Liveness packet count and per-target rate.** These set the wave arithmetic and determine whether the send batches stay large enough to be efficient across many sockets. Needs measurement of the fan-out shape rather than a guess (Decision 6).
2. **May a single agent invocation mix kinds?** With the orchestrator choosing, a process could be handed a 1000pps stress test and then liveness waves while its own link is still recovering. Keeping an invocation to one kind is a small stickiness rule if wanted.
3. **Does a wss-configured gateway still bind its plain ws port in practice?** The entry model implies yes (`clients_ws_port` is mandatory, `clients_wss_port` optional), but this should be confirmed against a live node before relying on it for coverage.
4. **How ephemeral can the monitor session actually be?** Whether `insert_shared_keys` and `create_bandwidth_entry` can both be skipped cleanly decides whether the agent's client identity is random per test or derived (Decision 13).
