## Context

Network monitor v3 today performs exactly one kind of measurement: a high-rate two-hop stress test against a single mixnode, routed `[tested_node, agent]` so the node relays each packet straight back. The orchestrator assigns work lazily from a staleness-ordered node table guarded by a `testrun_in_progress` lock set, agents are one-shot jobs that test exactly one node and exit, and results are submitted to nym-api in signed monotonic batches that feed the stress component of node performance.

Liveness scoring meanwhile lives in the nym-api-internal v1 monitor, which measures delivery over five-node routes and attributes the result to one substituted node. Its bias is documented in its own design record and is the motivation for this change (see proposal.md).

The relevant constraints are:

- **The `network-monitors` contract cannot be redeployed, but it CAN be migrated.** It carries a `migrate` entry point guarded by cw2 `ensure_from_older_version`, and its admin is the Nymtech SA multisig, so an additive change is a governance action rather than a redeploy. The binding constraint is the third-party node fleet, and it is a constraint on the SHAPE of the change rather than on whether one happens: `cw_serde` does not set `deny_unknown_fields`, so an un-upgraded node silently ignores an unknown field on a message or query response whose variant it already knows, whereas a NEW or RETYPED `ExecuteMsg` variant fails to deserialise and the node's event handler logs and continues, which means it silently stops learning about agents. Additive optional fields are therefore fleet-safe; new or retyped variants are not, and this change uses only the former (Decision 14).
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
- Changing the source-IP basis of the node-side MIXNET gates (Noise responder, routing filter, replay bypass). Moving those onto the Noise-authenticated x25519 static key remains the separately-identified follow-on. This change re-keys only the new client-session gate, which no Noise handshake can ever cover (Decision 14).
- Removing agent IP addresses from the contract. Considered and rejected in Decision 14, because the node must know an agent's address to route the mixnode probe's return hop at all.
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

The run produces two measurements. The score denominator is fixed by the kind at two measurements, so a phase that produces nothing scores zero rather than being dropped from the average. A phase-1 failure MUST NOT abort the run. Only failure to establish the session aborts, in which case both measurements are zero.

**Why.** The two phases test independent capabilities and a gateway needs both to be useful, so both must always be measured and a missing measurement must never be more favourable than a zero one. This mirrors v1, which seeds every tested node at zero received so that an unreachable node scores 0 rather than being omitted. Sharing one session is required anyway, because final-hop delivery needs a live session at the moment the packet arrives.

**Alternative considered.** A single combined loop (`client -> GW -> agent-as-mix -> GW -> client`) exercising both directions per packet. Rejected because it halves the packet budget but destroys direction attribution, which is the entire reason for going minimal-hop. Also considered and rejected: separate assignments per phase, which would allow two agents to measure half a gateway each and produce results nobody can compose.

**Consequence.** An agent that dies after phase 1 abandons the whole run rather than resuming it. Phase 1 involves no sphinx processing at the gateway at all (the client supplies an explicit next hop and the gateway forwards verbatim), so a phase-1 failure implicates the session, the bandwidth path, or the outbound forwarder, never the sphinx layer. Neither phase exercises `forward_hop_processing_enabled`, which is derived from `modes.mixnode` and is off on a pure gateway, so the tests can never accidentally depend on a gateway performing mix forwarding.

### Decision 4: Gateway client sessions are established over `ws://<announced-ip>` only

**Choice.** The agent connects to `ws://{announced_ip}:{clients_ws_port}`, constructed directly from the refresher's stored data. Announced hostnames and wss entries are ignored. Testing the wss ingress is a future test kind.

**Why.** Three properties fall out at once: the source IP the gateway observes is genuinely the agent's, so the monitor-session identification of Decision 5 is sound by construction rather than dependent on the operator's ingress topology; there is no DNS dependency, matching the minimal-hop design of every other probe; and there is no certificate handling. Authenticity is unaffected, because the registration handshake authenticates the gateway's ed25519 identity against the identity key the orchestrator holds from the mixnet contract. TLS would only protect a transport that the gateway protocol already protects with a handshake-derived shared key.

