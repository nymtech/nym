# nym-network-monitor Specification

## Purpose
The v3 network monitor: chain-authorised, one-shot agents probe individual nym-nodes for stress and liveness, an orchestrator assigns the work and stores the results, nym-nodes gate probe acceptance (and the replay/bloomfilter bypass) on the on-chain authorisation, and every nym-api publishes each node's figure from its settled performance snapshot of the contract the results reach (`network-monitor-contract-submission`). This capability covers the orchestrator and agent lifecycles, the on-chain agent authorisation and its node-side propagation, the probe mechanics and scoring, nym-api's reading of the performance contract and the monitor-era routes it keeps deprecated, and the configuration surface. Rewarding and active-set selection from the contract's figures are `nym-epoch-driver`'s.
## Requirements
### Requirement: The orchestrator gates startup on chain balance, contract authorisation and identity-key reconciliation

Each network-monitor orchestrator SHALL be an independent daemon holding an ed25519 identity keypair and a Nyx (bip39) account. On startup it MUST complete the following gates in order before serving, aborting startup on any failure: initialise its SQLite database; build a direct-signing nyxd client from its mnemonic; verify its signing account holds at least `1_000_000 unym` (to cover its on-chain transaction fees); verify its bech32 address is present in the network-monitors contract's authorised-orchestrator set, retrying up to `chain_authorisation_check_max_attempts` (default 10) spaced by `chain_authorisation_check_retry_delay` (default 1 minute); only when its configuration enables performance submission, verify its account is an authorised network monitor in the performance contract, with the same retry budget, as `network-monitor-contract-submission` specifies; and reconcile its announced identity key, submitting an `UpdateOrchestratorIdentityKey` transaction and sleeping 30 seconds when the on-chain key differs from (or is missing versus) its local key. The orchestrator holds no nym-api client and makes no call to nym-api.

A fleet MAY run more than one orchestrator; each orchestrator manages only the agents it authorised (`authorised_by == self`).

#### Scenario: An unauthorised orchestrator does not start serving
- **WHEN** an orchestrator's bech32 address is not in the contract's authorised set after the configured retries
- **THEN** startup aborts and no HTTP server or background task begins

#### Scenario: A stale on-chain identity key is reconciled before serving
- **WHEN** the identity key announced on-chain differs from the orchestrator's local key
- **THEN** the orchestrator submits `UpdateOrchestratorIdentityKey`, waits for it to take effect, and only then continues startup

#### Scenario: Insufficient balance aborts startup
- **WHEN** the signing account holds fewer than `1_000_000 unym`
- **THEN** startup aborts before any agent can be authorised on-chain

### Requirement: The node refresher builds the testable-node registry from the mixnet contract and each node's self-description

The node refresher SHALL source the node list from the MIXNET contract (all `NymNodeBond`s), NOT from nym-api. For each bonded node it MUST query that node's self-described HTTP endpoint directly (with host-info verification) to learn EVERY ip address the node announces, its announced mix port, its versioned x25519 noise key, its sphinx key and key-rotation id, and whether it reports the mixnode and the entry-gateway role. For a node that announces an entry-gateway interface it MUST additionally learn that interface's plain client websocket port. It MUST NOT record whether the node also announces a wss entry: NOTHING reads that fact. The probe ignores wss entries by construction, the submission carries no such field, and the divergence surface deliberately does not bucket on it, so storing it here would be a copy with no consumer at all. Per-node queries MUST be bounded by `node_info_query_timeout` (default 10 seconds) and run with concurrency `number_of_concurrent_node_queries` (default 32).

A node MUST be described COMPLETELY or not at all: every self-described field comes from one reading of the node's endpoint, and a failure of any part of that reading - including the client websocket interface of a gateway-capable node - MUST discard the whole reading rather than storing the fields that did answer. The refresher MUST record the bond of ALL bonded nodes, including unreachable ones, and MUST write a node's description only from a complete reading, replacing the previous description whole. A node that could not be described keeps its previous description untouched: deleting it would drop a merely slow node out of EVERY kind until a later cycle answered, which at the liveness cadence costs several test slots per incident. A missing description therefore means "never completely described" or "no longer bonded", never "did not answer this time". `identity_key` is never updated, since a `node_id` maps to exactly one identity and is never reassigned.

After every SUCCESSFUL read of the contract's bonds, the refresher MUST delete the description of every node absent from that read and keep its bond, so that an unbonded node stops being assigned while remaining visible on the read surface with its last-seen time. A failed contract read MUST write nothing, so it can never delete a description. Without this an unbonded node keeps its last reading forever and is tested indefinitely, since nothing else removes it.

The consequence, accepted deliberately, is that a node which stops answering keeps its last reading indefinitely while it remains bonded, and continues to be assigned against it. That is the intended behaviour: a probe against stale data fails, and for a liveness measurement an unreachable node failing its probe is the measurement, whereas silently not testing it is not.

The announced address set MUST be canonicalised (`IpAddr::to_canonical()`), deduplicated and sorted before being stored, because test runs rotate through it by position: a node is free to report its addresses in a different order on every refresh (a resolved hostname typically will), and a duplicate entry would stall the rotation on a subset of the set. The description MUST store the announced mix port rather than a socket address: the address a run targets comes from the rotation over the announced set, so an address stored beside the port would only ever be read for its port.

#### Scenario: A reachable node's keys are recorded
- **WHEN** the refresher queries a bonded node that answers its self-description
- **THEN** its description is stored with the node's announced address set, mix port, noise key, sphinx key, key-rotation id, and reported roles

#### Scenario: A gateway's client websocket port is recorded
- **WHEN** the refresher queries a node that announces an entry-gateway interface
- **THEN** the node's plain client websocket port is stored, and its wss announcement is not, no consumer anywhere in the subsystem reading it

#### Scenario: The announced address set is stored in a stable order
- **WHEN** a node reports its announced addresses in a different order on a later refresh
- **THEN** the stored set is unchanged, because it is canonicalised, deduplicated and sorted before storage, keeping the per-address test rotation stable

#### Scenario: An unreachable node is retained with prior data
- **WHEN** a bonded node does not answer within `node_info_query_timeout`
- **THEN** only its bond is recorded, and its previous description survives untouched, so it stays eligible for testing

#### Scenario: A node seen for the first time without answering has a bond and no description
- **WHEN** a node the orchestrator has never described does not answer
- **THEN** its bond is recorded and no description is written, which is what marks it as never described rather than as unreachable this cycle

#### Scenario: A partial reading is discarded
- **WHEN** a gateway-capable node answers for its keys and roles but not for its client websocket interface
- **THEN** the whole reading is discarded and the node keeps its previous description, rather than being stored as described everywhere except that interface

#### Scenario: An unbonded node loses its description
- **WHEN** a node with a description is absent from a successful read of the contract's bonds
- **THEN** its description is deleted and its bond is kept, so it is no longer assigned but is still listed

#### Scenario: A failed contract read deletes nothing
- **WHEN** the contract's bonds cannot be read
- **THEN** the refresh writes nothing and every description is left in place

### Requirement: Testruns are assigned lazily from a staleness-ordered node table guarded by an in-flight lock set

There SHALL be no in-memory work queue. Work is identified by `(node_id, test_kind)`, and staleness, the address rotation and the eligibility gates are all evaluated PER KIND, so that kinds running at different cadences do not disturb one another.

When an agent requests work, the orchestrator MUST choose the kind, then select targets inside a `BEGIN IMMEDIATE` write transaction that: excludes any node with a `testrun_in_progress` row, REGARDLESS of which kind that row belongs to; requires the node to have a description reporting the role that kind probes; treats a node as eligible only if that kind has never tested it or last tested it before `now - staleness_age` for that kind; orders by that kind's test timestamp ascending with never-tested first; takes up to that kind's wave size, which is one for `mixnode_stress`; rotates each selected node onto the next address in its announced set FOR THAT KIND; records that address as that kind's rotation pointer; and atomically inserts a `testrun_in_progress` row for each, stamped with `started_at`, the kind, and an `expires_at` of `now` plus that kind's lease budget. The response MUST carry the chosen kind and its per-target payload, or an empty assignment when no eligible node exists.

