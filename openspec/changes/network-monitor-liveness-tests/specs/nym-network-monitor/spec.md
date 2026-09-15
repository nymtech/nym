## ADDED Requirements

### Requirement: Liveness is a second test kind whose mixnode probe is the stress probe at a low-volume profile

The subsystem SHALL support a second test kind, `liveness`, alongside `stress`. Every unit of work MUST be identified by a `test_kind`, and the orchestrator MUST be the party that decides which kind a given assignment carries; an agent MUST support every kind.

For a mixnode (or a `mixnode_and_gateway` node), a liveness probe SHALL be the same two-hop self-loop probe as a stress test - route `[tested_node, this_agent]`, `AckPacket`-sized mix packets, the same connectivity and bloomfilter probe sequence, the same `reuse_header` behaviour - executed under its own low-volume profile (`liveness_packets`, `liveness_target_rate`, `liveness_waiting_duration`) rather than the stress profile. The reported sent count MUST be forced to the profile's expected packet count on success, exactly as for a stress test, so a node that applies back-pressure to a liveness probe is penalised rather than flattered.

Because the probe traverses exactly one node besides the agent, a liveness score MUST depend only on that node and the agent's own link. No other node of the network may appear in the measured path.

#### Scenario: A mixnode liveness probe measures only the tested node
- **WHEN** a liveness assignment for a mixnode is executed
- **THEN** the packets traverse only the tested node and return to the agent, so no third node's behaviour can affect the score

#### Scenario: The liveness profile is used rather than the stress profile
- **WHEN** a liveness probe runs against a mixnode
- **THEN** it sends `liveness_packets` packets at `liveness_target_rate` and waits `liveness_waiting_duration` for stragglers, leaving the stress profile's values untouched

#### Scenario: Back-pressure on a liveness probe is penalised
- **WHEN** a node throttles the agent so that fewer than the expected packets are pushed, and the probe otherwise completes
- **THEN** the reported sent count is the expected count, lowering the node's delivery ratio

### Requirement: Gateway liveness is one indivisible test of two phases over a single client session

A gateway liveness assignment SHALL be a single, indivisible unit of work performed by ONE agent within ONE gateway client session, comprising two phases:

1. **Ingress** - the agent, acting as a gateway client, submits a `ForwardSphinx` request whose next hop is the agent's own mixnet address, and counts the packets that arrive at its mixnet listener. This exercises the client session, the bandwidth path, and the gateway's outbound forwarder (including its Noise handshake as initiator). The gateway performs NO sphinx processing on this path: the client supplies an explicit next hop and the gateway forwards the packet verbatim, so an ingress failure implicates the session or the forwarder and never the sphinx layer.
2. **Egress** - the agent, acting as a mixnode, sends packets to the gateway's mixnet listener as FINAL-hop packets addressed to its own client session, and counts the packets pushed back to it over that session. This exercises the gateway's mixnet ingress, its final-hop sphinx unwrapping, its destination resolution, and its delivery to a live client.

The client session MUST be established before either phase and held open through the drain window of both, because final-hop delivery requires a live session at the instant the packet arrives. The two phases MUST NOT be assignable, executable, or submittable separately.

A gateway liveness run MUST produce a signal for each phase. The score denominator MUST be fixed by the test kind at two signals, so a phase that produces no signal scores zero rather than being excluded from the average. A phase-1 failure MUST NOT abort the run; only failure to establish the client session aborts it, in which case both signals are zero. Neither phase relies on `forward_hop_processing_enabled`, which is derived from a node's mixnode mode and is disabled on a gateway-only node.

#### Scenario: Both phases run in one session by one agent
- **WHEN** a gateway liveness assignment is handed out
- **THEN** a single agent establishes one client session and performs both the ingress and egress phases within it, and no other agent is ever assigned either phase of that run

#### Scenario: A dead egress path is not hidden by a healthy ingress path
- **WHEN** the ingress phase returns every packet and the egress phase returns none
- **THEN** the run records one full and one zero signal, and the reported score is their average over the kind's fixed two-signal denominator

#### Scenario: An ingress failure still yields an egress measurement
- **WHEN** the gateway fails to forward the ingress phase's packets
- **THEN** the run continues into the egress phase and records the ingress signal as zero, rather than aborting

#### Scenario: A session that cannot be established scores zero on both phases
- **WHEN** the agent cannot establish its client session with the gateway
- **THEN** the run aborts with the failure recorded and both signals are zero

### Requirement: Gateway client sessions are established over plain ws at an announced ip address

The agent SHALL establish its gateway client session at `ws://<announced-ip>:<clients_ws_port>`, constructed from the address the orchestrator assigned and the gateway's announced client websocket port. It MUST ignore any announced hostname and any announced wss entry, and MUST NOT reuse a topology helper that prefers either.

This is required so that the session reaches the node itself rather than an operator-owned TLS terminator or reverse proxy, and so that the probe carries no DNS or certificate dependency. A Nym node binds NO TLS listener: its clients websocket is bound unconditionally in entry mode and the announced wss port is an announcement field only, so an announced wss entry always denotes externally terminated TLS and the plain ws port is the node's own listener. Authenticity is unaffected: the registration handshake authenticates the gateway's ed25519 identity, which the orchestrator holds from the mixnet contract, so no transport-level certificate is needed to know which gateway answered.

A gateway liveness run MUST use ONE announced address for both of its phases, so that a run exercises a single address family end to end, consistent with the per-address rotation. Testing a gateway's wss ingress path is explicitly NOT covered by this kind and is recorded as intended future work (a separate test kind), which means a gateway whose plain-ws path works while its wss path is broken WILL score well here.

#### Scenario: The session targets an ip, not a hostname
- **WHEN** a gateway announces both a hostname with a wss port and a plain client websocket port
- **THEN** the agent connects to `ws://<announced-ip>:<clients_ws_port>` and neither resolves the hostname nor negotiates TLS

#### Scenario: The gateway is still authenticated
- **WHEN** the session is established without TLS
- **THEN** the gateway's identity is verified through the registration handshake against the identity key recorded for that node

#### Scenario: Both phases use one address family
- **WHEN** the rotation selects a gateway's ipv6 address for a run
- **THEN** both the client session and the mixnet leg use that ipv6 address, and the sphinx return hop carries the agent's ipv6 address

### Requirement: A liveness assignment is a wave of targets probed concurrently

A liveness assignment SHALL carry a wave of targets that the agent probes CONCURRENTLY, and one wave MUST be one concurrent batch: the agent MUST NOT split a wave into sequentially-executed sub-waves. A stress assignment remains a single target.