**Alternative considered.** Reusing `NymNode::ws_entry_address()`. Rejected: it returns `wss://{hostname}` whenever the gateway announces one, and even its no-TLS variant prefers a hostname over an IP. Using it would reintroduce both DNS and TLS, and would put an operator-owned reverse proxy in the middle of a measurement whose scoring assumes the observed source IP is the agent's.

**Confirmed since.** nym-node binds NO TLS listener at all: the clients websocket is started unconditionally in entry mode at `gateway_tasks.ws_bind_address`, and `announce_wss_port` is only an announcement field. So an announced wss entry always describes an externally terminated proxy, and the plain ws port is the node's own listener. `ws://ip` is therefore not merely the simpler target, it is the only way to measure the node rather than an operator's proxy.

**Consequence.** This is a deliberate divergence from v1, which does probe the wss path when announced. A gateway with a broken TLS ingress therefore scores well under v3 liveness and badly under v1, systematically. Accepted, and made legible by bucketing the divergence gauge on whether the gateway announces a wss entry (Decision 12). In the entry model `clients_ws_port` is mandatory while `clients_wss_port` is optional, so plain ws is the base case and this leaves no coverage hole at the announcement level.

### Decision 5: The node delivers monitor final-hop packets in-session or drops them, and treats a monitor client session as ephemeral and unmetered

**Choice.** Two changes in nym-node, both driven by the already-propagated authorised-agent set, but identified differently because they sit on different protocols:

1. A final-hop packet from an authorised monitor IP is processed rather than dropped, delivered to a live client session if there is one, and **never** written to disk storage. This stays source-IP keyed, like every other gate on the mixnet listener.
2. A client websocket session whose registration handshake authenticates an ed25519 identity announced on-chain for an authorised monitor is unmetered and persists nothing: no shared-keys row, no bandwidth row, no inbox. This is keyed on the verified identity, NOT on the source IP (Decision 14).

**Why.** (1) is the enabling change for gateway egress testing, which is refused outright today. Suppressing the disk fallback is not an optimisation but a correctness and hygiene requirement: it gives the agent exactly the semantics it wants (a packet that did not arrive on the socket was not delivered, full stop), and it stops monitor traffic from accruing undeliverable rows on every gateway in the network at liveness cadence. (2) removes the need for the agent to hold ecash ticketbooks, which for a one-shot process would mean an external credential-provisioning pipeline for a probe that transfers a few kilobytes.

**Alternative considered.** Real ticketbooks for agents, needing no node change. Rejected as disproportionate. Also considered: unmetered but still persisted, which is a smaller change but leaves a row per (agent, gateway) pair and keeps the inbox path live.

**Confirmed since.** The fully non-persisting variant is reachable. `register_client` is the whole persistence surface of registration: it calls `insert_shared_keys` for the storage id, then `get_available_bandwidth` and `create_bandwidth_entry` with it. `BandwidthStorageManager` uses that id only inside storage calls, so with storage access disabled it is unused and can become optional. And the inbox push is keyed on the client ADDRESS rather than the id, so with monitor final-hop packets never stored it is a no-op query even if left in place; skipping it is hygiene rather than correctness.

**Consequence.** An authorised monitor gains free unmetered gateway transit, and that transit is NOT confined to monitor destinations. The routing filter's "a monitor may only send to a monitor" rule applies to packets that arrive on the mixnet listener, where the monitor flag is set from the connection's source IP; a packet handed to the gateway over a client session is forwarded with that flag unset, so it routes anywhere an ordinary client's packet may go. The exemption is therefore an unconfined grant, which is precisely why it keys on a verified identity rather than on a source IP that k8s host ports share and CNI pools recycle (Decision 14). The `client_id` that the authenticated handler threads into `BandwidthStorageManager` is storage-assigned, so a non-persisting session needs that identity to become optional; this is the seam the change touches, not merely a skipped check.

