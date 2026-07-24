## Context

The network-monitor v3 subsystem is Nym's active mixnode stress-tester and one of the inputs to node-performance scoring. It is a chain-authorised distributed system with four moving parts: the `network-monitors` CosmWasm contract (a three-tier authorisation registry), one or more orchestrator daemons (`nym-network-monitor-v3/nym-network-monitor-orchestrator`), a fleet of short-lived agents (`nym-network-monitor-v3/nym-network-monitor-agent`), and two reacting endpoints - nym-api (which ingests signed result batches) and every nym-node (which independently reads the contract to decide whose probe traffic to accept). It has shipped, is in active use for stress testing, and is documented only as source plus inline comments. This document captures the architectural choices behind the implementation as it stands today; no behaviour change is proposed.

The subsystem is deliberately separated from the two other "network monitor" systems: the nym-api INTERNAL v1 monitor (spec'd as `network-monitor`) and the standalone locust-driven v2 crate `nym-network-monitor/`, which is no longer in use (out of scope). Where v1 measures reliability by looping self-addressed packets over verified multi-hop routes inside nym-api, v3 stress-tests one mixnode at a time from an external, chain-authorised agent at high packet rate, and reports a delivery ratio plus latency statistics.

## Goals / Non-Goals

**Goals:**
- Capture the chain-backed three-tier authorisation model and, critically, how it propagates to nym-nodes and gates the sphinx replay/bloomfilter bypass, since that is the load-bearing trust mechanism of the whole subsystem.
- Capture what a stress-test score measures: a delivery ratio of high-rate, looped-back sphinx packets over a fixed window, with the sent count forced to the expected count so back-pressure is penalised.
- Document the orchestrator lifecycle: startup gating, the node registry sourced from the mixnet contract plus self-description, queue-less staleness-ordered assignment, and signed monotonic at-least-once submission to nym-api.
- Document the agent one-shot lifecycle and probe mechanics, including the `reuse_header` replay that exercises the node bypass.
- Document the nym-api ingest validation (staleness, contract membership, replay, signature) and the database-level dedupe.
- Record the configuration surface and defaults, since they change what a score means.
- Record the known limitations as deliberate current-state facts, not latent bugs, and catalog the downstream consumer surface.

**Non-Goals:**
- The internal behaviour of the downstream consumers of the score (performance aggregation, the performance provider, rewarding). These are separate capabilities; this document catalogs the consumer surface (Decision 13) so a score's blast radius is legible, but does not respecify those mechanisms.
- The internals of the mixnet contract, nym-node packet forwarding, sphinx, or the Noise transport beyond the surface the subsystem relies on.
- The v1 internal monitor and the v2 locust-driven monitor.
- Any redesign of the flagged limitations. These are acknowledged as future work or follow-on changes; the spec captures the current surface.

## Decisions

### Decision 1: Chain-backed three-tier authorisation, resolved node-locally

**Choice.** Authorisation lives on the `network-monitors` contract as three tiers: the admin (Nymtech SA multisig / governance) authorises orchestrators; each orchestrator authorises its own agents (socket address plus x25519 noise key). Every nym-node independently reads that agent set from the contract and decides, on its own, whose traffic to accept.

**Why.** The nodes being probed must be able to verify who is allowed to probe them WITHOUT trusting the orchestrator or nym-api directly. Putting the allow-set on-chain makes it a single shared source of truth that every node can read and every party can audit, and it lets authorisation be delegated (admin to orchestrator to agent) without a central server in the probe path. This is a decentralised replacement for the hardcoded allow-set used by the v1/v2 designs.

**Consequence.** Authorisation is only as live as each node's view of the chain (Decision 2). An agent must be on-chain before it can probe, and the orchestrator must hold a funded Nyx account to write those authorisations.

### Decision 2: Node-side propagation is a real-time event subscription that gates the replay bypass

**Choice.** A node loads the full authorised-agent set once at startup (a failed load aborts startup) and then keeps it current through a nyxd WEBSOCKET event subscription that dispatches `AuthoriseNetworkMonitor` / `RevokeNetworkMonitor` / `RevokeAllNetworkMonitors` contract events - not a periodic poll. The set feeds two shared, lock-free, canonical-IP-keyed structures (a routing set and a Noise-key map), which gate the Noise responder handshake, packet routing, and - the critical one - the sphinx replay/bloomfilter bypass: a replayed packet is dropped UNLESS its source IP belongs to an authorised agent.

**Why.** The agent's `reuse_header` probe deliberately replays one sphinx header, which any correct node would otherwise drop as a replay. Gating the bypass on the on-chain allow-set lets exactly the authorised monitors stress a node while keeping the bloomfilter's protection against everyone else. An event subscription (rather than a poll) makes newly-authorised agents usable within block-plus-websocket latency (seconds).

**Alternative considered.** A periodic contract poll on the node's topology refresher. Rejected in favour of event subscription for lower latency; the topology refresher explicitly preserves the agent set rather than reloading it.

**Consequence.** The gate is by SOURCE IP only (not public key), so the IP an agent connects from must equal its on-chain `mixnet_address` IP (canonicalised on both insert and lookup), and NAT or a differing egress IP breaks probing. Because there is NO periodic reconciliation against the contract, a node that misses a revoke event (for example during websocket downtime) only re-syncs on its next restart. Both are documented current-state facts (see Risks and Open Questions).

### Decision 3: Orchestrator / agent split of responsibilities

**Choice.** The orchestrator is the single stateful, chain-identified daemon per operator: it holds the ed25519 identity and Nyx account, maintains the node registry and result database, assigns work, authorises agents on-chain, and submits results. Agents are cheap, stateless workers that only run a probe and report it.

**Why.** Concentrating chain identity, persistence, and submission in one place keeps the agents trivially horizontally scalable and disposable, and keeps exactly one signer per operator for nym-api's replay accounting.

**Consequence.** The orchestrator is a single point of failure and of chain-fee expenditure for its fleet; agents cannot do anything useful without a reachable, authorised orchestrator.

### Decision 4: The agent is a stateless one-shot job

**Choice.** `run-agent` announces, requests one assignment, tests one node, submits, and exits. Scale is achieved by launching many short-lived agent processes (for example as scheduled jobs), not by one long-running daemon.

**Why.** A run-to-completion job is simple to schedule, isolate, and scale elastically, and it matches the "one measurement per invocation" model cleanly. It also means a crashed agent leaves no daemon state - only a `testrun_in_progress` row that the orchestrator's eviction sweep reclaims.

**Consequence.** Throughput is governed by how frequently agents are launched and by the orchestrator's staleness gate, not by an in-agent loop. An empty assignment is a normal, expected exit.

### Decision 5: Two-hop, high-rate, looped-back probing

**Choice.** A stress test routes packets over `[tested_node, this_agent]` so the node relays each packet straight back to the agent, at a fixed target rate for a fixed window, and scores the delivery ratio (plus latency statistics from the returned packets).

**Why.** The goal is to measure a single node's forwarding capacity and reliability UNDER LOAD, in isolation. A two-hop loop attributes loss to the one node in the path (plus the agent's own link) and lets the agent both send and receive without any cooperating third party. This differs from v1's multi-route topology-substitution approach, which measures reachability at low volume across the live network.