The kind MUST be the one whose next assignable node is MOST OVERDUE across every enabled kind. A node is due at its last test by that kind plus that kind's staleness interval, and a node the kind has never tested outranks every measured one. Due times MUST be compared rather than raw last-tested timestamps, since kinds run at different cadences and a two-hour kind would otherwise always look older than a fifteen-minute one. Equally overdue kinds MUST resolve in the kinds' declaration order, which MUST place both liveness kinds ahead of `mixnode_stress`, so that a full cold start, where every kind is equally never-tested, begins with liveness. A disabled kind MUST NOT be considered, and when the chosen kind's assignment comes back empty because a concurrent request took its eligible nodes, the request MUST fall through to the next most overdue kind rather than being answered with no work.

The policy is deliberately not fair, and no kind is owed a share of requests: under sustained overload every kind falls behind by a similar absolute margin, and a backlog of never-tested nodes, from a cold start or from a newly added kind, is drained before any other kind's re-tests. No kind can be starved indefinitely, because a kind left unserved only grows more overdue until it is chosen.

Excluding any node that has an open in-progress row of ANY kind is required, not incidental: a node being stress-tested at high rate while a liveness probe measures it would bias both results. That lock is also the ONLY exclusion between kinds: a node is eligible again the moment its row clears, with no cooldown afterwards, because a stress test's 30000 packets at 1000 packets/second is around 3.1 Mbps at the probe's `AckPacket` sizing and so leaves nothing draining for a later probe to charge to the node.

The rotation MUST take the address following the previously handed-out one for that kind, wrapping around at the end of the set and restarting from the first address when the pointer is unset or no longer announced. It MUST advance when the assignment is handed out rather than when a result arrives, so a run that is abandoned still moves the node onto its next address.

The staleness gate is per NODE AND KIND while the rotation is per ADDRESS, so a node announcing N addresses has each individual address tested by a given kind roughly every N × that kind's `staleness_age` rather than every `staleness_age`.

#### Scenario: The oldest-tested eligible node is assigned
- **WHEN** an authorised, announced agent requests a testrun and eligible nodes exist
- **THEN** the orchestrator picks a kind and returns the never-tested-or-oldest node for that kind, inserting a `testrun_in_progress` row for it in the same transaction

#### Scenario: Overdue is judged by due time, not by age
- **WHEN** the `mixnode_stress` head was last tested 2 hours 5 minutes ago against a 2-hour interval, and the `mixnode_liveness` head 30 minutes ago against a 15-minute interval
- **THEN** the request is assigned `mixnode_liveness`, which is 15 minutes overdue against stress's 5, although the stress node was tested longer ago

#### Scenario: A cold start begins with liveness
- **WHEN** no kind has yet tested any node
- **THEN** requests are assigned the liveness kinds until neither has a never-tested node left to hand out, and only then `mixnode_stress`

#### Scenario: A kind with nothing to assign is skipped
- **WHEN** the most overdue kind is disabled, has no eligible node, or has its eligible nodes taken by a concurrent request first
- **THEN** the request is assigned the next most overdue kind rather than being answered with no work

#### Scenario: A node already in progress is not reassigned
- **WHEN** a node has an open `testrun_in_progress` row
- **THEN** it is excluded from assignment by every kind until that row is cleared

#### Scenario: A node under one kind of test is not assigned another
- **WHEN** a node has an open `testrun_in_progress` row from a `mixnode_stress` test
- **THEN** it is excluded from both liveness kinds until that row is cleared, and vice versa

#### Scenario: Consecutive runs against one node rotate through its addresses
- **WHEN** a node announcing both an ipv4 and an ipv6 address is assigned by the same kind on two successive occasions
- **THEN** the second assignment targets the other address, so both are exercised in turn rather than whichever one a refresh happened to store

#### Scenario: An abandoned run still advances the rotation
- **WHEN** an assignment is handed out and no result is ever submitted for it
- **THEN** that kind's rotation pointer for the node has already advanced, so its next assignment of the node targets its next address rather than repeating the same one

#### Scenario: Kinds do not disturb each other's rotation
- **WHEN** a liveness test and a stress test are both assigned for one node over time
- **THEN** each kind advances its own rotation pointer, so neither skips addresses because of the other

#### Scenario: A node freed by one kind is immediately available to another
- **WHEN** a node's stress test finishes and its in-flight row clears
- **THEN** it is eligible for a liveness assignment straight away, the per-node lock being the whole of the mutual exclusion between kinds

#### Scenario: A liveness assignment carries a wave
- **WHEN** an agent is assigned liveness work and many nodes are eligible
- **THEN** up to that kind's wave size of targets are returned in one assignment, each with its own in-flight row and lease

#### Scenario: No eligible node yields an empty assignment
- **WHEN** no enabled kind has an eligible node, every node each kind may assign being either in progress or tested by that kind more recently than its `staleness_age`
- **THEN** the agent receives an empty assignment and exits without testing

### Requirement: The orchestrator authorises both of an announcing agent's addresses on-chain in one transaction

An agent SHALL announce a PAIR of mixnet socket addresses, one ipv4 and one ipv6, because a tested node sees whichever family it was reached over as the source of the probe traffic and gates on that source ip; authorising only one family would leave probes over the other rejected. An agent SHALL additionally announce the base58 ed25519 CLIENT IDENTITY public key it will present when opening a gateway client session, derived from its noise key rather than provisioned, so that the gateway session exemption can be keyed on a verified identity instead of a source address.

On `POST /v1/agent/announce` the orchestrator SHALL reject with a 400, before touching any state, an announcement whose addresses are not one plain ipv4 address and one ipv6 address that is not ipv4-mapped. Such a pair MUST NOT be normalised into shape, because an ipv4-mapped ipv6 address collapses onto the ipv4 one when a node canonicalises the authorised set, leaving the agent with a single authorised ingress while both the contract and the orchestrator believe it has two, and because rewriting an address would authorise something the agent never announced and will not use in its sphinx return hop. An announced identity key that is not valid base58 decoding to 32 bytes MUST likewise be rejected before any state is touched, but NOT by a check at that same point: the announce request MUST carry the identity as a typed ed25519 public key rather than an unvalidated string, so the malformed case is rejected during request deserialisation and is unrepresentable by the time the handler runs. The address pair cannot be handled that way because its rule is a relation between two individually well-formed addresses. The consequence is that a malformed identity answers with the deserialisation rejection's status rather than this 400, which is accepted: the property the requirement exists to protect is that nothing is cached and nothing is written on-chain, and that holds more strongly when the handler is never entered at all.

It MUST then upsert the agent into its in-memory `KnownAgents` cache, keyed by the agent's ipv4 mixnet socket address with the ipv6 address and the identity key held inside the entry, and, if the agent was not already announced, MUST authorise BOTH addresses in the network-monitors contract by submitting ONE transaction carrying an `AuthoriseNetworkMonitor` message per address, each with the agent's base58 x25519 noise key, noise version, and identity key, then mark the agent announced. Both authorisations MUST travel in a single transaction so that an agent is never left with only one of its addresses authorised. A contract transaction failure MUST surface as a 500 and leave the agent un-announced; re-announcing is safe because the contract's agent save is an upsert. An agent whose announced noise key, ipv6 address, OR identity key differs from the cached one MUST have its announced flag reset so it is re-authorised, and that divergence SHOULD be surfaced (log plus counter) because a superseded ipv6 address stays authorised in the contract. One counter covering all three is sufficient, and the identity MUST NOT get a series of its own: it is derived from the noise key, so it cannot diverge unless the noise key does, and a dedicated series would read flat zero outside a change to the derivation itself. The log MUST still name which of them diverged, since that is where the distinction is actually needed. This on-chain write is what ultimately causes network nodes to accept the agent's probe connections and to recognise its client sessions.

Because the identity is carried on an OPTIONAL contract field and the save is an upsert keyed by socket address, agents authorised before the field existed acquire it on their next announcement with no backfill step and with no revocation of the old entry: the announcement rewrites both of the agent's entries in place. A cached entry therefore ALWAYS carries an identity, because the rehydration drop rule above leaves an identity-less on-chain entry out of the cache rather than admitting one that no consumer could use.

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
- **THEN** the request is rejected while being deserialised, so the handler never runs and nothing is cached or written on-chain

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

### Requirement: The orchestrator exposes agent and operator HTTP surfaces on separate bearer tokens