Concurrency is required so that the assignment's lease is bounded by the WORST CASE OF ONE target rather than by the sum over the wave, which is what makes a full-population sweep feasible at liveness cadence and what makes a multi-target lease safe at all.

To execute a wave the agent MUST bind ONE shared ingress listener, MUST build a Noise view containing EVERY target's noise key keyed by every address that target is known by, and MUST treat the union of every target's announced addresses as its known-source set. Returned packets MUST be attributed to a target by the source address of the connection they arrive on, which is sound because node ip addresses are unique across the node population (unlike agent addresses, which may share an ip and be disambiguated only by port). The union is collision-free for the same reason.

The send rate MUST be configured as an AGGREGATE budget across the wave with the per-target rate derived from it, never as a per-target rate multiplied by the wave width, so that widening a wave cannot silently multiply the agent's egress load.

Every result MUST be submitted as soon as its own target completes, rather than at the end of the wave, so that each target's in-flight lock is released independently and an agent that dies mid-wave loses only the targets it had not yet reported.

#### Scenario: A wave's duration is bounded by its slowest target, not their sum
- **WHEN** a wave contains several unresponsive targets
- **THEN** they time out concurrently, so the wave completes within roughly one target's worst case rather than the sum of them

#### Scenario: Returned packets are attributed to the right target
- **WHEN** several targets in one wave return packets to the shared listener at the same time
- **THEN** each packet is attributed to the target whose address the connection originated from

#### Scenario: Aggregate load does not scale with wave width
- **WHEN** the wave width is increased
- **THEN** the per-target rate falls so that the aggregate send rate stays within its configured budget

#### Scenario: A crashed agent only holds its unreported targets
- **WHEN** an agent completes part of a wave and then dies
- **THEN** the completed targets have already been submitted and unlocked, and only the unreported ones wait for the lease to expire

### Requirement: The agent's gateway client identity is derived, announced on-chain, and verified by the gateway

The agent SHALL obtain the ed25519 client identity it needs for a gateway client session WITHOUT any additional on-disk key material, by deriving it deterministically from its existing x25519 noise private key using a labelled key-derivation function whose output is used directly as the ed25519 seed. The derivation MUST carry a domain-separation label so that it can never collide with any other value derived from the same secret. Seeding a general-purpose random number generator with the raw private key bytes MUST NOT be used, because it provides no such separation.

The identity MUST be STABLE rather than freshly generated per test, because it is announced before it is used: the agent SHALL include its base58 ed25519 identity public key in its announcement to the orchestrator, and the orchestrator SHALL record it on-chain with the agent's authorisation. A gateway SHALL grant the ephemeral unmetered monitor session ONLY to a client whose registration handshake authenticates an identity present in the on-chain authorised-agent set, and MUST NOT grant it on the basis of the connection's source IP.

Keying this gate on the identity rather than the source IP is required, not preferred. The registration handshake already possession-authenticates the client's ed25519 key before any session exists, so the check costs no new protocol; the exemption it guards grants unmetered mixnet transit that the routing filter does not confine to monitor destinations; and agents share source IPs through distinct host ports while address pools recycle them, so an IP-keyed exemption can be inherited by an unrelated workload. The Noise-authenticated static key that the separately-identified follow-up applies to the mixnet gates CANNOT serve here, because the client websocket port performs no Noise handshake.

The agent's client identity is permanently recognisable to a gateway. This is accepted: the agent's addresses are published on-chain, so liveness traffic is not covert regardless of the identity's lifetime. Rotating the noise key rotates the derived identity and therefore requires re-announcement.

#### Scenario: No new key file is required
- **WHEN** an agent performs a gateway liveness test
- **THEN** it uses an identity derived from its noise key, and no additional key is generated on disk or provisioned by an operator

#### Scenario: A derived identity is stable across runs
- **WHEN** the same agent tests the same gateway on two occasions
- **THEN** the same client identity is presented both times, so an on-chain announcement remains valid and at most one stored entry could ever exist for that pair

#### Scenario: The session exemption follows the identity, not the address
- **WHEN** an authorised agent opens a client session from an egress address that is not among its authorised mixnet addresses, presenting its announced identity
- **THEN** the gateway grants the ephemeral unmetered session, because the gate is the verified identity

#### Scenario: An unannounced identity is metered like any client
- **WHEN** a client presents an ed25519 identity that is not in the gateway's authorised-monitor identity set
- **THEN** the session is metered and requires credentials as normal, whatever its source IP

#### Scenario: A gateway that has not ingested the identity scores the run zero
- **WHEN** an agent tests a gateway that has not yet ingested its announced identity
- **THEN** the session cannot be established without credentials and both phases score zero, which is the same outcome as a missed authorisation event

### Requirement: nym-api ingests liveness batches on their own endpoint with their own replay state

nym-api SHALL accept liveness result batches on an endpoint distinct from the stress-testing batch endpoint, applying the same ordered validation (staleness window, contract membership of the signer, strict per-signer timestamp monotonicity, ed25519 signature over the JSON body) and the same `SignedMessage` envelope shape.

The per-signer high-water mark used for the monotonicity check MUST be held SEPARATELY per endpoint. It MUST NOT be shared with the stress-testing endpoint, because a single orchestrator identity submits both streams and two interleaved streams validated against one high-water mark would reject each other indefinitely.

Unlike stress-test ingest, liveness ingest MUST accept results for gateway-capable nodes as well as mixnodes, and MUST record each result's per-signal breakdown alongside its averaged score. Rows MUST deduplicate at the database on `(testrun_id, submitter_pubkey)` so that at-least-once resends are idempotent.

#### Scenario: Interleaved streams do not reject each other
- **WHEN** one orchestrator submits a stress batch and then a liveness batch whose timestamp is lower than the stress batch's
- **THEN** both are accepted, because each endpoint tracks that signer's high-water mark independently

#### Scenario: Gateway liveness results are accepted
- **WHEN** a liveness batch contains a result for a gateway-only node
- **THEN** it is validated and stored, rather than being dropped as a non-mixnode entry

#### Scenario: A resent liveness result does not duplicate rows
- **WHEN** the same `(testrun_id, submitter_pubkey)` arrives twice
- **THEN** the second insert is ignored

### Requirement: Each test kind defines which node types it assigns and how their results are typed