### Decision 6: An agent tests a wave of targets concurrently, and one wave is one concurrent batch

**Choice.** An assignment hands the agent N targets which it establishes and probes concurrently. The lease stamped on the in-progress rows is `now + kind_budget + slack`, independent of N, because the agent runs the whole wave at once rather than in sub-waves. Aggregate send rate is configured as a total budget with the per-target rate derived from it, never the other way round.

**Why.** Concurrency turns the lease bound from the sum of per-target worst cases into the maximum of them, which is what makes a full-network sweep at v1's cadence possible and what makes batch assignment safe at all. Sequentially, a 50-target batch of unresponsive nodes runs past any reasonable timeout, the orchestrator evicts the leases while the agent is still working, and the nodes get reassigned to a second agent, producing exactly the concurrent double-measurement the per-node mutex exists to prevent. Defining a wave as one concurrent batch keeps the orchestrator from having to know the agent's concurrency in order to size the lease.

**Alternative considered.** One target per request in a loop until the assignment comes back empty. It needs no lease arithmetic at all and bounds a crash to one node, at the cost of one round trip per target (a low single-digit percentage of a multi-second test). Rejected only because concurrency raises sweep throughput by an order of magnitude, which is the binding requirement; the loop remains the fallback if wave concurrency proves troublesome.

**Consequence.** The agent's tester is built around exactly one target today: one listener bound per run, a `NoiseNetworkView` holding one node's key, source-address acceptance gated on one node's addresses, and a packet processor built from one reusable header. A wave needs a shared listener, a Noise view holding every target's key, and per-target attribution. Attribution is by the source IP of the node's return connection, which is sound because node IPs are unique across the population; the agent's known-source set and Noise view for a wave are the union of every target's announced addresses, and that union is collision-free for the same reason. Measurements within a wave share the agent's NIC, CPU and scheduler, so latency figures from a wave are not comparable with stress-test latency figures (Decision 11). The existing "10k packets per second" figure was measured as batched writes down one connection and does not transfer to many sockets each carrying a low rate. Rather than block the work on measuring the fan-out shape, the aggregate budget is a configured knob carrying a provisional default, so an agent host that cannot sustain it is a configuration change rather than a code change.

### Decision 7: Per-kind staleness and rotation, retained per-node mutex, leases materialised on the row

**Choice.** A new `node_test_state (node_id, test_kind, tested_role)` table holds `last_tested_at`, `last_testrun_id` and `last_tested_ip`, replacing `nym_node.last_testrun` and `nym_node.last_tested_ip`. `testrun_in_progress` keeps its `node_id` primary key and gains `expires_at` plus `test_kind` and `tested_role` columns. Eviction becomes `DELETE FROM testrun_in_progress WHERE expires_at < ?`. Liveness eligibility additionally requires that the node's `(stress, mixnode)` `last_tested_at` is older than a cooldown.

**Why.** Per-kind state is what stops a 15-minute liveness cadence and a 2-hour stress cadence from fighting over one staleness pointer and one rotation cursor. The key carries the ROLE as well as the kind because the two liveness probes are different measurements of the same node: a `mixnode_and_gateway` node must be eligible for both, and a two-part key would let its mixnode-liveness run advance the very timestamp that gates its gateway-liveness eligibility, so it would alternate roles across cycles instead of being measured in both. Materialising the deadline on the row means the eviction sweep never needs to learn about kinds, so a future expensive kind that runs for minutes needs no eviction change; today's sweep takes a single cutoff derived from a global `test_timeout` and cannot express two budgets. Keeping the in-progress key on `node_id` alone gives the "one test at a time per node, across kinds AND roles" property for free. The cooldown covers the case the mutex cannot: a liveness test handed out the instant a stress test's row clears measures a node whose queues are still draining.

`tested_role` on `testrun_in_progress` is not observability, it is the authoritative source of the role when the result comes back. `testrun` records `tested_role`, and the submission carries only the node and the address, so without it the orchestrator would have to trust the agent's echo for a field it assigned itself.