The orchestrator SHALL protect its HTTP API with static shared-secret bearer tokens, using two distinct tokens: `agents_token` for the agent surface (`POST /v1/agent/announce`, `POST /v1/agent/request-testrun`, `POST /v1/agent/submit-testrun-result`) and `metrics_and_results_token` for the operator surface (six read-only `GET /v1/results/*` endpoints and `GET /v1/metrics/prometheus`). The root path MUST redirect to Swagger. These bearer tokens authenticate the caller as "a trusted agent" or "a trusted operator"; they are NOT per-agent identities or signatures, and the on-chain allow-set is what actually authorises an agent to probe nodes (enforced at announce-time contract writes and at nym-api ingestion, not at this HTTP layer).

#### Scenario: The agent and operator surfaces require different tokens
- **WHEN** a caller presents the `agents_token` to a `/v1/results/*` endpoint
- **THEN** the request is rejected, because that surface requires the `metrics_and_results_token`

#### Scenario: Bearer auth is not an on-chain authorisation
- **WHEN** an agent presents a valid `agents_token`
- **THEN** it may call the agent endpoints, but whether nodes accept its probes still depends on its on-chain authorisation, not on the token

### Requirement: Stale in-flight dispatches and old results are evicted

The stale-data eviction task SHALL clear `testrun_in_progress` rows whose `expires_at` has passed, so that a dispatch abandoned by a crashed or hung agent frees its node for reassignment, and MUST delete completed testruns older than `testrun_eviction_age` (default 7 days) from every kind's results table. One eviction sweep MUST run before the HTTP server begins serving.

These two duties SHALL run on independent cadences, `lease_release_interval` for releasing expired dispatches and `retention_sweep_interval` for deleting old rows, because their requirements are opposed: a held lease excludes its node from assignment and so must be released well inside the shortest lease budget, whereas deleting rows past a multi-day retention is expensive and need happen only rarely. Neither cadence SHALL be derived from the other, nor from any one kind's lease budget: a single cadence computed from the stress budget leaves an abandoned liveness wave holding its targets for several times its own lease.

The deadline MUST be materialised on the in-progress row at hand-out rather than derived by the sweep from a single global timeout, because different test kinds have different budgets (a stress run is minutes, a liveness wave is seconds) and a future kind may be more expensive still. The sweep therefore requires no knowledge of kinds.

#### Scenario: A timed-out dispatch is released
- **WHEN** a `testrun_in_progress` row's `expires_at` has passed
- **THEN** it is removed and the node becomes eligible for assignment again

#### Scenario: Kinds with different budgets coexist
- **WHEN** a long-budget stress dispatch and a short-budget liveness dispatch are both in flight
- **THEN** each is evicted according to its own deadline, and the short one does not keep the long one alive nor vice versa

#### Scenario: Old results are pruned
- **WHEN** completed testruns are older than `testrun_eviction_age`
- **THEN** they are deleted from their kind's results table

#### Scenario: Releasing leases does not wait on retention
- **WHEN** an abandoned dispatch of the shortest-lease kind passes its deadline between two retention passes
- **THEN** it is released on the next lease-release tick rather than waiting for a retention pass, so the node is excluded from assignment for no longer than one lease-release interval past its deadline

### Requirement: The agent is a one-shot job that announces, requests one assignment, tests, submits, and exits

The `run-agent` path SHALL be a run-to-completion job, NOT a long-lived daemon: it builds an orchestrator client with a bearer token, loads its x25519 noise key, announces itself, requests a single assignment, and - if one is returned - executes it and exits. An assignment is either ONE stress target or a WAVE of liveness targets executed concurrently; in the wave case each target's result MUST be submitted as soon as that target finishes rather than at the end of the wave. When the assignment is empty it MUST log that no work is available and exit without testing. Fleet scale is therefore achieved by running many short-lived agent invocations rather than one persistent process, with liveness sweep throughput coming from wave concurrency within an invocation rather than from a longer-lived process.

The agent MUST be able to execute every test kind the orchestrator may assign, since the orchestrator is the party that chooses. The agent binary MUST also provide `build-info`, a `keygen` subcommand that generates ONLY an x25519 noise key (no ed25519 key), and a `test-node` subcommand that runs a single manual test against an explicitly-specified node bypassing the orchestrator (with no `node_id`).

#### Scenario: An assignment is tested once and submitted
- **WHEN** the agent receives a non-empty assignment
- **THEN** it executes every target it was given, submits each result, and exits

#### Scenario: No work available exits cleanly
- **WHEN** the agent receives an empty assignment
- **THEN** it logs that no work is available and exits without testing or submitting

#### Scenario: A wave's results are submitted as they complete
- **WHEN** one target of a liveness wave finishes while others are still running
- **THEN** its result is submitted immediately rather than being held until the wave ends

### Requirement: The agent authenticates to the orchestrator with a static bearer token and does not sign requests

Every agent-to-orchestrator call SHALL carry the orchestrator's shared `agents_token` as a bearer token. There MUST be no ed25519 (or other) request signing at the agent-to-orchestrator layer; the agent's on-chain identity is its pair of socket addresses plus x25519 noise key, written to the contract by the orchestrator at announce time, not proven per-request to the orchestrator. A corollary is that the orchestrator cannot distinguish an agent re-provisioned with a new noise key from a different agent claiming its ipv4 socket address, so a changed announcement is accepted and surfaced rather than rejected.

#### Scenario: Calls are bearer-authenticated only
- **WHEN** the agent announces, requests work, or submits a result
- **THEN** it authenticates solely with the bearer token and signs nothing

### Requirement: A stress test is a two-hop self-loop probe with a fixed connectivity, bloomfilter, and load sequence

A stress test SHALL route packets over a two-hop path `[tested_node, this_agent]`, so the tested node acts as a forward mix hop relaying each packet straight back to the agent's own listener. Packets MUST be `AckPacket`-sized sphinx packets typed as mix packets, each carrying a 16-byte payload of `{ id: u64, sending_timestamp }`, with per-hop sphinx delay `packet_delay` (default 50 milliseconds). A run MUST proceed through: establishing an outbound Noise egress connection to the node as initiator; spawning an ingress listener that accepts a connection only from an address the tested node is known by and completes a Noise handshake as responder; a connectivity probe (one packet, whose round trip minus `packet_delay` becomes the baseline `approximate_latency`); a bloomfilter probe (only when `reuse_header` is set); the load test; result collection; and teardown. A failure at the egress, connectivity, or bloomfilter step MUST abort the run with the error recorded in the result rather than crashing the agent.

The agent's own hop in the sphinx route MUST carry whichever of its two announced addresses matches the family of the address being tested, so that a run exercises one family in both directions rather than measuring the node's ipv6 ingress against its ipv4 egress.

Ingress acceptance MUST treat EVERY address the node is known by (all of `node_ips`, plus the assigned address) as the node itself, compared on canonical form, because a multi-homed node may return the packets from an address other than the one it was reached on, and because a dual-stack listener reports ipv4 peers in their ipv4-mapped ipv6 form. The tested node's noise key MUST likewise be registered under every one of those addresses, not only the assigned one: the Noise responder handshake looks the initiator up by source ip and silently downgrades to raw TCP on a miss, which the listener then rejects as the node not speaking the protocol. Both gates therefore have to learn the same set, or widening one only moves the failure to the other.

#### Scenario: The tested node loops packets back to the agent
- **WHEN** a stress test runs against a node
- **THEN** each probe packet enters the node as a mix hop and is relayed back to the agent's own ingress listener

#### Scenario: An ipv6 run is measured over ipv6 in both directions
- **WHEN** the assigned address is the node's ipv6 address
- **THEN** the sphinx return hop carries the agent's ipv6 address, so the node returns the packets over ipv6 rather than over ipv4

#### Scenario: A node replying from another of its addresses is still accepted
- **WHEN** the tested node returns the probe packets from an announced address other than the one it was reached on
- **THEN** the agent accepts the connection and completes the Noise handshake, because that address is one it knows the node by and the node's noise key is registered under it

#### Scenario: A node that never returns the connectivity packet aborts the run
- **WHEN** the connectivity probe packet does not come back
- **THEN** the run aborts and the failure is recorded in the result

### Requirement: The load test sends at a fixed rate for a fixed window and deduplicates returns by packet id