**Consequence.** The score conflates the node with the agent's own network path and transient conditions; it is mitigated by high packet volume, repeated staleness-driven runs, and downstream averaging (Decision 13). There is a latency dimension here that v1 lacks (the returned-packet RTT distribution).

### Decision 6: `reuse_header` replays one sphinx header to exercise the bypass

**Choice.** By default the agent builds one sphinx header, re-derives the payload keys by replaying the route key exchange, and re-encapsulates a fresh payload into that same header for every packet - a deliberate replay - and runs a bloomfilter probe first to confirm the node accepts it.

**Why.** Reusing the header avoids rebuilding sphinx headers at the target rate (a real CPU cost at 1000 packets/second) and, more importantly, directly exercises the authorised-monitor replay-bypass path that production stress testing depends on. The upfront bloomfilter probe fails fast if the node is not configured to accept the monitor.

**Consequence.** Correct operation depends on Decision 2's node-side bypass being in place for the agent; without it, every replayed packet is dropped and the node scores near zero.

### Decision 7: The reported sent count is forced to the expected count

**Choice.** On a successful load test the agent overwrites `packets_sent` with `expected_packets = floor(target_rate * sending_duration)` rather than the number it actually pushed through, so the downstream `received / sent` ratio becomes `received / expected`.

**Why.** A node that cannot keep up applies TCP back-pressure to the agent's egress connection, so the agent physically sends fewer packets. Scoring against the actual-sent count would HIDE that: a slow node would look reliable because it received most of the few packets it allowed through. Forcing the denominator to the expected count makes throttling a node's own performance problem.