Each test kind SHALL declare the node types it is eligible to assign. The `stress` kind MUST assign only nodes whose type is `mixnode` or `mixnode_and_gateway` and MUST record its runs as the mixnode test type. The `liveness` kind MUST assign nodes of type `mixnode`, `gateway`, or `mixnode_and_gateway`, selecting the mixnode probe for mixing-capable nodes and the two-phase gateway probe for gateway-capable ones. A node that is both MUST be eligible for both probes, each producing its own signal, and its liveness score MUST be the average over the signals its probes produce.

A node whose type is `unknown` (never successfully self-described) MUST remain ineligible for every kind.

#### Scenario: A gateway-only node is assignable for liveness but not for stress
- **WHEN** the orchestrator selects work for a gateway-only node
- **THEN** it may assign a liveness test and never a stress test

#### Scenario: A dual-role node is measured in both roles
- **WHEN** a `mixnode_and_gateway` node is liveness-tested
- **THEN** it is probed both as a mixing hop and as a gateway, and its score averages every signal produced

#### Scenario: An unclassified node is never assigned
- **WHEN** a node has never answered its self-description
- **THEN** no test of any kind is assigned to it

### Requirement: Orchestrator state is a per-kind SQLite schema and the agent registry is in-memory only

The orchestrator SHALL persist state in a SQLite database whose work-tracking tables are keyed per test kind: a submission-watermark table (one row per kind); `nym_node` (the node registry with its self-described keys, type, announced address set, and gateway client websocket details); a per-kind work-state table keyed `(node_id, test_kind)` holding that kind's last-tested timestamp, last testrun id, and address rotation pointer; `testrun` (completed runs, each recording its kind and which address was tested); a per-signal child table of `testrun` holding the counts and latency distributions of each measured signal; and `testrun_in_progress` (the in-flight dispatch lock set, keyed by `node_id` alone so that only one test of any kind runs against a node at a time, and carrying a materialised `expires_at`).

The per-kind last-tested timestamp MUST be stored directly rather than read through a join onto the last testrun row, so that evicting an old result does not make a node read as never-tested and jump the assignment queue.

The agent registry MUST NOT be persisted; it lives only in the in-memory `KnownAgents` cache and is rebuilt from the contract on each startup, which means agents' announced flags reset across a restart and each agent re-announces (and is re-authorised on-chain) on its next run.

Rehydrating that cache from the contract requires recovering which pair of on-chain entries belongs to one agent. The contract stores one entry per socket address and carries no field linking an agent's two addresses, so the orchestrator MUST group the entries by their x25519 noise key, which is unique per agent (see the network-monitors-contract capability, which does NOT enforce that uniqueness). Entries that do not form exactly one ipv4/ipv6 pair MUST be dropped from the cache rather than guessed at - they are either authorisations predating the paired announcement or leftovers from an agent that has since changed an address - which is safe precisely because the cache only exists to skip redundant contract transactions, and an agent always announces before requesting work.

#### Scenario: Each kind keeps its own staleness and rotation position
- **WHEN** a node has been tested by both kinds
- **THEN** the work-state table holds one row per kind, each with its own last-tested timestamp and rotation pointer

#### Scenario: Evicting an old result does not reset staleness
- **WHEN** a node's last completed testrun is deleted by result eviction
- **THEN** the node's per-kind last-tested timestamp is unchanged, so it does not read as never-tested

#### Scenario: An agent's two on-chain entries are re-paired after a restart
- **WHEN** the orchestrator restarts and reads its agents from the contract
- **THEN** the ipv4 and ipv6 entries sharing one noise key are rehydrated as a single announced agent

#### Scenario: An unpairable on-chain entry is dropped rather than guessed
- **WHEN** an agent has an on-chain entry with no counterpart of the other family under the same noise key
- **THEN** it is left out of the rehydrated cache and re-created by that agent's next announcement, at the cost of one redundant authorisation transaction

#### Scenario: Node registry and results survive a restart
- **WHEN** the orchestrator restarts
- **THEN** its node registry, per-kind work state, completed testruns with their signals, and per-kind submission watermarks are loaded from SQLite

#### Scenario: The agent set is rebuilt from the contract, not from disk
- **WHEN** the orchestrator restarts
- **THEN** its agent registry is rehydrated from the network-monitors contract rather than read from local storage

## MODIFIED Requirements

### Requirement: The node refresher builds the testable-node registry from the mixnet contract and each node's self-description

The node refresher SHALL source the node list from the MIXNET contract (all `NymNodeBond`s), NOT from nym-api. For each bonded node it MUST query that node's self-described HTTP endpoint directly (with host-info verification) to learn EVERY ip address the node announces, its announced mix port, its versioned x25519 noise key, its sphinx key and key-rotation id, and its role-derived `NodeType`. For a node that announces an entry-gateway interface it MUST additionally learn that interface's plain client websocket port, and MUST record whether the node also announces a wss entry (a hostname plus a wss port), because the presence of a wss entry is what distinguishes divergence this subsystem knowingly introduces from divergence that indicates a fault. Per-node queries MUST be bounded by `node_info_query_timeout` (default 10 seconds) and run with concurrency `number_of_concurrent_node_queries` (default 32); a node that fails to answer leaves the corresponding fields NULL. The refresher MUST persist ALL bonded nodes, including unreachable ones (upserting on `node_id`, updating every field except `identity_key`), so that previously-learned keys are retained when a node is transiently unreachable.

The announced address set MUST be canonicalised (`IpAddr::to_canonical()`), deduplicated and sorted before being stored, because test runs rotate through it by position: a node is free to report its addresses in a different order on every refresh (a resolved hostname typically will), and a duplicate entry would stall the rotation on a subset of the set. The stored `mixnet_socket_address` MUST be derived deterministically from the first address of that sorted set plus the announced mix port, and contributes only that port to the address a given run actually targets.

#### Scenario: A reachable node's keys are recorded
- **WHEN** the refresher queries a bonded node that answers its self-description
- **THEN** the node's announced address set, socket address, noise key, sphinx key, key-rotation id, and type are stored

#### Scenario: A gateway's client websocket port is recorded
- **WHEN** the refresher queries a node that announces an entry-gateway interface
- **THEN** the node's plain client websocket port is stored, along with whether it also announces a wss entry

#### Scenario: The announced address set is stored in a stable order
- **WHEN** a node reports its announced addresses in a different order on a later refresh
- **THEN** the stored set is unchanged, because it is canonicalised, deduplicated and sorted before storage, keeping the per-address test rotation stable

#### Scenario: An unreachable node is retained with prior data
- **WHEN** a bonded node does not answer within `node_info_query_timeout`
- **THEN** the node row is still upserted, leaving newly-unknown fields NULL and keeping any previously stored keys