The load test SHALL send packets in batches of `sending_batch_size` (default 50), one batch per `batch_interval` (computed as `sending_batch_size / target_rate` seconds), using an interval whose missed-tick behaviour is "delay" so it does NOT burst to catch up. It MUST stop when `sending_duration` (default 30 seconds) elapses or when `expected_packets` (computed as `floor(target_rate * sending_duration)`, `target_rate` default 1000 packets/second) have been sent. Collection MUST drain immediately-available returned packets and then wait up to `waiting_duration` (default 5 seconds) for stragglers. Returned packets MUST be deduplicated by their `id`; a duplicate MUST set a `received_duplicates` flag and log an alarm. The received count MUST be the number of distinct, successfully-decrypted returned packets; undecryptable packets are dropped.

#### Scenario: Sending holds the target rate without bursting
- **WHEN** the sender falls behind its tick schedule
- **THEN** it delays rather than sending a catch-up burst, keeping to the target rate

#### Scenario: Duplicate returns are flagged
- **WHEN** the same packet id is returned more than once
- **THEN** the duplicate is not double-counted, the `received_duplicates` flag is set, and an alarm is logged

### Requirement: `reuse_header` replays one sphinx header to exercise the node's bloomfilter-bypass path

When `reuse_header` is enabled (the default), the agent SHALL pre-build a single sphinx header once, re-derive the per-hop payload keys by replaying the route key exchange, and re-encapsulate a fresh 16-byte payload into that shared header for every packet - deliberately REPLAYING the same header so that it exercises the node's replay/bloomfilter-bypass path for authorised monitors. When `reuse_header` is disabled, the agent MUST build a fresh header and sphinx key per packet. The bloomfilter probe step runs only under `reuse_header`, confirming before the load test that the node is configured to accept the monitor's replayed traffic.

#### Scenario: The default replays one header
- **WHEN** `reuse_header` is enabled
- **THEN** every load-test packet reuses the same sphinx header with a fresh payload, relying on the node's authorised-monitor replay bypass

#### Scenario: The bloomfilter probe validates the bypass first
- **WHEN** `reuse_header` is enabled and the bloomfilter probe packet does not come back
- **THEN** the run aborts before the load test, because the node is not accepting the monitor's replayed header

### Requirement: On success the reported sent count is forced to the expected count so back-pressure is penalised

On the successful completion of the load test the agent SHALL overwrite the reported `packets_sent` with `expected_packets` (`floor(target_rate * sending_duration)`), rather than the number it actually managed to push through. This makes the downstream `received / sent` ratio effectively `received / expected`, so a node that throttled the agent via TCP back-pressure - preventing it from pushing all expected packets - is correctly penalised as if all expected packets had been sent. The per-batch actual-sent counts MUST remain in place for the send-error early-exit path, so partial progress is still visible when a run aborts mid-send.

#### Scenario: A throttling node is scored as if fully loaded
- **WHEN** a node applies TCP back-pressure so the agent sends fewer than `expected_packets`
- **THEN** on success the reported `packets_sent` is set to `expected_packets`, lowering the node's `received / sent` score

#### Scenario: A send error preserves the actual sent count
- **WHEN** the load test aborts on a send error
- **THEN** the last per-batch actual-sent count is reported, not the expected count

### Requirement: The per-node result captures counts, handshake and latency statistics, and an optional error

Each test SHALL produce a result carrying `time_taken`, an optional `error`, and measurements SHAPED BY ITS KIND: one named measurement per interface the kind exercises, so that a result can neither omit an interface its kind requires nor carry one it does not. The kind is the tag of that shape rather than a separate field, so the two cannot disagree. Each measurement MUST carry: ingress and egress Noise-handshake durations; the sphinx packet delay; `packets_sent` and `packets_received`; the baseline `approximate_latency`; the per-packet round-trip distribution (minimum, mean, median, maximum, standard deviation); and a `received_duplicates` flag. The latency of the agent's own batch sends is NOT measured: it describes the agent's socket rather than the node, and nothing read it. A `mixnode_stress` or `mixnode_liveness` run carries exactly one measurement (`mix_forwarding`); a `gateway_liveness` run carries one per phase (`client_ingest` and `client_delivery`). Only a critical failure (for example an inability to bind the ingress listener) MUST bubble up as an error return; node-level failures (no response, bloomfilter misconfiguration, a rejected Noise handshake, a refused client session) MUST be recorded inside the returned result so the orchestrator always receives partial data.

The per-interface breakdown MUST be persisted and exposed on the operator read surface even though downstream consumers receive only the averaged score, because a gateway with a healthy ingress and a dead egress is otherwise indistinguishable from one that is uniformly half-lossy.

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
- **THEN** the ingress and egress measurements are separately visible, alongside the averaged score submitted downstream

#### Scenario: A rejected handshake scores zero rather than being excluded
- **WHEN** a node rejects the agent's Noise handshake, so nothing can be measured
- **THEN** the result records the failure and scores zero, because a node that will not accept a connection is not routable and an unmeasurable node must not score better than a measurably broken one

### Requirement: The subsystem's behaviour is governed by orchestrator and agent configuration surfaces with defined defaults

The orchestrator SHALL be configured with the following defaults: `test_interval` 2 hours, `test_timeout` 5 minutes, `node_refresh_rate` 2 hours, `node_info_query_timeout` 10 seconds, `testrun_eviction_age` 7 days, `number_of_concurrent_node_queries` 32, `chain_authorisation_check_max_attempts` 10, `chain_authorisation_check_retry_delay` 1 minute, `metrics_refresh_interval` 15 seconds, `agent_liveness_window` 15 minutes, `lease_release_interval` 30 seconds, `retention_sweep_interval` 1 hour, and an HTTP bind of `0.0.0.0:8080`; plus required secrets (`agents_token`, `metrics_and_results_token`, the bip39 `mnemonic`, and the base58 ed25519 `private_key`) and required endpoints (`rpc_url`, the mixnet and network-monitors contract addresses, and `database_path`). There is no nym-api endpoint and no result-submission interval or batch size: results reach the performance contract through the materialiser, as `network-monitor-contract-submission` specifies. `metrics_refresh_interval` MUST NOT exceed the deployment's Prometheus scrape interval, or sampled series go stale between scrapes. `lease_release_interval` MUST NOT exceed the shortest configured lease budget, since every lease held past its deadline is a node excluded from assignment; configuration SHALL fail rather than accept a value that cannot satisfy this.

Where a knob governs a per-kind behaviour it MUST be expressible per kind, except that the two liveness kinds share one staleness interval, one lease budget and one enable flag. The orchestrator MUST additionally carry, for the liveness kinds: a staleness interval (defaulting well below the stress `test_interval`, so that liveness tracks v1's cadence) of 15 minutes, a lease budget used as the in-progress `expires_at` (which MUST bound one concurrent wave, not the sum over its targets, and MUST therefore cover the slower of the two probes) of 1 minute, a wave size PER KIND - 100 for `mixnode_liveness` and 50 for `gateway_liveness`, since a wave is one concurrent batch and a gateway target costs the agent a live client session where a mixnode target costs a Noise connection - and an enable flag allowing both liveness kinds to be switched off without a code change, i.e. by configuration and a restart rather than by a build, defaulting to ON and therefore expressible as an explicit value rather than as a bare presence flag, since a flag that can only be switched on cannot be switched off by a deployment. `test_interval` and `test_timeout` remain the staleness interval and lease budget of `mixnode_stress`.

The agent SHALL be configured with the following defaults: `sending_duration` 30 seconds, `waiting_duration` 5 seconds, `packet_delay` 50 milliseconds (which MUST be non-zero), `target_rate` 1000 packets/second, `reuse_header` true, `egress_connection_timeout` 5 seconds, `noise_handshake_timeout` 3 seconds, `sending_batch_size` 50, and a listener bind of `[::]:9000`; plus the required orchestrator URL, orchestrator bearer token, announced ipv4 host address, announced ipv6 host address, shared announced port, and noise-key path. The agent MUST additionally carry a liveness profile: a per-target packet count, a PER-TARGET send rate (never an aggregate budget divided across the wave), a straggler wait, and a per-target timeout. All knobs MUST be overridable by CLI flag or environment variable.

The liveness profile's initial values are PROVISIONAL, chosen for score granularity rather than measured against agent hardware: a per-target packet count of 50 (giving 2% granularity against v1's three packets per route), a per-target rate of 50 packets/second, a straggler wait of 5 seconds, a per-target timeout of 30 seconds, and wave sizes of 100 mixnode targets and 50 gateway targets. The rate is well below the stress profile's 1000 packets/second, so no target is under load, and it puts a target's send window at about one second, which is as long as a probe asking whether a node forwards at all has any reason to hold one open. A full mixnode wave therefore emits 5000 packets/second for roughly that second, about 15 Mbps at the probe's `AckPacket` sizing, while totalling 5000 packets against a stress run's 30000. The packet count is sized for the interpretability of a SINGLE run on the operator read surface, not for statistical power, which comes from the cadence: at a 15-minute interval a node contributes on the order of a hundred runs to each daily aggregation window. Because they are provisional, every one of them MUST be tunable in a deployment without a code change, and no behaviour may depend on a specific value: an agent host that cannot sustain a full wave MUST be correctable by configuration alone.

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