**Alternative considered.** A per-kind in-progress table. Rejected: it would permit simultaneous stress and liveness measurement of one node, which biases both.

The wss announcement is deliberately NOT stored alongside `clients_ws_port`, even though the refresher sees it: the only consumer is the divergence gauge's bucketing in Decision 12, which runs in nym-api and can read the same self-described `mixnet_websockets` interface from that side's described-nodes cache. Storing it here would be a second copy of a fact no orchestrator path reads, since the probe ignores wss entries by construction and the submission carries no such field.

**Consequence.** Denormalising `last_tested_at` also fixes an existing defect. Today staleness is read through `JOIN testrun tr ON tr.id = n.last_testrun` with `ON DELETE SET NULL`, so when eviction removes a node's last run the node reads as never-tested and jumps the queue.

### Decision 8: Results carry per-interface measurement rows under a run-level row

**Choice.** `testrun` keeps run-level facts (kind, node, tested address, timing, error) and a new `testrun_measurement (testrun_id, interface)` child table carries the counts and latency distributions per interface the run exercised. A mixnode liveness or stress run exercises one interface (`mix_forwarding`); a gateway liveness run exercises two (`client_ingest`, `client_delivery`). The score reported downstream is the average over the kind's fixed measurement set.

The discriminator names the node FUNCTION exercised, not a route: every value traverses the mixnet, so a route-shaped name would not distinguish them. The test kind never appears in it, because the kind is a property of the run and already sits on the parent row - encoding it twice would make a row whose kind and interface disagree representable.

**Why.** Downstream consumers want one number per node, but an operator needs to tell "perfect ingress, dead egress" from "uniformly half-lossy", and averaging destroys that distinction. This is the same argument that put the tested address on each result during the dual-stack work: without it a per-address failure is indistinguishable from a dead node. A child table also stops `testrun` from growing a column group per future test kind.

**Alternative considered.** Nullable per-direction columns on `testrun`. Simpler to query, worse to extend, and the proposal explicitly anticipates further test kinds.

### Decision 9: Per-kind submission streams

**Choice.** Each test kind has its own submission watermark and its own nym-api endpoint, and each endpoint keeps its own per-signer replay high-water mark.

**Why.** The existing single `metadata.last_submitted_testrun_id` cannot serve two destinations; the first liveness submission would drag the stress watermark past unsubmitted rows. Separate per-signer high-water marks are equally load-bearing: nym-api rejects any batch whose timestamp is not strictly greater than the signer's last accepted one, so two interleaved streams signed by the same orchestrator identity and validated against one shared map would reject each other indefinitely.

**Consequence.** The liveness ingest path repeats the staleness, membership, monotonicity and signature checks rather than sharing the stress path's state.

### Decision 10: No further hardening of the IP-keyed monitor trust on the mixnet listener

**Choice.** Monitor identification on the MIXNET listener, meaning the Noise responder gate, the routing filter and the replay bypass, stays keyed on the authorised agent's source IP. No same-IP forwarding restriction is added. The client-session exemption is the exception and is keyed on a verified identity instead (Decision 14).

**Why.** An IP that is compromised can be de-authorised by the orchestrator, which bounds the exposure without new mechanism, and monitor-sourced packets on that listener are confined to monitor destinations by the existing routing filter. Moving the mixnet gates onto the Noise-authenticated x25519 static key is already an identified follow-on and remains the right place for that hardening. It cannot, however, be the answer for the client-session gate: `upgrade_noise_responder` is invoked only from the mixnet connection handler, so the client websocket port runs no Noise handshake and has no static key to authenticate. A gate placed there can never be fixed by the Noise follow-on, which is what forces Decision 14 rather than leaving it as future work.

**Consequence.** Keeps `agent1 -> node -> agent2` possible, which is the only way to attribute a mixnode's loss to its inbound versus outbound direction, and which a same-IP restriction would have foreclosed.