### Requirement: Testruns are assigned lazily from a staleness-ordered node table guarded by an in-flight lock set

There SHALL be no in-memory work queue. Work is identified by `(node_id, test_kind)`, and staleness, the address rotation and the eligibility gates are all evaluated PER KIND, so that kinds running at different cadences do not disturb one another.

When an agent requests work, the orchestrator MUST choose the kind, then select targets inside a `BEGIN IMMEDIATE` write transaction that: excludes any node with a `testrun_in_progress` row, REGARDLESS of which kind that row belongs to; requires the fields that kind needs to be non-null; requires the node's type to be one the kind may assign; treats a node as eligible only if that kind has never tested it or last tested it before `now - staleness_age` for that kind; for the `liveness` kind additionally requires that the node's `stress` kind last ran before `now - liveness_after_stress_cooldown`; orders by that kind's test timestamp ascending with never-tested first; takes one target for a `stress` assignment or up to `liveness_wave_size` targets for a `liveness` assignment; rotates each selected node onto the next address in its announced set FOR THAT KIND; records that address as the node's per-kind rotation pointer; and atomically inserts a `testrun_in_progress` row for each, stamped with `started_at`, the kind, and an `expires_at` of `now` plus that kind's lease budget. The response MUST carry the chosen kind and its per-target payload, or an empty assignment when no eligible node exists.

Excluding any node that has an open in-progress row of ANY kind is required, not incidental: a node being stress-tested at high rate while a liveness probe measures it would bias both results.

The rotation MUST take the address following the previously handed-out one for that kind, wrapping around at the end of the set and restarting from the first address when the pointer is unset or no longer announced. It MUST advance when the assignment is handed out rather than when a result arrives, so a run that is abandoned still moves the node onto its next address. A node stored before the announced set was tracked MUST remain testable by falling back to the single address in its `mixnet_socket_address`.

The staleness gate is per NODE AND KIND while the rotation is per ADDRESS, so a node announcing N addresses has each individual address tested by a given kind roughly every N × that kind's `staleness_age` rather than every `staleness_age`.

#### Scenario: The oldest-tested eligible node is assigned for the chosen kind
- **WHEN** an authorised, announced agent requests a testrun and eligible nodes exist
- **THEN** the orchestrator picks a kind and returns the never-tested-or-oldest node for that kind, inserting a `testrun_in_progress` row for it in the same transaction

#### Scenario: A node under one kind of test is not assigned another
- **WHEN** a node has an open `testrun_in_progress` row from a stress test
- **THEN** it is excluded from liveness assignment until that row is cleared, and vice versa

#### Scenario: Kinds do not disturb each other's rotation
- **WHEN** a liveness test and a stress test are both assigned for one node over time
- **THEN** each kind advances its own rotation pointer, so neither skips addresses because of the other

#### Scenario: A recently stress-tested node is not immediately liveness-tested
- **WHEN** a node's stress test completed more recently than `liveness_after_stress_cooldown`
- **THEN** it is not eligible for a liveness assignment yet, so its liveness score is not measured while it is still recovering from load

#### Scenario: A liveness assignment carries a wave
- **WHEN** an agent is assigned liveness work and many nodes are eligible
- **THEN** up to `liveness_wave_size` targets are returned in one assignment, each with its own in-flight row and lease

#### Scenario: No eligible node yields an empty assignment
- **WHEN** every node is either in progress or was tested by the chosen kind more recently than its `staleness_age`
- **THEN** the agent receives an empty assignment and exits without testing

### Requirement: The orchestrator authorises both of an announcing agent's addresses on-chain in one transaction

An agent SHALL announce a PAIR of mixnet socket addresses, one ipv4 and one ipv6, because a tested node sees whichever family it was reached over as the source of the probe traffic and gates on that source ip; authorising only one family would leave probes over the other rejected. An agent SHALL additionally announce the base58 ed25519 CLIENT IDENTITY public key it will present when opening a gateway client session, derived from its noise key rather than provisioned, so that the gateway session exemption can be keyed on a verified identity instead of a source address.

On `POST /v1/agent/announce` the orchestrator SHALL reject with a 400, before touching any state, an announcement whose addresses are not one plain ipv4 address and one ipv6 address that is not ipv4-mapped. Such a pair MUST NOT be normalised into shape, because an ipv4-mapped ipv6 address collapses onto the ipv4 one when a node canonicalises the authorised set, leaving the agent with a single authorised ingress while both the contract and the orchestrator believe it has two, and because rewriting an address would authorise something the agent never announced and will not use in its sphinx return hop. An announced identity key that is not valid base58 decoding to 32 bytes MUST likewise be rejected with a 400 at the same point.

It MUST then upsert the agent into its in-memory `KnownAgents` cache, keyed by the agent's ipv4 mixnet socket address with the ipv6 address and the identity key held inside the entry, and, if the agent was not already announced, MUST authorise BOTH addresses in the network-monitors contract by submitting ONE transaction carrying an `AuthoriseNetworkMonitor` message per address, each with the agent's base58 x25519 noise key, noise version, and identity key, then mark the agent announced. Both authorisations MUST travel in a single transaction so that an agent is never left with only one of its addresses authorised. A contract transaction failure MUST surface as a 500 and leave the agent un-announced; re-announcing is safe because the contract's agent save is an upsert. An agent whose announced noise key, ipv6 address, OR identity key differs from the cached one MUST have its announced flag reset so it is re-authorised, and that divergence SHOULD be surfaced (log plus counter) because a superseded ipv6 address stays authorised in the contract. This on-chain write is what ultimately causes network nodes to accept the agent's probe connections and to recognise its client sessions.

Because the identity is carried on an OPTIONAL contract field and the save is an upsert, agents authorised before the field existed acquire it on their next announcement with no backfill step. The orchestrator MUST NOT treat a cached entry without an identity as invalid, since it may have been rehydrated from such an entry after a restart.

#### Scenario: A first announcement authorises both addresses on-chain
- **WHEN** a not-yet-announced agent calls `announce`
- **THEN** the orchestrator writes a single transaction carrying one `AuthoriseNetworkMonitor` message for each of the agent's two addresses, both with its noise key and its identity key, and marks it announced

#### Scenario: A contract failure is not silently swallowed
- **WHEN** the authorisation transaction fails
- **THEN** the announce call returns a 500 and the agent remains un-announced, with neither address authorised

#### Scenario: A malformed address pair is rejected outright
- **WHEN** an agent announces two addresses of the same family, the same address twice, or an ipv4-mapped ipv6 address
- **THEN** the call is rejected with a 400 and nothing is cached or written on-chain