#### Scenario: Liveness can be disabled without a new build
- **WHEN** the orchestrator's liveness enable flag is unset
- **THEN** neither liveness kind is assigned and stress testing continues unaffected

### Requirement: Liveness is tested by two kinds of its own, and the mixnode probe is the stress probe at a low-volume profile

The subsystem SHALL support liveness testing alongside stress testing. A test kind is ONE probe against ONE role, and the kinds are `mixnode_liveness`, `gateway_liveness` and `mixnode_stress`; "liveness" names the family of the first two rather than a kind of its own. Every unit of work MUST be identified by exactly one kind, and the orchestrator MUST be the party that decides which kind a given assignment carries; an agent MUST support every kind.

For a node reporting the mixnode role, a liveness probe SHALL be the same two-hop self-loop probe as a stress test - route `[tested_node, this_agent]`, `AckPacket`-sized mix packets, the same connectivity and bloomfilter probe sequence, the same `reuse_header` behaviour - executed under its own low-volume profile (`liveness_packets`, `liveness_target_rate`, `liveness_waiting_duration`) rather than the stress profile. The reported sent count MUST be forced to the profile's expected packet count on success, exactly as for a stress test, so a node that applies back-pressure to a liveness probe is penalised rather than flattered.

A liveness probe MUST be PACED at its configured rate rather than dispatched as a burst, because a burst applies exactly the load the kind exists to avoid applying. Since the inter-batch interval is the batch size divided by the rate, a batch that is large relative to the packet count collapses the send window into a couple of dispatches seconds apart, which is what inheriting the stress batch size would do. The liveness batch size MUST therefore be DERIVED from the per-target packet count, as a fixed number of dispatches spread across the send window, and MUST NOT be inherited from the stress profile or configured on its own, so that retuning the provisional count or rate cannot silently reintroduce bursting. At the profile's own values the interval between dispatches MUST remain on a millisecond scale.

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

A gateway liveness run MUST produce a measurement for each phase. The score denominator MUST be fixed by the test kind at two measurements, so a phase that produces no measurement scores zero rather than being excluded from the average. A phase-1 failure MUST NOT abort the run; only failure to establish the client session aborts it, in which case both measurements are zero. Neither phase relies on `forward_hop_processing_enabled`, which is derived from a node's mixnode mode and is disabled on a gateway-only node.

#### Scenario: Both phases run in one session by one agent
- **WHEN** a gateway liveness assignment is handed out
- **THEN** a single agent establishes one client session and performs both the ingress and egress phases within it, and no other agent is ever assigned either phase of that run

#### Scenario: A dead egress path is not hidden by a healthy ingress path
- **WHEN** the ingress phase returns every packet and the egress phase returns none
- **THEN** the run records one full and one zero measurement, and the reported score is their average over the kind's fixed two-measurement denominator

#### Scenario: An ingress failure still yields an egress measurement
- **WHEN** the gateway fails to forward the ingress phase's packets
- **THEN** the run continues into the egress phase and records the ingress measurement as zero, rather than aborting

#### Scenario: A session that cannot be established scores zero on both phases
- **WHEN** the agent cannot establish its client session with the gateway
- **THEN** the run aborts with the failure recorded and both measurements are zero

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

To execute a wave the agent MUST bind ONE shared ingress listener and MUST treat the union of every target's announced addresses, canonicalised, as its known-source set, so that any target's return connection is accepted from any address that target is known by. The Noise view the responder is given MUST be scoped to the single source of the connection being upgraded rather than spanning the wave: the responder consults it only as a membership gate, falling back to plain TCP on a miss, and authenticates using the agent's own key rather than the initiator's, so a wave-spanning view would be the known-source set held a second time. The initiator side is where a target's noise key is actually required, and there each target's egress connection is established against its own. Returned packets MUST be attributed to a target by the source address of the connection they arrive on, which is sound because node ip addresses are unique across the node population (unlike agent addresses, which may share an ip and be disambiguated only by port). The union is collision-free for the same reason.

The send rate MUST be configured PER TARGET and MUST NOT depend on the wave width, so that a node is probed at the same rate whether its wave held three targets or a hundred and its delivery ratio is comparable across waves. A rate derived by dividing an aggregate budget across the received wave MUST NOT be used, because it would probe a node in a narrow wave far harder than the same node in a full one, and liveness measures delivery rather than throughput.

It follows that the agent's aggregate egress is the per-target rate times the wave width, and so is bounded by the orchestrator's wave size rather than by any agent-side knob. The per-target rate MUST stay far enough below the stress profile's that no individual target is placed under load, but a full wave's aggregate rate is NOT required to stay within one stress run's. A liveness probe sends orders of magnitude fewer packets per target than a stress test and finishes in about a second, so a full wave is a FRACTION of a stress run's volume even while briefly exceeding its instantaneous rate; volume is what the cost bound is expressed in, because the wave width the rate is multiplied by is the orchestrator's knob rather than the agent's.

Every result MUST be submitted as soon as its own target completes, rather than at the end of the wave, so that each target's in-flight lock is released independently and an agent that dies mid-wave loses only the targets it had not yet reported.

#### Scenario: A wave's duration is bounded by its slowest target, not their sum
- **WHEN** a wave contains several unresponsive targets
- **THEN** they time out concurrently, so the wave completes within roughly one target's worst case rather than the sum of them

#### Scenario: Returned packets are attributed to the right target
- **WHEN** several targets in one wave return packets to the shared listener at the same time
- **THEN** each packet is attributed to the target whose address the connection originated from

#### Scenario: A node is probed at the same rate in a narrow wave as in a full one
- **WHEN** one wave carries three targets and another carries a hundred
- **THEN** every target in both is probed at the configured per-target rate, so the two waves' delivery ratios are directly comparable

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

### Requirement: Each test kind defines which nodes it assigns and how their results are typed

Each test kind SHALL declare the role a node must report to be eligible for it. `mixnode_stress` and `mixnode_liveness` MUST assign only nodes reporting the mixnode role, and `gateway_liveness` MUST assign only nodes reporting the entry-gateway role. A dual-role node, reporting both, is therefore eligible for all three kinds, each with its own staleness, and its liveness score MUST be the average over the runs of both liveness kinds.

A node with no description, because it has never been completely described or is no longer bonded, MUST be ineligible for every kind, as MUST a described node that reports no roles.

#### Scenario: A gateway-only node is assignable for gateway liveness only
- **WHEN** the orchestrator selects work for a gateway-only node
- **THEN** it may assign `gateway_liveness` and never `mixnode_liveness` or `mixnode_stress`

#### Scenario: A dual-role node is measured in both roles
- **WHEN** a dual-role node is due for liveness
- **THEN** it is assigned both `mixnode_liveness` and `gateway_liveness`, and its score averages the runs of both

#### Scenario: An undescribed node is never assigned
- **WHEN** a node has never answered its self-description completely
- **THEN** no test of any kind is assigned to it

#### Scenario: An unbonded node is no longer assigned
- **WHEN** a node disappears from the mixnet contract's bonds
- **THEN** its description is deleted at the next successful refresh and no test of any kind is assigned to it

### Requirement: Orchestrator state is a per-kind SQLite schema and the agent registry is in-memory only