**Consequence.** A node that drops the agent for a legitimate transient reason, or a lossy agent link, can score low. The per-batch actual counts are retained only for the send-error early-exit path so mid-run aborts still show partial progress. Whether this back-pressure penalty is the intended semantics is an Open Question.

### Decision 8: Queue-less, crash-safe lazy assignment via a database lock set

**Choice.** There is no in-memory work queue. Assignment is a single `BEGIN IMMEDIATE` SQL statement that picks the never-tested-or-oldest eligible mixnode (excluding those with an open `testrun_in_progress` row and requiring known keys), and atomically inserts an in-progress row. A stale-eviction task reclaims in-progress rows older than `test_timeout`.

**Why.** Pushing the "queue" into the node table keyed by staleness makes the orchestrator restartable with zero in-memory work state, makes concurrent agent requests race-safe via the write transaction, and makes an abandoned dispatch self-healing.

**Consequence.** Fairness and freshness are governed by the `test_interval` staleness gate and the eviction cadence rather than an explicit scheduler. A node with missing keys (never successfully self-described) is simply never assigned.

### Decision 9: Layered authentication - bearer token plus on-chain allow-set at two seams

**Choice.** The orchestrator's HTTP API uses two static bearer tokens (agents versus operators). The agents do not sign their requests. The real authorisation that lets an agent probe nodes is enforced at two on-chain seams instead: the orchestrator WRITES the agent to the contract at announce time, and nym-api CHECKS the orchestrator's signer against the contract at ingest time.

**Why.** The bearer token only needs to keep untrusted callers off the orchestrator's API; the security-critical decision (may this agent stress a node) is made by nodes reading the chain (Decision 2), and the integrity of submitted results is protected by the orchestrator's signature (Decision 10). Adding per-request signing between agent and orchestrator would duplicate what the on-chain identity already provides.

**Consequence.** The bearer tokens are shared secrets whose leak or rotation is an operational concern (accepted as-is; see Resolved Questions Q3), and a valid token does not by itself let an agent probe anything.

### Decision 10: Signed, strictly-monotonic, at-least-once batch submission

**Choice.** The orchestrator signs each result batch with its ed25519 identity key over the JSON body, stamps a strictly increasing timestamp (bumping by 1 nanosecond if the clock did not advance), and POSTs it. nym-api checks staleness (30s), contract membership of the signer, strict timestamp monotonicity against a per-signer high-water mark, and the signature. The orchestrator advances its submission watermark only after a successful POST; nym-api deduplicates at the database by `(testrun_id, submitter_pubkey)`.

**Why.** The timestamp high-water mark gives cheap replay protection without server-side per-message storage; at-least-once delivery plus a database primary-key dedupe makes retries safe and idempotent. Signing binds each batch to the on-chain orchestrator identity.

**Alternative considered.** A per-message nonce or exactly-once protocol. Rejected as heavier than a monotonic timestamp plus primary-key dedupe.