#### Scenario: A malformed identity key is rejected outright
- **WHEN** an agent announces an identity key that is not valid base58 decoding to 32 bytes
- **THEN** the call is rejected with a 400 and nothing is cached or written on-chain

#### Scenario: A changed identity key triggers re-authorisation
- **WHEN** an already-announced agent announces an identity key that differs from the cached one
- **THEN** its announced flag is reset, both addresses are authorised again carrying the new identity, and the divergence is logged and counted

### Requirement: Network nodes learn the authorised-agent set from the contract and gate connection, routing, and replay-bypass on it

A Nym node SHALL derive which network-monitor agents may probe it directly from the network-monitors contract, not from any orchestrator or nym-api. A node MUST load the full authorised-agent set once at startup (via `get_all_network_monitor_agents`; a failed load aborts node startup) and MUST thereafter keep it current in REAL TIME through a nyxd websocket event subscription that dispatches `AuthoriseNetworkMonitor`, `RevokeNetworkMonitor`, and `RevokeAllNetworkMonitors` contract events. This is an event subscription, NOT a periodic contract poll; the node's periodic topology refresher explicitly preserves (does not reload) the agent set.

The node MUST fold the set into THREE shared, lock-free structures: a canonical-IP-keyed routing set (`RoutableNetworkMonitors`), a canonical-IP-keyed noise-key map (`NoiseNetworkView`, in which one IP may host several agents disambiguated by port), and a set of announced monitor ed25519 CLIENT IDENTITIES keyed by that identity rather than by any address. The two IP-keyed structures MUST key on `IpAddr::to_canonical()` at insert AND lookup so that a v4-mapped-IPv6 form matches its canonical IPv4 form. The identity set MUST tolerate the same identity arriving from several agent entries, since an agent authorises one entry per address family and both carry its identity, and MUST tolerate entries carrying no identity at all, which is a validly authorised agent that simply cannot be recognised on the client-session path. There is no separate "extra initiator IPs" allowlist; inbound acceptance is a facet of the noise map. The authorised set MUST gate five behaviours: (1) the Noise responder handshake - an inbound connection from an IP not in the noise map falls back to raw TCP and the agent's handshake fails; (2) packet routing through `NetworkRoutingFilter`, in which a packet originating from an authorised monitor may ONLY be routed to another authorised monitor; (3) most importantly, the sphinx REPLAY / bloomfilter BYPASS - a packet detected as replayed MUST be dropped as a replay UNLESS it originates from an authorised network-monitor agent IP, which is the mechanism that lets the agent's deliberately-replayed probe header (see the `reuse_header` requirement) be processed rather than filtered; (4) FINAL-HOP DELIVERY - a final-hop packet originating from an authorised monitor MUST be processed and delivered to a live client session, and MUST NOT be written to the recipient's on-disk store if no session is live, so that a packet which did not arrive on the socket was definitively not delivered and monitor traffic cannot accrue undeliverable stored messages on every gateway; and (5) CLIENT SESSION METERING - a client websocket session whose registration handshake authenticates an ed25519 identity in the announced-identity set MUST be treated as an ephemeral monitor session: it MUST NOT be metered for bandwidth, MUST NOT require any bandwidth credential, and MUST NOT persist a shared-key, bandwidth, or stored-message entry.

Gates (4) and (5) are what make gateway liveness testing possible; before them a monitor's final-hop packets were dropped outright as unsupported, and a monitor could not open a client session without presenting bandwidth credentials.

Gates (1) through (4) are keyed by SOURCE IP only (not public key); the port is effectively ignored on the agent-as-initiator probe path (it is consulted only when the node dials an agent). Gate (5) is the ONE exception and MUST be keyed on the handshake-verified client identity, with the source IP playing no part, because the client websocket port performs no Noise handshake and so can never be covered by the follow-up that moves the mixnet gates onto the Noise-authenticated static key, and because the exemption it guards is not confined by gate (2): a packet handed to a gateway over a client session is forwarded without the monitor flag and may therefore be routed to any known node, so an IP-keyed exemption inherited by a co-tenant behind a shared host port or a recycled address pool would grant unconfined unmetered transit.

The consequences are: an agent cannot successfully probe a node until it is authorised on-chain AND that authorisation event has been ingested by the node (propagation is bounded by block inclusion plus websocket delivery, on the order of seconds, NOT by any refresh interval); for gates (1) through (4) the IP the agent actually connects from MUST equal one of the `mixnet_address` IPs recorded on-chain for it (an egress IP that is neither, whether through NAT or a third interface, still breaks all four); an authorised agent with an announced identity obtains unmetered gateway transit for sessions presenting that identity, bounded by the orchestrator's ability to revoke the authorisation; and because there is no periodic reconciliation against the contract, a node that misses a revoke event (for example during websocket downtime) only re-syncs on its next restart's one-time load, which for gate (5) means a revoked monitor keeps its unmetered sessions until then.

Because an agent authorises one ipv4 and one ipv6 address, it occupies TWO entries in each node's structures - one per address, both carrying the same noise key - so a probe arriving over either family passes every gate. The node treats those entries independently: it neither knows nor needs to know that they belong to one agent, and revoking one leaves the other authorised.

Intended follow-ups (recorded here as planned changes, NOT current behaviour): (1) add a periodic reconciliation of each node's authorised-agent set against the contract, so a missed revoke event no longer lingers until the next node restart - this is a prerequisite for liveness scores ever carrying weight, because a node that missed its agents' authorisation events fails every gate and is indistinguishable from a dead node; and (2) gate the replay bypass and the final-hop delivery on the agent's Noise-authenticated x25519 static key rather than its source IP. The current `Noise_XKpsk3` handshake already receives and possession-authenticates that key (the message-3 `se` step proves the agent holds the corresponding private key), so this hardening needs no packet-format change and would remove the source-IP spoofing and NAT-fragility of the present gates. Follow-up (2) covers the MIXNET gates only; the client-session exemption is out of its reach and is why gate (5) is identity-keyed from the outset.

#### Scenario: A newly authorised agent is accepted in near real time
- **WHEN** an orchestrator authorises an agent on-chain and the transaction is included in a block
- **THEN** each node's websocket watcher ingests the `AuthoriseNetworkMonitor` event and adds the agent's IP and noise key to its routing set and noise map without waiting for any refresh interval

#### Scenario: An unauthorised agent cannot complete a handshake or have replays accepted
- **WHEN** an agent that the node has not ingested opens a connection and sends replayed packets
- **THEN** the Noise handshake falls back to raw TCP and fails, and any replayed packet is dropped as a replay because it does not come from an authorised agent IP