The orchestrator SHALL persist state in a SQLite database of: `nym_node_bond`, the bonded-node registry (node id, identity key, and when the node was last seen bonded); `nym_node_description`, each node's latest complete self-description (mix port, announced address set, noise key, sphinx key and key-rotation id, the mixnode and entry-gateway role flags as the node reports them, and plain client websocket port), at most one per bonded node; a work-state table keyed `(node_id, test_kind)` holding that kind's last-tested timestamp and address rotation pointer; one results table PER KIND (`mixnode_liveness_testrun`, `gateway_liveness_testrun`, `mixnode_stress_testrun`), each row recording the node, the address that was tested, the run's timing and error, and one column group per interface that kind exercises; and `testrun_in_progress` (the in-flight dispatch lock set, keyed by `node_id` alone so that only one test of any kind runs against a node at a time, and carrying a materialised `expires_at` plus the kind it was dispatched for). The submission-watermark table of the former nym-api result stream SHALL be dropped by a migration.

The set of kinds MUST be listed once, in a `test_kind` table keyed by the kind's name that every kind column references, so that adding a kind inserts a row rather than rebuilding each table naming one, SQLite being unable to alter a `CHECK` constraint.

Every column of a description MUST be non-null except the client websocket port, which MUST be null exactly when the node does not report the entry-gateway role. The roles MUST be stored as the two flags the node reports rather than folded into a combined classification, since every reader asks about one role at a time. A node therefore has either a complete description or none, so no consumer has to reason about a partially described node, and eligibility for every kind requires one. Results and work state MUST reference the bond rather than the description, so deleting a description loses neither.

Each kind's results table MUST carry the same column group for every interface it exercises: one `mix_forwarding` group for the two mixnode kinds, and a `client_ingest` and a `client_delivery` group for `gateway_liveness`. A result MUST be stored only under the kind recorded on its in-flight row, and a result shaped for any other kind MUST be rejected; because a result carries exactly one measurement per interface its kind exercises, every column group is written from the submission and none can be missing or extra. A testrun id is unique only within its kind, so the operator read surface MUST address a completed run by its kind as well as its id.

Because each liveness role is a kind of its own, a dual-role node has separate work state for `mixnode_liveness` and `gateway_liveness`, and one kind's run never advances the timestamp that gates the other's eligibility. The dispatched kind MUST be recorded on the in-flight row, because the submission reports only the node and the address, so the orchestrator would otherwise depend on the agent echoing back a value the orchestrator itself chose in order to know which table the result belongs in.

The per-kind last-tested timestamp MUST be stored directly rather than read through a join onto the last testrun row, so that evicting an old result does not make a node read as never-tested and jump the assignment queue.

The agent registry MUST NOT be persisted; it lives only in the in-memory `KnownAgents` cache and is rebuilt from the contract on each startup, which means agents' announced flags reset across a restart and each agent re-announces (and is re-authorised on-chain) on its next run.

Rehydrating that cache from the contract requires recovering which pair of on-chain entries belongs to one agent. The contract stores one entry per socket address and carries no field linking an agent's two addresses, so the orchestrator MUST group the entries by their x25519 noise key, which is unique per agent (see the network-monitors-contract capability, which does NOT enforce that uniqueness). Entries that do not form exactly one ipv4/ipv6 pair MUST be dropped from the cache rather than guessed at - they are either authorisations predating the paired announcement or leftovers from an agent that has since changed an address - which is safe precisely because the cache only exists to skip redundant contract transactions, and an agent always announces before requesting work. The identity key is subject to the same rule: a pair whose two entries do not both carry the SAME identity MUST be dropped, an absent one being an authorisation predating the field and a disagreeing one being a half-written pair. Dropping rather than tolerating is what keeps every cached entry complete, so no consumer has to reason about a rehydrated agent whose identity is unknown, and it costs only the one redundant authorisation transaction the drop rule already accepts.

#### Scenario: Each kind keeps its own staleness and rotation position
- **WHEN** a node has been tested by several kinds
- **THEN** the work-state table holds one row per kind, each with its own last-tested timestamp and rotation pointer

#### Scenario: A dual-role node is not starved of either liveness probe
- **WHEN** a dual-role node has just been tested by `mixnode_liveness`
- **THEN** its `gateway_liveness` eligibility is unaffected, because that kind has its own last-tested timestamp

#### Scenario: Evicting an old result does not reset staleness
- **WHEN** a node's last completed testrun is deleted by result eviction
- **THEN** the node's per-kind last-tested timestamp is unchanged, so it does not read as never-tested

#### Scenario: A gateway description always carries its client port
- **WHEN** a gateway-capable node's description is stored
- **THEN** it carries the plain client websocket port, and a gateway-capable description without one cannot be written

#### Scenario: A result of the wrong kind is rejected
- **WHEN** a node dispatched for `mixnode_liveness` submits a result shaped for `gateway_liveness`
- **THEN** the submission is rejected and nothing is stored

#### Scenario: An agent's two on-chain entries are re-paired after a restart
- **WHEN** the orchestrator restarts and reads its agents from the contract
- **THEN** the ipv4 and ipv6 entries sharing one noise key are rehydrated as a single announced agent

#### Scenario: An unpairable on-chain entry is dropped rather than guessed
- **WHEN** an agent has an on-chain entry with no counterpart of the other family under the same noise key
- **THEN** it is left out of the rehydrated cache and re-created by that agent's next announcement, at the cost of one redundant authorisation transaction

#### Scenario: An on-chain pair without one identity is dropped rather than half-restored
- **WHEN** an agent's on-chain entries were written before the identity field existed, or carry identities that differ from one another
- **THEN** the pair is left out of the rehydrated cache and re-created, complete with its identity, by that agent's next announcement, with no revocation of the old entries because the authorisation save overwrites them in place

#### Scenario: Node registry and results survive a restart
- **WHEN** the orchestrator restarts
- **THEN** its node bonds and descriptions, per-kind work state and per-kind completed testruns are loaded from SQLite

#### Scenario: The agent set is rebuilt from the contract, not from disk
- **WHEN** the orchestrator restarts
- **THEN** its agent registry is rehydrated from the network-monitors contract rather than read from local storage

### Requirement: The orchestrator runs six long-lived tasks and rehydrates its agent set from the contract at startup

On a successful startup the orchestrator SHALL rebuild its in-memory agent registry from the contract (all agents whose `authorised_by` equals its own address) and then spawn exactly six long-lived tasks under a shutdown manager: an HTTP server (serves continuously); a node refresher on `node_refresh_rate` (default 2 hours, first tick immediate); a stale-data eviction task driving two independent timers, `lease_release_interval` (default 30 seconds) and `retention_sweep_interval` (default 1 hour); a chain-capability refresher, waking every 5 minutes (first tick immediate) to re-query the nodes whose cached on-chain standing is missing or due; a materialiser, whose first pass runs immediately and each later pass when the epoch in progress is due to end, or after a one-minute floor when the previous pass failed, the interval could not be read, or the epoch is overdue; and a scheduling-telemetry task on `metrics_refresh_interval` (default 15 seconds, first tick immediate). Submission to the performance contract, when on, runs as the last step of the materialiser's pass rather than as a task of its own; there is no result submitter to nym-api. The eviction task MUST additionally run one blocking sweep before serving, to release in-progress rows left behind by a prior crash.

#### Scenario: The full task set is spawned after gating
- **WHEN** an authorised orchestrator finishes its startup gates
- **THEN** it rehydrates its agent set from the contract and spawns the HTTP server, node refresher, stale-data eviction, chain-capability refresher, materialiser, and scheduling-telemetry tasks

#### Scenario: A prior crash's in-flight rows are released on boot
- **WHEN** the orchestrator restarts with `testrun_in_progress` rows left over from a previous run
- **THEN** one blocking eviction sweep runs before serving to clear timed-out in-progress dispatches

#### Scenario: Telemetry does not depend on the eviction cadence
- **WHEN** the scheduling-telemetry task and the stale-data eviction task are both running
- **THEN** the telemetry task refreshes its series on its own interval, which is shorter than the shortest lease budget, rather than being driven by the eviction sweep

### Requirement: Scheduling telemetry is published per kind from the scheduler's own predicate

The orchestrator SHALL publish, per test kind, the number of nodes due and not in flight, the number in flight, the number in flight whose lease has already expired, how long the longest-running dispatch has been outstanding, how overdue the oldest due node is, the number of nodes it has never tested, and the number of nodes that kind can test. The due count SHALL be produced by the same eligibility predicate the assignment path uses, counted rather than limited, so it cannot report a population the assignment would not have drawn from. A kind holding nothing in flight SHALL report an outstanding age of zero, so that an idle kind is not confused with one holding a very old dispatch.