**Consequence.** nym-api's high-water mark is in-memory and resets on restart, falling back to the process-online time; this is a documented behaviour backstopped by the database primary-key dedupe, and persisting the watermark is an identified follow-on (Resolved Questions Q8). A clock that runs backwards on the orchestrator would stall submission until it catches up.

### Decision 11: The node registry is built from the mixnet contract plus self-description

**Choice.** The orchestrator learns the node population from the MIXNET contract's bonds and then queries each node's own self-described HTTP endpoint for the socket address, noise key, sphinx key, key-rotation id, and type - not from nym-api. It stores all nodes, including unreachable ones, to retain prior keys.

**Why.** The orchestrator needs raw sphinx and noise material to build probe packets and Noise connections, which is exactly what a node self-describes; sourcing it directly keeps the orchestrator independent of nym-api's topology view. Retaining unreachable nodes avoids losing keys during transient outages.

**Consequence.** A node that never answers its self-description is never testable (its key fields stay NULL and assignment skips it). The registry can lag the chain by up to `node_refresh_rate`.

### Decision 12: Mixnodes-only today, with an unwired gateway seam

**Choice.** The orchestrator only assigns mixnode-capable nodes, only records the mixnode test type, and nym-api drops any non-mixnode entry. A gateway test type exists in the data model but is not wired to any path.

**Why.** Current product scope is mixnode stress testing. The gateway type is scaffolding for the planned liveness work (see Future direction), not live behaviour.

**Consequence.** The spec documents mixnodes-only as the normative current behaviour; the gateway seam is called out as unused so it is not mistaken for a feature.

### Decision 13: Downstream consumer catalog

**Choice.** This is a catalog, not a mechanism choice: it fixes the consumer surface of the stored stress-test results so the blast radius is legible. Stored results are aggregated (average performance plus a reachability flag over a window) into a stress-testing score; that score feeds the node performance provider, which folds it with routing and configuration components into each node's detailed performance (gated by `use_stress_testing_data`, `minimum_available_stress_testing_results`, and `stress_testing_score_weight`, and only for eligible mixnodes); and the composite performance flows into rewarding.

**Why.** A reader asking "what changes if a node's stress score drops?" needs the fan-out (aggregation to provider to rewarding, subject to the gating flags) in one place. The consumer subsystems keep their own capabilities; this catalog is the index.

**Consequence.** "The stress tester feeds rewards" is only true when the stress-testing flags are enabled and the node has enough results; otherwise the results are stored and queryable but do not move performance.

## Risks / Trade-offs

- **Two-hop score conflation.** A node's score reflects the node plus the agent's own link plus transient conditions. Mitigated by high volume, repeated runs, and downstream averaging.
- **Back-pressure penalty can over-penalise.** A legitimately-transient drop or a lossy agent link scores a node low (Decision 7). Accepted as intended behaviour (Resolved Questions Q2).
- **Source-IP-only replay bypass.** The node's bypass gate keys on source IP, not public key, so an attacker able to occupy or spoof an authorised agent's IP would inherit the replay bypass; and NAT breaks legitimate agents. Documented as a current property; an identified follow-on will key the bypass on the Noise-authenticated static key (Resolved Questions Q7).
- **No periodic node reconciliation.** A node that misses a revoke event only re-syncs on restart, so a revoked agent could retain acceptance on that node until it restarts. An identified follow-on will add periodic reconciliation (Resolved Questions Q6).
- **Orchestrator restart re-announces and re-pays.** The agent registry is in-memory; after a restart each agent re-announces and the orchestrator re-submits an `AuthoriseNetworkMonitor` transaction, spending fees. Accepted trade-off of the stateless-agent design.
- **Cascade-delete gas ceiling.** Revoking an orchestrator cascade-deletes its agents in one transaction; a very large agent set could exceed a block's gas. Flagged in-source as a future optimisation; not a risk at current cardinality.
- **Single point of failure.** One orchestrator per operator gates its whole fleet.

## Future direction (non-normative)