#### Scenario: Replayed probe traffic from an authorised agent bypasses the bloomfilter
- **WHEN** an authorised agent sends its deliberately-replayed probe header
- **THEN** the node still runs its replay-detection bloomfilter but bypasses the drop because the packet's source IP is in the authorised network-monitor set, and processes the packet

#### Scenario: A monitor's final-hop packet is delivered to its live session
- **WHEN** an authorised agent sends a final-hop packet to a gateway addressed to a client session it currently holds open
- **THEN** the gateway unwraps it and pushes it into that session

#### Scenario: A monitor's final-hop packet is dropped rather than stored
- **WHEN** an authorised agent sends a final-hop packet whose recipient has no live session
- **THEN** the packet is dropped and nothing is written to the on-disk store

#### Scenario: A monitor's client session needs no bandwidth
- **WHEN** an authorised agent opens a client websocket session presenting its announced ed25519 identity and forwards packets
- **THEN** the session is not metered, no credential is required, and no shared-key, bandwidth, or stored-message entry is persisted for it

#### Scenario: An authorised IP alone does not earn the session exemption
- **WHEN** a client opens a websocket session from an authorised agent's IP but presents an identity that is not in the announced-identity set
- **THEN** the session is metered and requires credentials like any other client's

#### Scenario: Revocation stops acceptance after the event is ingested, with no periodic re-sync
- **WHEN** an agent is revoked on-chain and the node ingests the `RevokeNetworkMonitor` event
- **THEN** the node removes it from the routing set and noise map so new handshakes fail, replays are dropped again, and its client sessions are metered like any other
- **AND** if the node misses that event it will only re-sync the agent set on its next restart, because there is no periodic reconciliation

### Requirement: Completed testruns are submitted to nym-api in signed, monotonic batches with at-least-once delivery

The result submitter SHALL forward completed testruns to nym-api, in a SEPARATE STREAM PER TEST KIND. Each stream MUST have its own destination endpoint and its own persisted watermark, because one watermark cannot describe two destinations: advancing a shared watermark for one stream would skip unsubmitted rows of the other. Stress results MUST be submitted to `POST /v3/nym-nodes/stress-testing/batch-submit`; liveness results MUST be submitted to their own endpoint.