The overdue figure SHALL be taken over nodes the kind has measured at least once, and nodes it has never measured SHALL be reported only as a count. A node with no result has no due time to be overdue against, and the assignment order places never-tested nodes ahead of every measured one, so folding them in would let a single newly bonded node own the head indefinitely and leave the overdue figure undefined for as long as the network keeps gaining nodes. Kept apart, the overdue figure answers whether the cadence is slipping and the count answers whether new nodes are being picked up, which are different failures with different responses.

#### Scenario: The due count agrees with what an assignment would draw
- **WHEN** telemetry reports a due count for a kind and an agent then requests work for that kind
- **THEN** the nodes the assignment locks are drawn from the same set the count was taken over, because both apply one predicate

#### Scenario: Expired leases are distinguishable from live ones
- **WHEN** in-flight rows exist whose `expires_at` has passed but which the eviction sweep has not yet reaped
- **THEN** they are counted both as in flight and as expired, so a consumer can tell a fleet busy with live work from one holding locks for abandoned runs

#### Scenario: A saturated kind does not read as an idle one
- **WHEN** every node due for a kind is currently locked in flight, so the kind has nothing assignable
- **THEN** its due count is zero while its in-flight count is positive, which together report saturation rather than idleness

#### Scenario: A wave running long is visible before its lease expires
- **WHEN** a dispatch has been outstanding for most of its lease but has not yet passed its deadline
- **THEN** the oldest outstanding age for its kind reflects it while its expired count stays zero, so narrowing headroom against the lease is observable without waiting for the dispatch to expire

#### Scenario: A newly bonded node does not make the overdue figure meaningless
- **WHEN** a node is described for the first time and a kind has never tested it, while every node that kind has measured is comfortably within its cadence
- **THEN** the overdue figure stays low and the never-tested count rises by one, rather than the new node becoming the head and leaving the overdue figure undefined

#### Scenario: New nodes going unpicked is visible on its own
- **WHEN** nodes have been described but a kind has not tested any of them for an extended period
- **THEN** its never-tested count stays elevated, which is a distinct failure from the cadence slipping on nodes already being measured

### Requirement: Utilisation is published as accumulated pending time rather than a computed ratio

For each kind the orchestrator SHALL maintain a monotonic counter of elapsed time during which that kind had work pending or in flight, advancing it by measured elapsed time on each telemetry tick. It SHALL NOT publish a pre-averaged utilisation ratio.

#### Scenario: The counter advances only while work exists

- **WHEN** a kind has neither due nor in-flight work across several telemetry ticks
- **THEN** its pending-time counter does not advance over those ticks, so the idle interval is recoverable as a rate over any window

#### Scenario: Utilisation survives a restart

- **WHEN** the orchestrator restarts and the counter resets to zero
- **THEN** a rate taken over the counter still reports utilisation correctly across the restart, because a counter reset is handled by the query rather than reported as an idle fleet

### Requirement: The orchestrator publishes the required assignment rate rather than the configuration behind it

The orchestrator SHALL publish, per kind, the assignment rate required to hold that kind's cadence, computed as its testable population over its configured staleness interval over its configured wave size cap. The wave size used SHALL be the configured cap rather than an observed average, because an observed average falls below the cap exactly when a kind's queue has drained and capacity is most plentiful, which would overstate the requirement precisely when there is most slack.

The orchestrator SHALL NOT publish configuration values as telemetry in their own right. A value that a consumer can read from configuration or environment is not an observation, and republishing it makes the process restate its own input.

#### Scenario: A required rate is available without a consumer knowing the configuration

- **WHEN** a consumer computes how far the fleet is from meeting a kind's requirement
- **THEN** it divides the published required rate by the observed work-request rate, carrying no cadence or wave size of its own

#### Scenario: A cadence change is reflected without consumer changes

- **WHEN** a kind's staleness interval is reconfigured and the orchestrator restarts
- **THEN** the published required rate reflects the new interval, and consumers dividing by it follow without being edited

### Requirement: The capacity denominator counts testable nodes, with exclusions published separately

The per-kind population SHALL count only nodes the scheduler can assign, meaning bonded, described and carrying that kind's role. The orchestrator SHALL separately publish the number of bonded nodes with no usable description, so nodes that are absent from every population remain visible.

#### Scenario: An undescribed bonded node is excluded and counted

- **WHEN** a node is bonded but has never answered a refresh with a usable description
- **THEN** it is absent from every kind's population and is counted among the excluded nodes

### Requirement: Agent fleet size is reported from recent activity, not from the announcement cache

The orchestrator SHALL record, against each agent it knows, when that agent last requested work, stamping it where the request handler already resolves the caller, and SHALL publish the number of agents whose last request falls within `agent_liveness_window`. The previously published counts of known and announced agents SHALL be removed, because the agent cache is seeded from every agent ever authorised, is keyed on an announced address that changes when an agent restarts, and is never pruned, so it counts addresses ever seen rather than agents currently working.

This count exists to reveal partial fleet loss, not to size the fleet. Sizing follows from required throughput against the observed request rate and uses no count, and a count cannot distinguish adding agents from invoking existing ones more often. What it uniquely shows is a loss that no throughput signal reveals, because work is pulled rather than pushed.

#### Scenario: Losing part of the fleet is visible before it hurts

- **WHEN** some agents stop requesting work while the remainder continue, and the remainder still has capacity to absorb the whole population within its cadence
- **THEN** the recently-seen count falls, even though throughput, pending counts and head lateness are all unchanged, because the surviving agents simply take more work per request

#### Scenario: An agent that stops asking for work leaves the count

- **WHEN** an agent has made no work request for longer than `agent_liveness_window`
- **THEN** it is no longer counted among recently seen agents, even though it remains in the cache and remains authorised in the contract

#### Scenario: A restarted agent on a new address is not double counted

- **WHEN** an agent restarts and announces a different address, creating a second cache entry
- **THEN** only the entry that has requested work within the window is counted, so the fleet size does not grow with restarts

### Requirement: A stalled telemetry task is distinguishable from an idle fleet

The orchestrator SHALL publish a monotonic count of completed telemetry ticks.

#### Scenario: A stopped task is detectable

- **WHEN** the telemetry task stops running while the orchestrator continues to serve
- **THEN** the tick count stops advancing, distinguishing a stalled task from a fleet that genuinely has no pending work, which would otherwise look identical

### Requirement: Per-kind telemetry is carried as a label rather than as one metric name per kind

Series that differ only by the test kind, the node role, or the outcome of one operation SHALL be published as a single metric family carrying that dimension as a label. A series that totals an operation SHALL remain distinct from the family that breaks that operation down by outcome, so that a breakdown is never aggregated together with its own total.

#### Scenario: One family covers every kind

- **WHEN** a consumer queries assignments, in-flight counts, expired leases or wave sizes
- **THEN** each is one metric family whose series are distinguished by a kind label, and an aggregate across kinds requires no enumeration of metric names

#### Scenario: A new kind needs no new metric name

- **WHEN** a further test kind is introduced
- **THEN** it appears as an additional label value on the existing families, without a new metric name and without changes to consumers that aggregate across the label

#### Scenario: Run duration is readable per kind

- **WHEN** test-run durations are queried for a single kind
- **THEN** that kind's distribution is recoverable from the shared duration family, whose buckets span both the liveness lease and the longer stress lease, rather than being merged with every other kind's durations

### Requirement: Every nym-api reads the performance contract for its snapshots

Every nym-api SHALL read the performance contract, producing attested performance snapshots as `performance-attestation-provider` specifies, whatever its configuration. No setting SHALL decide whether nym-api reads the contract, and it MUST NOT be inferred from the network details carrying the address: the address is assumed present, and network details that do not carry it MUST fail startup. No nym-api holds a window of the contract's per-epoch medians, rewards, or selects the active set: those are `nym-epoch-driver`'s. nym-api computes no performance figure of its own and has no mode: its annotations are built from the attested snapshots as "nym-api publishes the verified routing figure as each node's performance" specifies.

The `[performance_provider]`, `[rewarding]` and `[network_monitor]` sections SHALL no longer exist. A configuration that still carries any of them MUST load, their contents being ignored.

#### Scenario: A missing address fails startup
- **WHEN** the network details carry no performance contract address
- **THEN** startup fails, whatever the configuration