### Decision 11: Liveness scores delivery ratio only; latency is recorded, not scored

**Choice.** The liveness score is the delivery ratio averaged over the kind's measurements. The full RTT distribution keeps being recorded per interface and exposed on the orchestrator's own read surface, but carries no weight. It is NOT submitted to nym-api: the stress stream never sent latency either, and a submission shape carrying figures nothing reads would have to be versioned before it could be trusted. Adding it is an additive field on the batch content if a latency-weighted score is ever wanted.

**Why.** Two confounds make a latency-derived score untrustworthy today. Nodes defer replay checking in batches bounded by `maximum_replay_detection_deferral` (50ms) and `maximum_replay_detection_pending_packets` (100), and only defer when the bloomfilter lock is contended, so a low-volume probe measures a busy node as slower than an idle one in a step function, penalising exactly the nodes that are carrying traffic. And measurements inside a wave include the agent's own queueing. Recording the distribution first means the weighting decision can be made against real data.

**Consequence.** v1's absence of any latency input to the score is preserved for now, so the migration changes attribution without also changing what the score means.

### Decision 12: Liveness ships as a third performance component at weight zero, with a divergence gauge

**Choice.** Liveness enters node performance as a component alongside the v1 routing score and the v3 stress score, initially weighted zero, mirroring the existing `use_stress_testing_data` / `minimum_available_*` / `*_score_weight` gating trio. Alongside it, a gauge reports per-node divergence between v3 liveness and v1 routing, bucketed by whether the node announces a wss entry.

**Why.** Two independent populations will score zero on liveness for reasons unrelated to their forwarding: nodes that have not ingested their agents' authorisations, and gateways not yet carrying the Decision 5 node changes. Shipping at weight zero makes both harmless, and the divergence gauge turns the eventual cutover into a measurement rather than a judgement call. Bucketing on the wss announcement separates the divergence this design knowingly introduces (Decision 4) from the divergence that indicates a real problem.

**Consequence.** A liveness failure whose cause is a rejected Noise handshake still scores zero, because a node that will not accept a connection is not routable and "unmeasurable" must never score better than "measurably broken". During shadow mode the distinct failure outcomes are recorded so that the divergence population can be explained rather than merely counted.

### Decision 13: The agent's client identity is derived from its noise key rather than provisioned

**Choice.** The agent derives its ed25519 client identity deterministically from its existing x25519 noise private key via HKDF with a domain-separation label, using the output directly as the ed25519 seed. The random-per-test alternative is dropped, because Decision 14 requires the identity to be announced before it is used.

**Why.** This adds no on-disk key material to provision and no operator step, which was the original point, and a stable identity is what makes the on-chain announcement possible at all. A labelled KDF rather than seeding a CSPRNG with the raw private key gives domain separation, so any future derivation from the same secret stays independent; this is key separation, which is what HKDF is for, not key reuse. As a side benefit, a stable identity keeps registration rows bounded at one per (agent, gateway) pair should the fully non-persisting session of Decision 5 turn out not to be reachable.

**Consequence.** The noise key becomes a root secret, which is not an escalation because holding it already allows impersonating the agent to every node. Rotating the noise key rotates the client identity, which now also means re-announcing it. The agent's client identity is permanently recognisable to a gateway, which costs nothing: the agent's addresses are published on-chain, so liveness traffic is not covert regardless.

### Decision 14: The agent's ed25519 client identity is announced on-chain, and the gateway session exemption keys on it

**Choice.** The `network-monitors` contract is migrated to carry an OPTIONAL base58 ed25519 identity key on each agent entry, supplied by the orchestrator on `AuthoriseNetworkMonitor` and announced to the orchestrator by the agent alongside its address pair and noise key. A gateway grants the ephemeral unmetered session of Decision 5 only to a client whose registration handshake authenticates an identity present in that on-chain set. The source IP plays no part in this gate.