For each stream the submitter MUST read that stream's persisted watermark, fetch completed testruns of that kind after it in ascending id order, and send them in chunks of `result_submission_batch_size` (default 50). Each stress `TestRun` MUST be converted to a `StressTestResult` whose `test_performance` is `packets_received / packets_sent` (or `0.0` when `packets_sent` is zero or duplicates were seen) and whose `was_reachable` is `error.is_none()`. Each liveness `TestRun` MUST be converted to a result whose performance is the average over that kind's fixed signal set, carrying the per-signal breakdown, with a signal that produced no measurement counted as zero. Each batch MUST be wrapped in a submission content carrying `{ signer, timestamp, results }`, given a timestamp that is strictly increasing (bumped by 1 nanosecond if the clock has not advanced since the last batch, matching nym-api's replay guard), and signed with the orchestrator's ed25519 identity key. Each stream's watermark MUST be advanced only AFTER a successful POST, so a failed submission re-sends the same testruns on the next cycle (at-least-once delivery).

#### Scenario: Only new testruns are submitted, in order
- **WHEN** the submitter runs with a watermark of N for a given kind
- **THEN** it submits testruns of that kind with id greater than N in ascending id order, chunked by the batch size

#### Scenario: One stream's progress does not advance another's
- **WHEN** a liveness batch is submitted successfully while stress results are still pending
- **THEN** only the liveness watermark advances and the pending stress results are still submitted on their own stream

#### Scenario: A failed POST is retried, not skipped
- **WHEN** a batch POST fails
- **THEN** that stream's watermark is not advanced and the same testruns are resubmitted on the next cycle

#### Scenario: Batch timestamps are strictly monotonic
- **WHEN** two batches are produced within the same clock tick
- **THEN** the second batch's timestamp is bumped so it is strictly greater than the first, satisfying nym-api's replay check

### Requirement: Stale in-flight dispatches and old results are evicted

The stale-data eviction task SHALL clear `testrun_in_progress` rows whose `expires_at` has passed, so that a dispatch abandoned by a crashed or hung agent frees its node for reassignment, and MUST delete completed testruns older than `testrun_eviction_age` (default 7 days), along with their per-signal rows. One eviction sweep MUST run before the HTTP server begins serving.

The deadline MUST be materialised on the in-progress row at hand-out rather than derived by the sweep from a single global timeout, because different test kinds have different budgets (a stress run is minutes, a liveness wave is seconds) and a future kind may be more expensive still. The sweep therefore requires no knowledge of kinds.

#### Scenario: A timed-out dispatch is released
- **WHEN** a `testrun_in_progress` row's `expires_at` has passed
- **THEN** it is removed and the node becomes eligible for assignment again

#### Scenario: Kinds with different budgets coexist
- **WHEN** a long-budget stress dispatch and a short-budget liveness dispatch are both in flight
- **THEN** each is evicted according to its own deadline, and the short one does not keep the long one alive nor vice versa

#### Scenario: Old results are pruned
- **WHEN** completed testruns are older than `testrun_eviction_age`
- **THEN** they and their per-signal rows are deleted from the database

### Requirement: The agent is a one-shot job that announces, requests one assignment, tests, submits, and exits

The `run-agent` path SHALL be a run-to-completion job, NOT a long-lived daemon: it builds an orchestrator client with a bearer token, loads its x25519 noise key, announces itself, requests a single assignment, and - if one is returned - executes it and exits. An assignment is either ONE stress target or a WAVE of liveness targets executed concurrently; in the wave case each target's result MUST be submitted as soon as that target finishes rather than at the end of the wave. When the assignment is empty it MUST log that no work is available and exit without testing. Fleet scale is therefore achieved by running many short-lived agent invocations rather than one persistent process, with liveness sweep throughput coming from wave concurrency within an invocation rather than from a longer-lived process.

The agent MUST be able to execute every test kind the orchestrator may assign, since the orchestrator is the party that chooses. The agent binary MUST also provide `build-info`, a `keygen` subcommand that generates ONLY an x25519 noise key (no ed25519 key), and a `test-node` subcommand that runs a single manual test against an explicitly-specified node bypassing the orchestrator (with no `node_id`).

#### Scenario: An assignment is executed once and submitted
- **WHEN** the agent receives a non-empty assignment
- **THEN** it executes every target it was given, submits each result, and exits

#### Scenario: No work available exits cleanly
- **WHEN** the agent receives an empty assignment
- **THEN** it logs that no work is available and exits without testing or submitting

#### Scenario: A wave's results are submitted as they complete
- **WHEN** one target of a liveness wave finishes while others are still running
- **THEN** its result is submitted immediately rather than being held until the wave ends

### Requirement: The per-node result captures counts, handshake and latency statistics, and an optional error

Each test SHALL produce a result carrying its test kind, `time_taken`, and an optional `error`, plus ONE OR MORE SIGNALS. Each signal MUST carry: ingress and egress Noise-handshake durations; the sphinx packet delay; `packets_sent` and `packets_received`; the baseline `approximate_latency`; per-packet and per-send latency distributions (minimum, mean, median, maximum, standard deviation); and a `received_duplicates` flag. A stress or mixnode-liveness run produces exactly one signal; a gateway-liveness run produces one per phase. Only a critical failure (for example an inability to bind the ingress listener) MUST bubble up as an error return; node-level failures (no response, bloomfilter misconfiguration, a rejected Noise handshake, a refused client session) MUST be recorded inside the returned result so the orchestrator always receives partial data.

The per-signal breakdown MUST be persisted and exposed on the operator read surface even though downstream consumers receive only the averaged score, because a gateway with a healthy ingress and a dead egress is otherwise indistinguishable from one that is uniformly half-lossy.

The submission that carries a result to the orchestrator MUST additionally report WHICH address was tested, and that address MUST be persisted with the run and exposed on the operator read surface. A node may announce several addresses of which only some are healthy, so without it a per-address failure is indistinguishable from a dead node and gets averaged into that node's single result series.

Latency statistics MUST be recorded but MUST NOT contribute to any score. Two confounds make them unsuitable for scoring today: a node defers replay checking in batches bounded by its deferral time and pending-packet count and only defers when its bloomfilter lock is contended, so a low-volume probe measures a busy node as slower than an idle one in a step function; and measurements taken inside a concurrent wave include the agent's own queueing. Consequently liveness latency figures MUST NOT be compared with stress-test latency figures.

#### Scenario: A node-level failure still yields a result
- **WHEN** a node fails to respond, is misconfigured, or refuses the Noise handshake
- **THEN** the agent returns a result with the failure recorded in its `error` field rather than failing the job

#### Scenario: A result is attributable to the address it measured
- **WHEN** a run against one of a node's several announced addresses fails
- **THEN** the stored run records the tested address, so the failure is attributable to that address rather than to the node as a whole

#### Scenario: A gateway result keeps its per-phase breakdown
- **WHEN** a gateway liveness run is stored and read back
- **THEN** the ingress and egress signals are separately visible, alongside the averaged score submitted downstream

#### Scenario: A rejected handshake scores zero rather than being excluded
- **WHEN** a node rejects the agent's Noise handshake, so nothing can be measured
- **THEN** the result records the failure and scores zero, because a node that will not accept a connection is not routable and an unmeasurable node must not score better than a measurably broken one

### Requirement: nym-api accepts batches only from contract-authorised orchestrators after staleness, replay, and signature checks

The nym-api handler for `POST /v3/nym-nodes/stress-testing/batch-submit` SHALL validate each submission through six ordered steps: (1) reject the batch if its body is older than a 30-second staleness window; (2) reject it unless the signer's ed25519 public key is in the `NetworkMonitorsCache` authorised set, which is populated from the network-monitors contract's authorised-orchestrator identity keys and refreshed lazily on a TTL (default 30 minutes); (3) reject it unless its timestamp is strictly greater than the per-signer high-water mark held in an in-memory `LastNMSubmissions` map, falling back to the process-online time when no prior submission is recorded (for example after a restart); (4) reject it unless the ed25519 signature over the JSON body verifies against the signer; (5) update the per-signer high-water mark; and (6) validate and insert the individual results.

The per-signer high-water mark MUST be scoped to the submission endpoint. Each ingest endpoint MUST keep its own map, so that two streams signed by one orchestrator identity cannot invalidate each other's timestamps.

Because the per-signer high-water mark is held in memory, it resets to the process-online time on restart; the database primary-key dedupe described in the next requirement is what ultimately guarantees idempotency. Intended follow-up (recorded here as a planned change, NOT current behaviour): persist the per-signer high-water mark across restarts as defense-in-depth.

#### Scenario: A batch from an unknown signer is rejected
- **WHEN** the signer's key is not in the contract-derived authorised set
- **THEN** the submission is rejected as unauthorised

#### Scenario: A replayed or out-of-order batch is rejected
- **WHEN** a batch's timestamp is not strictly greater than the signer's last accepted timestamp for that endpoint
- **THEN** the submission is rejected

#### Scenario: A tampered batch fails the signature check
- **WHEN** the body does not match its ed25519 signature for the given signer
- **THEN** the submission is rejected as failing its integrity check

#### Scenario: One endpoint's high-water mark does not gate another's
- **WHEN** the same orchestrator submits to the stress and liveness endpoints with interleaved timestamps
- **THEN** each endpoint evaluates monotonicity against its own map and both submissions are accepted

### Requirement: Stored stress-test scores feed node performance and rewarding through a defined consumer surface

The stored stress-test and liveness results SHALL form the subsystem's output contract to the rest of nym-api; the detailed behaviour of each consumer is owned by its own capability, and this requirement fixes only WHICH subsystems read the results and FOR WHAT. The stored per-node stress results MUST be aggregated (average performance and a reachability flag over a configured window) into a stress-testing score; the stored per-node liveness results MUST be aggregated the same way into a separate liveness score; each score MUST feed the node performance provider, which folds them - together with routing and configuration components - into each node's detailed performance, each gated by its own `use_*_data`, `minimum_available_*_results`, and `*_score_weight` configuration flags; and the resulting composite performance MUST flow into rewarding via the node's rewarding-performance derivation.

The liveness score's weight MUST default to ZERO, so that liveness is recorded and queryable without affecting performance or rewarding until the operator deliberately enables it. This is required because two populations will score zero on liveness for reasons unrelated to their forwarding capability: nodes that have not ingested their agents' on-chain authorisations, and gateways not yet carrying the final-hop and monitor-session behaviour that gateway liveness depends on.

While the weight is zero, nym-api MUST expose a DIVERGENCE metric comparing each node's aggregated liveness score against the v1 monitor's routing score, bucketed by whether the node announces a wss entry gateway address. The bucketing is required because this subsystem deliberately probes only the plain-ws ingress, so a gateway with a broken TLS ingress is EXPECTED to diverge; without the bucket, that expected divergence is indistinguishable from a node that never learned about its agents. This metric is the evidence on which the eventual decisions to weight liveness, and separately to retire the v1 routing score, are to be based.

#### Scenario: Stress scores contribute to node performance when enabled
- **WHEN** stress-testing data is enabled and a mixnode has at least the minimum number of available results
- **THEN** its averaged stress score is folded into its detailed performance according to the configured weight

#### Scenario: Liveness scores are inert by default
- **WHEN** liveness results are stored and aggregated with the default configuration
- **THEN** they are queryable and appear in the divergence metric, but contribute nothing to any node's performance or reward

#### Scenario: Divergence is attributable
- **WHEN** a gateway scores zero on liveness while the v1 monitor scores it as routable
- **THEN** the divergence metric records it in the bucket matching whether it announces a wss entry, so an expected TLS-path divergence is distinguishable from an unexpected one

#### Scenario: The consumer surface is bounded
- **WHEN** reasoning about the blast radius of a stress or liveness score
- **THEN** the readers are the performance aggregation query, the performance provider, and rewarding, each specified by its own capability

### Requirement: The subsystem's behaviour is governed by orchestrator and agent configuration surfaces with defined defaults

The orchestrator SHALL be configured with the following defaults: `test_interval` 2 hours, `test_timeout` 5 minutes, `node_refresh_rate` 2 hours, `node_info_query_timeout` 10 seconds, `testrun_eviction_age` 7 days, `result_submission_interval` 15 minutes, `result_submission_batch_size` 50, `number_of_concurrent_node_queries` 32, `chain_authorisation_check_max_attempts` 10, `chain_authorisation_check_retry_delay` 1 minute, and an HTTP bind of `0.0.0.0:8080`; plus required secrets (`agents_token`, `metrics_and_results_token`, the bip39 `mnemonic`, and the base58 ed25519 `private_key`) and required endpoints (`nym_api_endpoint`, `rpc_url`, the mixnet and network-monitors contract addresses, and `database_path`).

Where a knob governs a per-kind behaviour it MUST be expressible per kind. The orchestrator MUST additionally carry, for the liveness kind: a staleness interval (defaulting well below the stress `test_interval`, so that liveness tracks v1's cadence), a lease budget used as the in-progress `expires_at` (which MUST bound one concurrent wave, not the sum over its targets), a wave size, a `liveness_after_stress_cooldown`, and an enable flag allowing liveness assignment to be switched off without redeploying. `test_timeout` remains the stress kind's lease budget.

The agent SHALL be configured with the following defaults: `sending_duration` 30 seconds, `waiting_duration` 5 seconds, `packet_delay` 50 milliseconds (which MUST be non-zero), `target_rate` 1000 packets/second, `reuse_header` true, `egress_connection_timeout` 5 seconds, `noise_handshake_timeout` 3 seconds, `sending_batch_size` 50, and a listener bind of `[::]:9000`; plus the required orchestrator URL, orchestrator bearer token, announced ipv4 host address, announced ipv6 host address, shared announced port, and noise-key path. The agent MUST additionally carry a liveness profile: a per-target packet count, an AGGREGATE send-rate budget from which the per-target rate is derived (never the reverse), a straggler wait, and per-target timeouts. All knobs MUST be overridable by CLI flag or environment variable.

The liveness profile's initial values are PROVISIONAL, chosen for score granularity rather than measured against agent hardware: a per-target packet count of 100 (giving 1% granularity against v1's three packets per route), an aggregate budget of 500 packets/second, and a wave size of 20. Because they are provisional, every one of them MUST be tunable in a deployment without a code change, and no behaviour may depend on a specific value: an agent host that cannot sustain the aggregate budget MUST be correctable by configuration alone.

The announced pair MUST be validated at configuration time, applying the same rule the orchestrator enforces on announce, so a misconfigured deployment fails immediately rather than on its first announcement. The listener bind default MUST remain dual-stack (`[::]`), since an ipv4-only bind cannot receive the return traffic for a run whose return hop is the agent's ipv6 address, and since one shared listener serves every target of a concurrent wave.

#### Scenario: Defaults match the documented values
- **WHEN** an orchestrator or agent is configured without overriding a given knob
- **THEN** the effective value is the default listed above

#### Scenario: A zero packet delay is rejected
- **WHEN** the agent is configured with a `packet_delay` of zero
- **THEN** configuration construction fails

#### Scenario: A malformed announced address pair is rejected at startup
- **WHEN** the agent is configured with two announced addresses of the same family, or with an ipv4-mapped ipv6 address
- **THEN** configuration construction fails before the agent announces itself

#### Scenario: Liveness can be disabled without a redeploy
- **WHEN** the orchestrator's liveness enable flag is unset
- **THEN** no liveness assignment is handed out and stress testing continues unaffected

## REMOVED Requirements

### Requirement: Orchestrator state is a four-table SQLite database and the agent registry is in-memory only

**Reason**: The schema is no longer four tables, and its shape is no longer describable per node rather than per (node, kind). Work state moves into a per-kind table, results gain a per-signal child table, in-progress rows gain a materialised lease, and the single submission watermark becomes one per kind. The requirement's normative content is replaced by "Orchestrator state is a per-kind SQLite schema and the agent registry is in-memory only", which restates the agent-rehydration rules unchanged.

**Migration**: A migration MUST move `nym_node.last_testrun` and `nym_node.last_tested_ip` into the new per-kind work-state table under the `stress` kind, so existing nodes keep their staleness position and address rotation; MUST backfill `expires_at` on any in-progress row from `started_at` plus the stress lease budget; and MUST carry the existing `metadata.last_submitted_testrun_id` across as the stress stream's watermark. No result data is lost: existing `testrun` rows keep their columns and are read as single-signal runs.

### Requirement: The subsystem tests mixnodes only; the gateway test path is an unwired extension seam

**Reason**: Gateways are now tested by the liveness kind, so the statement that the gateway path is unwired scaffolding is no longer true. Replaced by "Each test kind defines which node types it assigns and how their results are typed", which keeps the mixnodes-only restriction for the stress kind and defines gateway eligibility for the liveness kind.

**Migration**: None for stored data: stress runs continue to be recorded as the mixnode test type and nym-api continues to drop non-mixnode entries on the stress endpoint. Gateway results arrive only on the liveness endpoint, which accepts them.