#### Scenario: No nym-api holds a window
- **WHEN** any nym-api runs
- **THEN** it produces performance snapshots, holds no window, reads no per-epoch medians for one, and sends no epoch transaction

#### Scenario: A removed section does not break a configuration
- **WHEN** a configuration still carries `[performance_provider]`, `[rewarding]` or `[network_monitor]`
- **THEN** it loads and the sections have no effect

### Requirement: nym-api publishes the verified routing figure as each node's performance

nym-api SHALL read each node's published performance per request from the latest settled performance snapshot (`performance-attestation-provider`), with no cache of its own between them: a node in that snapshot's bond set with a committed entry is published at the entry's committed score, or 0 when the entry has none, and a bonded node without a committed entry is published at 0. The topology routes publish that figure alone, as its nearest whole percent, so every one of them publishes the value the contract's digest committed at that snapshot's height, which nym-api verified before signing it, to that precision, from the moment the snapshot settles. Every client fetches the topology routes, so finding a node's committed entry MUST take constant time: the snapshot holds an index of its bonded entries, built whenever the snapshot is produced or loaded from disk and never saved with it. The per-node annotation routes SHALL build the annotation of the node asked for from the same entry: its `performance_score` as above, its current role the node's role in the current rewarded set, and no chain capabilities; a node without a committed entry has no annotation. No describe-based filter applies: whether a node is alive is the network monitor's measurement. The published figure MUST NOT be the rewarding figure, which differs from the routing figure only after a partial measurement gap longer than the contract's 24-epoch lookback or in the minutes after a weights update, and which is on chain through `RewardingScore`.

The remaining performance fields are served only by the per-node annotation and performance routes, and MAY therefore carry values the digest does not commit. The config, stress and liveness scores SHALL come from the medians of the entry's bundle that the producer holds beside that snapshot (`performance-attestation-provider`), each 0 when the kind has no median, with the stress and liveness reachability flags set exactly when their median is present. An entry from an epoch older than the one before the newest among the snapshot's entries has no medians, and its sub-scores SHALL keep their defaults. The config score's breakdown is not in the contract and SHALL be inferred from the score, since a node scores 0 unless it is described, runs nym-node and accepted the terms, and 1 only with no versions behind: a positive score SHALL mark the node described, running nym-node and having accepted the terms, and a score of 1 SHALL give 0 versions behind, the versions behind being absent otherwise; a score of 0, or no config median, SHALL leave the config score at the shape served for a node nym-api cannot describe. The config score's chain capabilities and the routing score SHALL keep their defaults. An entry without a score, a bundle the contract cannot score (no config median, or only zero-weighted kinds measured), SHALL be annotated with `performance_score` 0 and the sub-scores of its medians.

The freshness timestamps the topology routes report SHALL take the snapshot's block time where they took the annotations' refresh time. While no performance snapshot is held, the routes that read one MUST answer 503; nym-api's startup waits for one to settle, so this happens only once the snapshots have stopped settling for longer than they are retained.

#### Scenario: Published performance is the committed score
- **WHEN** node 7's committed entry in the latest settled snapshot is from epoch 10 with score 0.85
- **THEN** its annotation's `performance_score` is 0.85, and the skimmed routes publish it at 85%

#### Scenario: Published performance is the nearest whole percent
- **WHEN** a node's committed score is 0.29
- **THEN** the skimmed routes publish it at 29%

#### Scenario: Sub-scores come from the entry's bundle medians
- **WHEN** node 7's entry is from the newest epoch among the snapshot's entries, and its bundle there has liveness 0.9, stress 0.8 and config 0.95 medians
- **THEN** its annotation carries liveness 0.9 and stress 0.8 with both reachability flags set, config 0.95 marking it described, running nym-node and having accepted the terms with no versions-behind figure, and the routing score at its default

#### Scenario: A full config score means no versions behind
- **WHEN** a node's bundle has a config median of 1
- **THEN** its annotation's config score is 1 with 0 versions behind

#### Scenario: A gateway's stress score is unset
- **WHEN** a gateway's bundle has liveness and config medians and no stress median
- **THEN** its stress score is 0 with the reachability flag unset

#### Scenario: An entry from an older epoch keeps default sub-scores
- **WHEN** node 8's committed entry is from epoch 80 with score 0.7, and the newest epoch among the snapshot's entries is 100
- **THEN** its `performance_score` is 0.7 and its config, stress, liveness and routing scores keep their defaults

#### Scenario: A node never measured has no annotation
- **WHEN** a node is in the snapshot's bond set but has no committed entry
- **THEN** it has no annotation and is served 0

#### Scenario: An unscoreable entry is published at zero
- **WHEN** a node's committed entry has no score
- **THEN** its annotation's `performance_score` is 0 and it carries its bundle's medians as sub-scores

#### Scenario: An undescribed node is still published
- **WHEN** a node nym-api cannot describe has a committed entry with a score
- **THEN** it is annotated with that score

#### Scenario: A newly settled snapshot is published at once
- **WHEN** a newer performance snapshot settles with node 7's score changed from 0.85 to 0.9
- **THEN** the next request to a topology route publishes node 7 at 90%, with no refresh in between

#### Scenario: A loaded snapshot is indexed
- **WHEN** nym-api restarts and loads its retained snapshots from disk
- **THEN** each node's committed entry is found through the rebuilt index, the saved file carrying none

#### Scenario: No held snapshot answers 503
- **WHEN** a topology or per-node annotation route is requested while no performance snapshot is held
- **THEN** it answers 503

### Requirement: nym-api keeps every monitor-era GET route mounted and deprecated over frozen data

Every GET route nym-api served before the v1 monitor's removal SHALL stay mounted, marked deprecated in the OpenAPI document and on its handler, with its documentation saying that the data behind it no longer changes. The `/v1/status` monitor routes (per-node histories and core-status counts, the unstable per-node test results and the run details) SHALL be mounted unconditionally, there being no monitor flag, and SHALL read the v1 monitor's tables, which nothing writes any more and whose rows are never purged, so each answers what it answered at the cutover. The three `/v1/nym-nodes/*-history` routes SHALL read the historical-uptime tables the same way, a date query for a day after the cutover answering null. The POST routes that fed the storage (`submit-node-monitoring-results`, `submit-gateway-monitoring-results`, the stress and liveness `batch-submit`) SHALL NOT exist. Four GET routes are removed by explicit exception, none of them serving data that could be frozen: `/v3/nym-nodes/stress-testing/known-monitors/{identity_key}`, which answered only whether this nym-api would accept an orchestrator's submissions and whose one reader was the orchestrator's startup check; `/v3/unstable/nym-nodes/liveness-divergence` and `/v3/unstable/nym-nodes/contract-divergence`, temporary gauges built under `/unstable` to be removed once they had informed the weighting of liveness and the switch to the performance contract, both of which have happened; and `/v1/utility/node-annotations-cache-timestamp`, reachable only with the operator's bearer token and named after an annotation cache that no longer exists, the topology routes' freshness timestamps publishing the snapshot block time it would answer. Only the `*_v2` tables and the two v3 result tables, which no GET route reads, are dropped.

#### Scenario: A history route serves the frozen rows
- **WHEN** `/v1/nym-nodes/uptime-history/{node_id}` is requested for a node that had rows before the cutover
- **THEN** it answers those rows, unchanged from the cutover, and the route is marked deprecated in the OpenAPI document

#### Scenario: A status route no longer needs the monitor flag
- **WHEN** `/v1/status/mixnode/{mix_id}/core-status-count` is requested on any nym-api
- **THEN** it is mounted and answers the counts over the last runs recorded before the cutover

#### Scenario: The divergence routes are gone
- **WHEN** `/v3/unstable/nym-nodes/contract-divergence` or `/v3/unstable/nym-nodes/liveness-divergence` is requested
- **THEN** the route does not exist

#### Scenario: A submission route is gone
- **WHEN** a POST is made to `/v3/nym-nodes/stress-testing/batch-submit`
- **THEN** the route does not exist and nothing is stored

#### Scenario: The known-monitors route is gone
- **WHEN** `/v3/nym-nodes/stress-testing/known-monitors/{identity_key}` is requested
- **THEN** the route does not exist

#### Scenario: The annotation cache timestamp route is gone
- **WHEN** `/v1/utility/node-annotations-cache-timestamp` is requested with the operator's bearer token
- **THEN** the route does not exist