**Why.** The gateway registration handshake is already mutually authenticating: the gateway takes the client's ed25519 identity from the initialisation message and, at step 5, decrypts and verifies the client's signature over both ephemeral DH public keys against it. By the time a session exists the gateway therefore holds a possession-proven client identity, so announcing that identity turns the exemption into a set-membership test on a key the gateway has already verified. No new protocol message, no challenge to design, no client protocol version negotiation. Against that, the alternative gate is a source IP that several agents share through distinct k8s host ports and that CNI pools recycle, guarding an exemption which grants unconfined unmetered mixnet transit (Decision 5). The gate that most needs a real identity is the one where an identity is already sitting unused.

The migration is fleet-safe in the exact sense the Context constraint requires. An additive field on the existing `AuthoriseNetworkMonitor` variant and on the stored entry is silently ignored by un-upgraded nodes, unlike a new variant. Making it `Option` means no data migration and no backfill logic: the contract's agent save is an upsert and agents re-announce before every run, so every live agent's entry acquires its identity within one liveness cycle. And the consumer that needs the field is a gateway that must be upgraded for Decision 5 regardless, so the change adds a governance action rather than a second fleet rollout, which is what distinguishes this from the earlier decision to defer contract work.

**Alternative considered.** An orchestrator-signed capability token presented at registration, verified against the orchestrator identity key that the contract already stores, needing no contract change at all. Genuinely attractive on revocation, where a short-lived scoped token beats an on-chain entry that only a multisig transaction can remove. Rejected on total cost: it needs a slot in the versioned client control protocol and a new orchestrator-identity cache on every node, where the on-chain identity needs neither, because the handshake already carries and proves the key. Worth revisiting if the client protocol is being changed for other reasons.

**Alternative considered.** Going further and REMOVING agent IP addresses from the contract, so the monitor fleet is not publicly enumerable. The Noise pattern permits it: `Noise_XKpsk3` transmits the initiator's static key in message 3 with `se` proving possession, and the PSK is derived from the responder's own public key, so a node needs no advance knowledge of an initiator to complete a handshake and learn its identity. Rejected for three reasons. The routing filter's destination set is `HashSet<IpAddr>` and the mixnode probe's return hop is a fresh outbound connection, so a node that does not know the agent's address cannot send the probe back at all; replacing the set with a per-connection "may return to this peer" capability would work but forecloses the `agent1 -> node -> agent2` shape Decision 10 preserves. Dropping the address also means rekeying agent storage and retyping `RevokeNetworkMonitor`, which is the fleet-unsafe kind of change: an un-upgraded node would fail to parse revocations and keep a stale bypass indefinitely. And the benefit is convenience rather than secrecy, since any operator can read the monitor set out of their own node, and the retained `reuse_header` replay bypass forces the node to classify a connection as a monitor at packet time anyway. Real indistinguishability needs fresh headers, real ticketbooks and no special treatment, which is a different and much larger design.

**Alternative considered.** Since the message is being touched anyway, consolidating the agent's address pair into ONE `AuthoriseNetworkMonitor` carrying explicit ipv4 and ipv6 fields, instead of the two messages sent in one transaction today. Rejected, because the two-message shape is what carries dual-stack authorisation to un-upgraded nodes rather than a workaround for the frozen contract. A node dispatches `handle_msg` per message within a transaction, so an un-upgraded node already applies both authorisations independently without knowing that dual-stack exists. Renaming `mixnet_address` into a v4/v6 pair breaks deserialisation on those nodes, which log and continue and therefore silently stop learning about every agent; keeping `mixnet_address` and adding an optional v6 field parses, but an un-upgraded node then authorises the ipv4 address only, silently reintroducing the ipv6 rejection this subsystem already fixed. The consolidation also buys less than it appears: atomicity comes from the transaction, not from the message count, so the only saving is the `execute_multiple` call site.