This section is NOT part of the normative spec; it records the known roadmap so the current-behaviour spec can be extended coherently. Agents are expected to also perform single-hop LIVENESS checks for mixnodes AND gateways, in order to REPLACE the nym-api-internal v1 network monitor, and to take over some of the gateway-probing tasks currently performed by node-status-api (the gateway-probe work is tracked in PR #6945 on branch `openspec/ns-api-gw-probe`, referenced here but not deep-dived). The current implementation already carries the seams for this: a gateway test type in the data model, a node registry that retains gateway-capable nodes, `is_mixnode` / `was_reachable` fields on each result, and a contract/agent authorisation model that is node-type-agnostic. When that work lands it will be specified as its own change (or a modification to the `nym-network-monitor` capability), not folded into this current-behaviour baseline.

## Resolved Questions

All questions were walked through with the maintainer on 2026-07-23. Following the precedent of the archived reverse-engineering specs, each resolves as either "document as current behaviour" or "open a follow-on change"; no resolution edits the spec to describe behaviour the code does not have. Five resolve as document-and-keep; three (Q6, Q7, Q8) also open a follow-on change (listed under "Identified follow-on changes" below).

1. **Mixnodes-only horizon (Decision 12).** RESOLVED - keep mixnodes-only. The gateway path stays a non-normative Future-direction note and will be specified when the liveness rework lands, not now.
2. **Back-pressure penalty (Decision 7).** RESOLVED - intended, documented as-is. A node that throttles the agent is meant to score low; no follow-on.
3. **Bearer-token auth (Decision 9).** RESOLVED - accepted as-is. Bearer-only agent auth is sufficient because the security-critical decision is made on-chain by nodes and result integrity is signed; no follow-on.
4. **v2 crate status.** RESOLVED - the standalone locust-driven `nym-network-monitor/` (v2) crate is NOT in use. The spec describes it as an unused legacy crate rather than an active system.
5. **Contract admin.** RESOLVED - the production admin is the Nymtech SA multisig / governance; the spec statement stands.
6. **No node-side reconciliation (Decision 2).** RESOLVED - documented as current behaviour, and a follow-on change is opened. The event-only propagation is the baseline; the follow-on adds a periodic reconciliation sweep so a node that missed a revoke event re-syncs without a restart.
7. **Source-IP-only replay bypass (Decision 2).** RESOLVED - documented as current behaviour, and a follow-on change is opened. Investigation during review established that the fix needs NO packet/wire-format change: the `XKpsk3` responder handshake already receives and possession-authenticates the initiating agent's x25519 static key (the message-3 `se` DH proves the agent holds the corresponding private key), and the node can match that authenticated key against the on-chain `bs58_x25519_noise` set. Today the responder verifies no such key (the PSK is derived from the node's own public key, so any peer knowing the node's public key can handshake) and the bypass is gated purely on source IP. The follow-on plumbs the authenticated static key to the bypass check, removing the IP-spoof / NAT vector.
8. **nym-api replay watermark reset.** RESOLVED - documented as current behaviour, and a follow-on change is opened. The database primary-key `(testrun_id, submitter_pubkey)` dedupe already makes replays idempotent, so the in-memory watermark is defense-in-depth rather than a data-integrity guarantee (a restart resetting it to the process-online time cannot cause duplicate or fabricated rows). The follow-on persists the per-signer high-water mark across restarts as hardening.

## Identified follow-on changes

These are recorded inline in the `nym-network-monitor` spec as clearly-marked "intended follow-up" notes on the relevant requirements, so anyone reading the canonical spec sees them; each will become its own OpenSpec change when implemented. None are part of this current-behaviour baseline:

- **Node-side periodic reconciliation** (from Q6): add a periodic reconciliation of each node's authorised-agent set against the contract, so a missed `Revoke` event does not linger until the next restart.
- **Identity-keyed replay bypass** (from Q7): gate the node's sphinx replay/bloomfilter bypass on the agent's Noise-authenticated x25519 static key instead of its source IP (no wire-format change).
- **Persisted replay watermark** (from Q8): persist nym-api's per-signer submission high-water mark across restarts.