The version that WOULD simplify meaningfully is one entry per agent keyed by the noise key and holding both addresses, which deletes the noise-key group-by in the orchestrator's cache rehydration, makes noise-key uniqueness enforceable, and turns an address change into an update instead of an insert-plus-orphan. That is a storage rekey plus a retyped `RevokeNetworkMonitor` plus a changed paged-query response, and since a failed startup load aborts node startup, an un-upgraded node would fail to start after such a migration. It therefore has to be staged across releases in the manner the contract capability's schema-evolution requirement describes, and it is a separate change that liveness does not need.

**Consequence.** The exemption survives a change of the agent's egress address, which matters because pods are rescheduled onto recycled addresses and the orchestrator still has no revocation path. A gateway that has not ingested an agent's identity meters that agent's session and the run scores zero on both phases, which is the same failure mode as a node that has not ingested an authorisation and is already covered by shipping at weight zero. The node gains a third derived structure alongside the routing set and the noise map, keyed by identity rather than by IP, populated from the same startup load and the same websocket events. Nothing else in the change consumes the field: the mixnet gates keep their IP keying (Decision 10), and assignment, scheduling and submission are unaffected.

## Risks / Trade-offs

- **A node that has not ingested its agent authorisations scores zero on liveness.** This is the migration's central risk, because unlike v1 the probe requires the tested node to have explicitly allowlisted the prober. → Ship at weight zero with a divergence gauge (Decision 12), and treat the already-identified node-side periodic reconciliation follow-on as a prerequisite for the cutover rather than as optional hardening.
- **Liveness traffic is not covert at the mixnode, where v1's was.** A dishonest node can forward monitor traffic perfectly and drop everything else, because the authorised agent set is public on-chain. → Accepted deliberately: the current risk is negligible and the route-conflation bias being fixed is real and observed. The mitigation, if it is ever needed, is many agents across many providers, or an occasional multi-hop client-path run reusing the gateway-client capability this change introduces.
- **Un-upgraded gateways score zero on the egress phase**, since the final-hop drop is the current behaviour. → Weight zero until the fleet has upgraded; the divergence gauge measures when that is true.
- **A gateway with a broken wss ingress scores well on liveness and badly under v1.** → Known and intended (Decision 4); made legible by the wss bucket in the divergence gauge, and closed later by a dedicated wss test kind.
- **A gateway whose ws listener is not dual-stack scores zero on its ipv6 rotation.** → Reported as a genuine finding rather than suppressed, since it means the gateway does not serve ipv6 clients, but it will present as a population of half-scoring gateways on day one and should be expected.
- **Free unmetered gateway transit for an authorised monitor, unconfined by the routing filter**, since a packet entering through a client session does not carry the monitor flag that restricts monitor traffic to monitor destinations. → Keyed on a handshake-verified on-chain ed25519 identity rather than on a shared or recycled source IP (Decision 14), and bounded by revocation.
- **A gateway that has not ingested an agent's announced identity meters that agent's session**, so the run scores zero on both phases rather than one. → Same failure mode and same mitigation as a missed authorisation event: weight zero plus the divergence gauge, with the periodic-reconciliation follow-on as a cutover prerequisite.
- **Wave concurrency shares the agent's link across targets**, so one saturated target's back-pressure and the agent's own scheduling contaminate its neighbours' latency figures. → Aggregate rate budget rather than per-target times width, and latency excluded from scoring (Decision 11).
- **A crashed agent locks a whole wave** until its lease expires. → Leases are per-kind and sized to a single concurrent wave, so the exposure is tens of seconds rather than the current global five-minute timeout; results submitted per target release their own locks as the wave progresses.
- **The gateway session exemption assumes no proxy in front of the plain ws port.** → Made structural by connecting to `ws://ip` (Decision 4); an out-of-bandwidth failure must be recorded as a distinguishable outcome so a proxied gateway is diagnosable rather than silently scored zero.

## Migration Plan

1. Migrate the contract to carry the optional agent ed25519 identity, and land the orchestrator and agent sides of the announcement so that live agents start populating the field through the existing upsert. This is inert for every consumer until step 3 reaches a gateway, and safe for un-upgraded nodes because the field is additive.
2. Land the orchestrator schema migration and per-kind scheduling with liveness assignment disabled, so the stress test keeps running on the new tables. The migration reshapes the work-tracking tables EMPTY rather than backfilling them, preserving only the node registry: local results are a retry buffer already submitted every `result_submission_interval`, and every in-flight lease is orphaned by the restart the deploy implies. The cost is one full-population sweep, because every node reads as never-tested.
3. Land the nym-node changes (final-hop delivery policy, ephemeral unmetered monitor session keyed on the announced identity) and let them propagate through the fleet. They are inert until an agent exercises them.
4. Land the agent's liveness profile and wave concurrency, and enable mixnode liveness. This needs no gateway-side change and validates the wave machinery on the larger population.
5. Enable gateway liveness once enough of the fleet carries step 3.
6. Land nym-api ingest and the shadow-weighted component at weight zero, with the divergence gauge.
7. Decide the weighting, and separately the v1 cutover, from the divergence data. Both are later changes.

Rollback at any step is a config change rather than a revert: liveness assignment can be disabled in the orchestrator, and the component weight is already zero. The contract migration is not reversible, but it is inert on its own: an announced identity that no gateway consults changes nothing. The nym-node changes are the only step that is not trivially reversible in a running fleet, which is why they carry no behaviour for anyone but authorised monitors.

## Resolved Questions

1. **Liveness packet count and per-target rate.** RESOLVED by decision rather than by measurement: the profile ships with provisional defaults chosen for score granularity, and every value is a configured knob. A per-target count of 100 gives 1% granularity and roughly 2.2% binomial noise at a true 95% delivery, against v1's three packets per route at 33% granularity, so it is a thirtyfold increase in evidence per node and a sensible floor. The aggregate rate budget starts at 500 packets/second, and the wave size is sized per role: 100 targets for the mixnode probe and 50 for the gateway probe. Splitting it follows from a wave being one concurrent batch, which makes the wave size the count an agent holds open at once: a gateway target costs a live client session measuring two interfaces, a mixnode target costs a Noise connection measuring one, and v1 already ran its whole gateway population through a 50-client window per cycle. The lease still does not scale with the wave, but it has to cover the slower of the two probes. Deliberately NOT measured first: the agent hosts are not the machines these numbers would be measured on, so an agent that cannot sustain the budget is a configuration change. The sweep arithmetic that the knobs must satisfy is `T_send = count x wave / aggregate_rate` and `population / wave <= invocations x (interval / T_wave)`.
2. **May a single agent invocation mix kinds?** RESOLVED, no, by construction: an invocation takes exactly ONE assignment, which is either a single stress target or a single liveness wave, then exits. No stickiness rule is needed. The residual concern is different from the one originally recorded: consecutive invocations on the same HOST, where a liveness wave measures an agent still recovering from a stress test and charges the loss to every node in the wave. Not addressed now, because 30000 packets at 1000pps is around 16 Mbps for a 2KB packet and saturates nothing on a container host, and sockets die with the process. If deployment ever shows contaminated waves, the remedy is a cooldown keyed on the agent's IP (not its socket address, since several agents share a host NIC behind distinct ports), symmetrical to the per-node `liveness_after_stress_cooldown`.
3. **Does a wss-configured gateway still bind its plain ws port?** RESOLVED, yes, and more strongly than the question assumed: nym-node binds no TLS listener at all, so an announced wss entry always denotes an externally terminated proxy. See Decision 4.
4. **How ephemeral can the monitor session actually be?** RESOLVED, fully non-persisting is reachable, with the storage-assigned client id becoming optional. See Decision 5.
5. **Does the gateway's session path have access to the derived monitor set?** RESOLVED, it needs its own handle rather than a read of the mixnet structures, because the gateway client handling lives in a different crate from the routing set and noise map. `CommonHandlerState` already carries live shared handles passed down from nym-node (`upgrade_mode`, `active_clients_store`), so the identity set becomes one more field on it plus a builder setter, populated from the same startup load and websocket events.
