# nym-network-monitor Specification

## Purpose
The v3 mixnode stress-testing subsystem: chain-authorised, one-shot agents drive high-rate looped-back sphinx traffic through individual mixnodes, an orchestrator assigns work and submits signed result batches to nym-api, nym-nodes gate probe acceptance (and the replay/bloomfilter bypass) on the on-chain authorisation, and nym-api folds the results into node-performance scoring and rewarding. This capability covers the orchestrator and agent lifecycles, the on-chain agent authorisation and its node-side propagation, the probe mechanics and scoring, signed submission and nym-api ingest, the downstream consumer surface, and the configuration surface. It is distinct from the nym-api-internal v1 monitor (`network-monitor`) and the unused legacy v2 crate.

## Requirements
### Requirement: The orchestrator gates startup on chain balance, contract authorisation, identity-key reconciliation, and nym-api recognition

Each network-monitor orchestrator SHALL be an independent daemon holding an ed25519 identity keypair and a Nyx (bip39) account. On startup it MUST complete the following gates in order before serving, aborting startup on any failure: initialise its SQLite database; build a direct-signing nyxd client from its mnemonic; verify its signing account holds at least `1_000_000 unym` (to cover its on-chain transaction fees); verify its bech32 address is present in the network-monitors contract's authorised-orchestrator set, retrying up to `chain_authorisation_check_max_attempts` (default 10) spaced by `chain_authorisation_check_retry_delay` (default 1 minute); reconcile its announced identity key, submitting an `UpdateOrchestratorIdentityKey` transaction and sleeping 30 seconds when the on-chain key differs from (or is missing versus) its local key; and verify nym-api recognises it as authorised via the `known-monitors` self-check endpoint.

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

### Requirement: The orchestrator runs four long-lived tasks and rehydrates its agent set from the contract at startup

On a successful startup the orchestrator SHALL rebuild its in-memory agent registry from the contract (all agents whose `authorised_by` equals its own address) and then spawn exactly four long-lived tasks under a shutdown manager: an HTTP server (serves continuously); a node refresher on `node_refresh_rate` (default 2 hours, first tick immediate); a result submitter on `result_submission_interval` (default 15 minutes, first tick offset by one interval); and a stale-data eviction task on a derived cadence of roughly 2 minutes 30 seconds at default settings. The eviction task MUST additionally run one blocking sweep before serving, to release in-progress rows left behind by a prior crash.

#### Scenario: The full task set is spawned after gating
- **WHEN** an authorised orchestrator finishes its startup gates
- **THEN** it rehydrates its agent set from the contract and spawns the HTTP server, node refresher, result submitter, and stale-data eviction tasks

#### Scenario: A prior crash's in-flight rows are released on boot
- **WHEN** the orchestrator restarts with `testrun_in_progress` rows left over from a previous run
- **THEN** one blocking eviction sweep runs before serving to clear timed-out in-progress dispatches

### Requirement: The node refresher builds the testable-node registry from the mixnet contract and each node's self-description

The node refresher SHALL source the node list from the MIXNET contract (all `NymNodeBond`s), NOT from nym-api. For each bonded node it MUST query that node's self-described HTTP endpoint directly (with host-info verification) to learn EVERY ip address the node announces, its announced mix port, its versioned x25519 noise key, its sphinx key and key-rotation id, and its role-derived `NodeType`. Per-node queries MUST be bounded by `node_info_query_timeout` (default 10 seconds) and run with concurrency `number_of_concurrent_node_queries` (default 32); a node that fails to answer leaves the corresponding fields NULL. The refresher MUST persist ALL bonded nodes, including unreachable ones (upserting on `node_id`, updating every field except `identity_key`), so that previously-learned keys are retained when a node is transiently unreachable.

The announced address set MUST be canonicalised (`IpAddr::to_canonical()`), deduplicated and sorted before being stored, because test runs rotate through it by position: a node is free to report its addresses in a different order on every refresh (a resolved hostname typically will), and a duplicate entry would stall the rotation on a subset of the set. The stored `mixnet_socket_address` MUST be derived deterministically from the first address of that sorted set plus the announced mix port, and contributes only that port to the address a given run actually targets.

#### Scenario: A reachable node's keys are recorded
- **WHEN** the refresher queries a bonded node that answers its self-description
- **THEN** the node's announced address set, socket address, noise key, sphinx key, key-rotation id, and type are stored

#### Scenario: The announced address set is stored in a stable order
- **WHEN** a node reports its announced addresses in a different order on a later refresh
- **THEN** the stored set is unchanged, because it is canonicalised, deduplicated and sorted before storage, keeping the per-address test rotation stable

#### Scenario: An unreachable node is retained with prior data
- **WHEN** a bonded node does not answer within `node_info_query_timeout`
- **THEN** the node row is still upserted, leaving newly-unknown fields NULL and keeping any previously stored keys

### Requirement: Testruns are assigned lazily from a staleness-ordered node table guarded by an in-flight lock set

There SHALL be no in-memory work queue. When an agent requests work, the orchestrator MUST select the next node inside a `BEGIN IMMEDIATE` write transaction that: excludes any node with a `testrun_in_progress` row; requires non-null socket address, noise key, and sphinx key; requires `node_type IN ('mixnode', 'mixnode_and_gateway')`; treats a node as eligible only if it has never been tested or was last tested before `now - staleness_age` (where `staleness_age` is `test_interval`, default 2 hours); orders by test timestamp ascending with never-tested first; takes one node; rotates onto the next address in that node's announced set; records that address as the node's rotation pointer; and atomically inserts a `testrun_in_progress` row for it. The response MUST be a `TestRunAssignment { node_id, node_address, node_ips, noise_key, sphinx_key, key_rotation_id }`, where `node_address` is the rotated-onto address combined with the node's mix port and `node_ips` is every address the node announces, or an empty assignment when no eligible node exists.

The rotation MUST take the address following the previously handed-out one, wrapping around at the end of the set and restarting from the first address when the pointer is unset or no longer announced. It MUST advance when the assignment is handed out rather than when a result arrives, so a run that is abandoned still moves the node onto its next address. A node stored before the announced set was tracked MUST remain testable by falling back to the single address in its `mixnet_socket_address`.

The staleness gate is per NODE while the rotation is per ADDRESS, so a node announcing N addresses has each individual address tested roughly every N × `staleness_age` rather than every `staleness_age`.

#### Scenario: The oldest-tested eligible node is assigned
- **WHEN** an authorised, announced agent requests a testrun and eligible nodes exist
- **THEN** the never-tested-or-oldest eligible mixnode is returned and a `testrun_in_progress` row is inserted for it in the same transaction

#### Scenario: Consecutive runs against one node rotate through its addresses
- **WHEN** a node announcing both an ipv4 and an ipv6 address is assigned on two successive occasions
- **THEN** the second assignment targets the other address, so both are exercised in turn rather than whichever one a refresh happened to store

#### Scenario: An abandoned run still advances the rotation
- **WHEN** an assignment is handed out and no result is ever submitted for it
- **THEN** the node's rotation pointer has already advanced, so its next assignment targets its next address rather than repeating the same one

#### Scenario: A node already in progress is not reassigned
- **WHEN** a node has an open `testrun_in_progress` row
- **THEN** it is excluded from assignment until that row is cleared

#### Scenario: No eligible node yields an empty assignment
- **WHEN** every node is either in progress or was tested more recently than `staleness_age`
- **THEN** the agent receives an empty assignment and exits without testing

### Requirement: The orchestrator authorises both of an announcing agent's addresses on-chain in one transaction

An agent SHALL announce a PAIR of mixnet socket addresses, one ipv4 and one ipv6, because a tested node sees whichever family it was reached over as the source of the probe traffic and gates on that source ip; authorising only one family would leave probes over the other rejected.

On `POST /v1/agent/announce` the orchestrator SHALL reject with a 400, before touching any state, an announcement whose addresses are not one plain ipv4 address and one ipv6 address that is not ipv4-mapped. Such a pair MUST NOT be normalised into shape, because an ipv4-mapped ipv6 address collapses onto the ipv4 one when a node canonicalises the authorised set, leaving the agent with a single authorised ingress while both the contract and the orchestrator believe it has two, and because rewriting an address would authorise something the agent never announced and will not use in its sphinx return hop.

It MUST then upsert the agent into its in-memory `KnownAgents` cache, keyed by the agent's ipv4 mixnet socket address with the ipv6 address held inside the entry, and, if the agent was not already announced, MUST authorise BOTH addresses in the network-monitors contract by submitting ONE transaction carrying an `AuthoriseNetworkMonitor` message per address, each with the agent's base58 x25519 noise key and noise version, then mark the agent announced. Both authorisations MUST travel in a single transaction so that an agent is never left with only one of its addresses authorised. A contract transaction failure MUST surface as a 500 and leave the agent un-announced; re-announcing is safe because the contract's agent save is an upsert. An agent whose announced noise key OR ipv6 address differs from the cached one MUST have its announced flag reset so it is re-authorised, and that divergence SHOULD be surfaced (log plus counter) because a superseded ipv6 address stays authorised in the contract. This on-chain write is what ultimately causes network nodes to accept the agent's probe connections.

#### Scenario: A first announcement authorises both addresses on-chain
- **WHEN** a not-yet-announced agent calls `announce`
- **THEN** the orchestrator writes a single transaction carrying one `AuthoriseNetworkMonitor` message for each of the agent's two addresses, both with its noise key, and marks it announced

#### Scenario: A contract failure is not silently swallowed
- **WHEN** the authorisation transaction fails
- **THEN** the announce call returns a 500 and the agent remains un-announced, with neither address authorised

#### Scenario: A malformed address pair is rejected outright
- **WHEN** an agent announces two addresses of the same family, the same address twice, or an ipv4-mapped ipv6 address
- **THEN** the call is rejected with a 400 and nothing is cached or written on-chain

### Requirement: Network nodes learn the authorised-agent set from the contract and gate connection, routing, and replay-bypass on it

A Nym node SHALL derive which network-monitor agents may probe it directly from the network-monitors contract, not from any orchestrator or nym-api. A node MUST load the full authorised-agent set once at startup (via `get_all_network_monitor_agents`; a failed load aborts node startup) and MUST thereafter keep it current in REAL TIME through a nyxd websocket event subscription that dispatches `AuthoriseNetworkMonitor`, `RevokeNetworkMonitor`, and `RevokeAllNetworkMonitors` contract events. This is an event subscription, NOT a periodic contract poll; the node's periodic topology refresher explicitly preserves (does not reload) the agent set.

The node MUST fold the set into two shared, lock-free, canonical-IP-keyed structures - a routing set (`RoutableNetworkMonitors`) and a noise-key map (`NoiseNetworkView`, in which one IP may host several agents disambiguated by port) - and MUST key both on `IpAddr::to_canonical()` at insert AND lookup so that a v4-mapped-IPv6 form matches its canonical IPv4 form. There is no separate "extra initiator IPs" allowlist; inbound acceptance is a facet of the noise map. The authorised set MUST gate three behaviours: (1) the Noise responder handshake - an inbound connection from an IP not in the noise map falls back to raw TCP and the agent's handshake fails; (2) packet routing through `NetworkRoutingFilter`; and (3) most importantly, the sphinx REPLAY / bloomfilter BYPASS - a packet detected as replayed MUST be dropped as a replay UNLESS it originates from an authorised network-monitor agent IP, which is the mechanism that lets the agent's deliberately-replayed probe header (see the `reuse_header` requirement) be processed rather than filtered.

The gate is by SOURCE IP only (not public key); the port is effectively ignored on the agent-as-initiator probe path (it is consulted only when the node dials an agent). The consequences are: an agent cannot successfully probe a node until it is authorised on-chain AND that authorisation event has been ingested by the node (propagation is bounded by block inclusion plus websocket delivery, on the order of seconds, NOT by any refresh interval); the IP the agent actually connects from MUST equal one of the `mixnet_address` IPs recorded on-chain for it (an egress IP that is neither, whether through NAT or a third interface, still breaks all three gates); and because there is no periodic reconciliation against the contract, a node that misses a revoke event (for example during websocket downtime) only re-syncs on its next restart's one-time load.

Because an agent authorises one ipv4 and one ipv6 address, it occupies TWO entries in each node's structures - one per address, both carrying the same noise key - so a probe arriving over either family passes all three gates. The node treats those entries independently: it neither knows nor needs to know that they belong to one agent, and revoking one leaves the other authorised.

Intended follow-ups (recorded here as planned changes, NOT current behaviour): (1) add a periodic reconciliation of each node's authorised-agent set against the contract, so a missed revoke event no longer lingers until the next node restart; and (2) gate the replay and bloomfilter bypass on the agent's Noise-authenticated x25519 static key rather than its source IP. The current `Noise_XKpsk3` handshake already receives and possession-authenticates that key (the message-3 `se` step proves the agent holds the corresponding private key), so this hardening needs no packet-format change and would remove the source-IP spoofing and NAT-fragility of the present gate.

#### Scenario: A newly authorised agent is accepted in near real time
- **WHEN** an orchestrator authorises an agent on-chain and the transaction is included in a block
- **THEN** each node's websocket watcher ingests the `AuthoriseNetworkMonitor` event and adds the agent's IP and noise key to its routing set and noise map without waiting for any refresh interval

#### Scenario: An unauthorised agent cannot complete a handshake or have replays accepted
- **WHEN** an agent that the node has not ingested opens a connection and sends replayed packets
- **THEN** the Noise handshake falls back to raw TCP and fails, and any replayed packet is dropped as a replay because it does not come from an authorised agent IP

#### Scenario: Replayed probe traffic from an authorised agent bypasses the bloomfilter
- **WHEN** an authorised agent sends its deliberately-replayed probe header
- **THEN** the node still runs its replay-detection bloomfilter but bypasses the drop because the packet's source IP is in the authorised network-monitor set, and processes the packet

#### Scenario: Revocation stops acceptance after the event is ingested, with no periodic re-sync
- **WHEN** an agent is revoked on-chain and the node ingests the `RevokeNetworkMonitor` event
- **THEN** the node removes it from the routing set and noise map so new handshakes fail and replays are dropped again
- **AND** if the node misses that event it will only re-sync the agent set on its next restart, because there is no periodic reconciliation

### Requirement: The orchestrator exposes agent and operator HTTP surfaces on separate bearer tokens

The orchestrator SHALL protect its HTTP API with static shared-secret bearer tokens, using two distinct tokens: `agents_token` for the agent surface (`POST /v1/agent/announce`, `POST /v1/agent/request-testrun`, `POST /v1/agent/submit-testrun-result`) and `metrics_and_results_token` for the operator surface (six read-only `GET /v1/results/*` endpoints and `GET /v1/metrics/prometheus`). The root path MUST redirect to Swagger. These bearer tokens authenticate the caller as "a trusted agent" or "a trusted operator"; they are NOT per-agent identities or signatures, and the on-chain allow-set is what actually authorises an agent to probe nodes (enforced at announce-time contract writes and at nym-api ingestion, not at this HTTP layer).

#### Scenario: The agent and operator surfaces require different tokens
- **WHEN** a caller presents the `agents_token` to a `/v1/results/*` endpoint
- **THEN** the request is rejected, because that surface requires the `metrics_and_results_token`

#### Scenario: Bearer auth is not an on-chain authorisation
- **WHEN** an agent presents a valid `agents_token`
- **THEN** it may call the agent endpoints, but whether nodes accept its probes still depends on its on-chain authorisation, not on the token

### Requirement: Completed testruns are submitted to nym-api in signed, monotonic batches with at-least-once delivery

The result submitter SHALL forward completed testruns to nym-api at `POST /v3/nym-nodes/stress-testing/batch-submit`. It MUST read a persisted watermark (`last_submitted_testrun_id`), fetch completed testruns after it in ascending id order, and send them in chunks of `result_submission_batch_size` (default 50). Each `TestRun` MUST be converted to a `StressTestResult` whose `test_performance` is `packets_received / packets_sent` (or `0.0` when `packets_sent` is zero or duplicates were seen) and whose `was_reachable` is `error.is_none()`. Each batch MUST be wrapped in a `StressTestBatchSubmissionContent { signer, timestamp, results }`, given a timestamp that is strictly increasing (bumped by 1 nanosecond if the clock has not advanced since the last batch, matching nym-api's replay guard), and signed with the orchestrator's ed25519 identity key. The watermark MUST be advanced only AFTER a successful POST, so a failed submission re-sends the same testruns on the next cycle (at-least-once delivery).

#### Scenario: Only new testruns are submitted, in order
- **WHEN** the submitter runs with a watermark of N
- **THEN** it submits testruns with id greater than N in ascending id order, chunked by the batch size

#### Scenario: A failed POST is retried, not skipped
- **WHEN** a batch POST fails
- **THEN** the watermark is not advanced and the same testruns are resubmitted on the next cycle

#### Scenario: Batch timestamps are strictly monotonic
- **WHEN** two batches are produced within the same clock tick
- **THEN** the second batch's timestamp is bumped so it is strictly greater than the first, satisfying nym-api's replay check

The submitter MUST NOT treat a successful POST as proof that the batch was stored: rows deduplicate at the database, so an accepted batch can store nothing. It MUST therefore read the per-result counts nym-api returns (`accepted`, `duplicates`, `rejected`), record each as a counter, and log a warning whenever `duplicates` or `rejected` is non-zero. A count that is absent (an older nym-api that does not report them) MUST be treated as "not reported" rather than as zero, and so neither logged nor counted.

#### Scenario: A batch that stored nothing is reported rather than silently accepted
- **WHEN** nym-api accepts a batch but deduplicates every result in it away
- **THEN** the submitter records the duplicate count and logs a warning, rather than inferring success from the 200 response

#### Scenario: Absent counts are not mistaken for zero
- **WHEN** the submitting orchestrator is talking to a nym-api that does not report the per-result counts
- **THEN** submission proceeds normally and no count is recorded or warned about, because "not reported" is distinct from "nothing stored"

### Requirement: Stale in-flight dispatches and old results are evicted

The stale-data eviction task SHALL clear `testrun_in_progress` rows older than `test_timeout` (default 5 minutes), so that a dispatch abandoned by a crashed or hung agent frees its node for reassignment, and MUST delete completed testruns older than `testrun_eviction_age` (default 7 days). One eviction sweep MUST run before the HTTP server begins serving.

#### Scenario: A timed-out dispatch is released
- **WHEN** a `testrun_in_progress` row is older than `test_timeout`
- **THEN** it is removed and the node becomes eligible for assignment again

#### Scenario: Old results are pruned
- **WHEN** completed testruns are older than `testrun_eviction_age`
- **THEN** they are deleted from the database

### Requirement: Orchestrator state is a four-table SQLite database and the agent registry is in-memory only

The orchestrator SHALL persist state in a SQLite database with four tables: `metadata` (the single submission watermark row), `nym_node` (the node registry with its self-described keys and type, its announced address set, and its per-node address rotation pointer), `testrun` (completed results, each recording which address was tested), and `testrun_in_progress` (the in-flight dispatch lock set). The agent registry MUST NOT be persisted; it lives only in the in-memory `KnownAgents` cache and is rebuilt from the contract on each startup, which means agents' announced flags reset across a restart and each agent re-announces (and is re-authorised on-chain) on its next run.

Rehydrating that cache from the contract requires recovering which pair of on-chain entries belongs to one agent. The contract stores one entry per socket address and carries no field linking an agent's two addresses, so the orchestrator MUST group the entries by their x25519 noise key, which is unique per agent (see the network-monitors-contract capability, which does NOT enforce that uniqueness). Entries that do not form exactly one ipv4/ipv6 pair MUST be dropped from the cache rather than guessed at - they are either authorisations predating the paired announcement or leftovers from an agent that has since changed an address - which is safe precisely because the cache only exists to skip redundant contract transactions, and an agent always announces before requesting work.

#### Scenario: An agent's two on-chain entries are re-paired after a restart
- **WHEN** the orchestrator restarts and reads its agents from the contract
- **THEN** the ipv4 and ipv6 entries sharing one noise key are rehydrated as a single announced agent

#### Scenario: An unpairable on-chain entry is dropped rather than guessed
- **WHEN** an agent has an on-chain entry with no counterpart of the other family under the same noise key
- **THEN** it is left out of the rehydrated cache and re-created by that agent's next announcement, at the cost of one redundant authorisation transaction

#### Scenario: Node registry and results survive a restart
- **WHEN** the orchestrator restarts
- **THEN** its node registry, completed testruns, and submission watermark are loaded from SQLite

#### Scenario: The agent set is rebuilt from the contract, not from disk
- **WHEN** the orchestrator restarts
- **THEN** its agent registry is rehydrated from the network-monitors contract rather than read from local storage

### Requirement: The agent is a one-shot job that announces, requests one assignment, tests, submits, and exits

The `run-agent` path SHALL be a run-to-completion job, NOT a long-lived daemon: it builds an orchestrator client with a bearer token, loads its x25519 noise key, announces itself, requests a single testrun assignment, and - if an assignment is returned - runs exactly one stress test and submits the result, then exits. When the assignment is empty it MUST log that no work is available and exit without testing. Fleet scale is therefore achieved by running many short-lived agent invocations rather than one persistent process. The agent binary MUST also provide `build-info`, a `keygen` subcommand that generates ONLY an x25519 noise key (no ed25519 key), and a `test-node` subcommand that runs a single manual test against an explicitly-specified node bypassing the orchestrator (with no `node_id`).

#### Scenario: An assignment is tested once and submitted
- **WHEN** the agent receives a non-empty assignment
- **THEN** it runs one stress test, submits the result to the orchestrator, and exits

#### Scenario: No work available exits cleanly
- **WHEN** the agent receives an empty assignment
- **THEN** it logs that no work is available and exits without testing or submitting

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

Each test SHALL produce a result carrying: `time_taken`; ingress and egress Noise-handshake durations; the sphinx packet delay; `packets_sent` and `packets_received`; the baseline `approximate_latency`; per-packet and per-send latency distributions (minimum, mean, median, maximum, standard deviation); a `received_duplicates` flag; and an optional `error` string. Only a critical failure (for example an inability to bind the ingress listener) MUST bubble up as an error return; node-level failures (no response, bloomfilter misconfiguration) MUST be recorded inside the returned result so the orchestrator always receives partial data.

The submission that carries a result to the orchestrator MUST additionally report WHICH address was tested, and that address MUST be persisted with the run and exposed on the operator read surface. A node may announce several addresses of which only some are healthy, so without it a per-address failure is indistinguishable from a dead node and gets averaged into that node's single result series.

#### Scenario: A node-level failure still yields a result
- **WHEN** a node fails to respond or is misconfigured
- **THEN** the agent returns a result with the failure recorded in its `error` field rather than failing the job

#### Scenario: A result is attributable to the address it measured
- **WHEN** a run against one of a node's several announced addresses fails
- **THEN** the stored run records the tested address, so the failure is attributable to that address rather than to the node as a whole

### Requirement: The subsystem tests mixnodes only; the gateway test path is an unwired extension seam

In its current behaviour the subsystem SHALL stress-test mixnodes only. The orchestrator MUST only ever assign nodes of type `mixnode` or `mixnode_and_gateway`, MUST record created testruns as the mixnode test type, and nym-api MUST drop any submitted result whose entry is not a mixnode. A gateway test type exists in the data model but is NOT wired to any assignment or execution path; it MUST be treated as an unused extension seam, not as current behaviour.

#### Scenario: Only mixnodes are assigned and stored
- **WHEN** the orchestrator selects a node to test
- **THEN** it only considers mixnode-capable nodes and marks the run as a mixnode test

#### Scenario: A non-mixnode result is dropped on ingest
- **WHEN** nym-api receives a stress-test entry that is not a mixnode
- **THEN** that entry is logged and dropped without failing the batch

### Requirement: nym-api accepts batches only from contract-authorised orchestrators after staleness, replay, and signature checks

The nym-api handler for `POST /v3/nym-nodes/stress-testing/batch-submit` SHALL validate each submission through six ordered steps: (1) reject the batch if its body is older than a 30-second staleness window; (2) reject it unless the signer's ed25519 public key is in the `NetworkMonitorsCache` authorised set, which is populated from the network-monitors contract's authorised-orchestrator identity keys and refreshed lazily on a TTL (default 30 minutes); (3) reject it unless its timestamp is strictly greater than the per-signer high-water mark held in an in-memory `LastNMSubmissions` map, falling back to the process-online time when no prior submission is recorded (for example after a restart); (4) reject it unless the ed25519 signature over the JSON body verifies against the signer; (5) update the per-signer high-water mark; and (6) validate and insert the individual results.

Because the per-signer high-water mark is held in memory, it resets to the process-online time on restart; the database primary-key dedupe described in the next requirement is what ultimately guarantees idempotency. Intended follow-up (recorded here as a planned change, NOT current behaviour): persist the per-signer high-water mark across restarts as defense-in-depth.

#### Scenario: A batch from an unknown signer is rejected
- **WHEN** the signer's key is not in the contract-derived authorised set
- **THEN** the submission is rejected as unauthorised

#### Scenario: A replayed or out-of-order batch is rejected
- **WHEN** a batch's timestamp is not strictly greater than the signer's last accepted timestamp
- **THEN** the submission is rejected

#### Scenario: A tampered batch fails the signature check
- **WHEN** the body does not match its ed25519 signature for the given signer
- **THEN** the submission is rejected as failing its integrity check

### Requirement: Per-entry validation drops invalid rows without failing the batch, and rows deduplicate at the database

Within an accepted batch, nym-api SHALL validate each result independently: an entry that is not a mixnode, or whose `test_performance` lies outside the range `[0.0, 1.0]`, MUST be logged and skipped WITHOUT failing the batch. Surviving rows MUST be inserted into the `nym_node_stress_testing_result` table using an insert-or-ignore semantics, so an at-least-once resend of the same testrun is deduplicated at the database. An empty result set MUST be a no-op.

A measurement's identity MUST be the measurement itself - `(node_id, test_timestamp, submitter_pubkey)`, held as a UNIQUE constraint - and MUST NOT be the submitting orchestrator's `testrun_id`, which is an AUTOINCREMENT counter in that orchestrator's own database and is NOT durable: the orchestrator's storage may be wiped, restarting the counter from 1 while its on-chain ed25519 identity persists. Keying on it meant every resubmitted id collided with a stored row and was discarded silently behind a 200 response until the counter climbed back past its previous high-water mark, which takes about as long as the wiped database had been alive. `testrun_id` MUST still be stored, for tracing a row back to the orchestrator's own read surface, but carries no uniqueness guarantee. Each row MUST additionally carry an `id INTEGER PRIMARY KEY AUTOINCREMENT` as a stable store-assigned handle; AUTOINCREMENT (rather than a bare rowid alias) is required so that ids are not reused once retention pruning deletes rows, since reuse would reintroduce the same class of collision one layer up.

The UNIQUE constraint's column order MUST lead with `(node_id, test_timestamp)` so that its backing index also serves the per-node windowed average, which is the only read query against the table and which the previous `(testrun_id, submitter_pubkey)` key could not serve at all.

Because insert-or-ignore makes "accepted" and "stored" different outcomes, the response MUST report the split: how many results were newly stored, how many deduplicated against an already-stored measurement, and how many were dropped by per-entry validation, the three summing to the number submitted. nym-api MUST additionally log a warning when any result deduplicated away. Every count MUST be optional on the wire so that this response stays readable by an orchestrator predating it, and so that adding the counts cannot turn a stored batch into a decode failure at a reader that ignores them.

#### Scenario: A batch that stores nothing is distinguishable from one that stores everything
- **WHEN** every result in an accepted batch deduplicates against an already-stored measurement
- **THEN** the response reports zero accepted and a non-zero duplicate count, and nym-api logs a warning, rather than returning an indistinguishable success

#### Scenario: An out-of-range performance value is skipped
- **WHEN** an entry's `test_performance` is outside `[0.0, 1.0]`
- **THEN** it is logged and skipped while the rest of the batch is inserted

#### Scenario: A resent testrun does not duplicate rows
- **WHEN** the same measurement, `(node_id, test_timestamp, submitter_pubkey)`, is submitted twice due to an at-least-once retry
- **THEN** the second insert is ignored and no duplicate row is created

#### Scenario: A wiped orchestrator's restarted counter does not lose measurements
- **WHEN** an orchestrator's database is wiped, so it re-emits `testrun_id` values it has already used, against a nym-api still holding rows under those ids
- **THEN** each new measurement is stored, because its identity is its `(node_id, test_timestamp, submitter_pubkey)` triple rather than the reused id

### Requirement: The signed submission payload is a JSON-signed SignedMessage envelope

The submission payload SHALL be a `StressTestBatchSubmission`, a `SignedMessage<StressTestBatchSubmissionContent>` serialised as a camelCase `{ body, signature }` envelope where `signature` is the base58 ed25519 signature over the JSON serialisation of `body`. The `body` MUST carry `{ signer: ed25519 public key, timestamp, results }`, and each `results` entry MUST be a `StressTestResult { testrun_id, node_id, is_mixnode, test_timestamp, test_performance, was_reachable }`. The signed bytes MUST be the exact JSON serialisation of `body`, so byte-exact round-tripping of that JSON is required for the signature to verify.

#### Scenario: The signature covers the JSON body
- **WHEN** a submission is produced
- **THEN** its signature is an ed25519 signature over the JSON serialisation of `body`, verified by re-serialising `body` and checking it against `signer`

### Requirement: Stored stress-test scores feed node performance and rewarding through a defined consumer surface

The stored stress-test results SHALL form the subsystem's output contract to the rest of nym-api; the detailed behaviour of each consumer is owned by its own capability, and this requirement fixes only WHICH subsystems read the results and FOR WHAT. The stored per-node results MUST be aggregated (average performance and a reachability flag over a configured window) into a stress-testing score; that score MUST feed the node performance provider, which folds it - together with routing and configuration components - into each node's detailed performance, gated by the `use_stress_testing_data`, `minimum_available_stress_testing_results`, and `stress_testing_score_weight` configuration flags and applied only to stress-test-eligible mixnodes; and the resulting composite performance MUST flow into rewarding via the node's rewarding-performance derivation.

#### Scenario: Stress scores contribute to node performance when enabled
- **WHEN** stress-testing data is enabled and a mixnode has at least the minimum number of available results
- **THEN** its averaged stress score is folded into its detailed performance according to the configured weight

#### Scenario: The consumer surface is bounded
- **WHEN** reasoning about the blast radius of a stress-test score
- **THEN** the readers are the performance aggregation query, the performance provider, and rewarding, each specified by its own capability

### Requirement: The subsystem's behaviour is governed by orchestrator and agent configuration surfaces with defined defaults

The orchestrator SHALL be configured with the following defaults: `test_interval` 2 hours, `test_timeout` 5 minutes, `node_refresh_rate` 2 hours, `node_info_query_timeout` 10 seconds, `testrun_eviction_age` 7 days, `result_submission_interval` 15 minutes, `result_submission_batch_size` 50, `number_of_concurrent_node_queries` 32, `chain_authorisation_check_max_attempts` 10, `chain_authorisation_check_retry_delay` 1 minute, and an HTTP bind of `0.0.0.0:8080`; plus required secrets (`agents_token`, `metrics_and_results_token`, the bip39 `mnemonic`, and the base58 ed25519 `private_key`) and required endpoints (`nym_api_endpoint`, `rpc_url`, the mixnet and network-monitors contract addresses, and `database_path`). The agent SHALL be configured with the following defaults: `sending_duration` 30 seconds, `waiting_duration` 5 seconds, `packet_delay` 50 milliseconds (which MUST be non-zero), `target_rate` 1000 packets/second, `reuse_header` true, `egress_connection_timeout` 5 seconds, `noise_handshake_timeout` 3 seconds, `sending_batch_size` 50, and a listener bind of `[::]:9000`; plus the required orchestrator URL, orchestrator bearer token, announced ipv4 host address, announced ipv6 host address, shared announced port, and noise-key path. All knobs MUST be overridable by CLI flag or environment variable.

The announced pair MUST be validated at configuration time, applying the same rule the orchestrator enforces on announce, so a misconfigured deployment fails immediately rather than on its first announcement. The listener bind default MUST remain dual-stack (`[::]`), since an ipv4-only bind cannot receive the return traffic for a run whose return hop is the agent's ipv6 address.

#### Scenario: Defaults match the documented values
- **WHEN** an orchestrator or agent is configured without overriding a given knob
- **THEN** the effective value is the default listed above

#### Scenario: A zero packet delay is rejected
- **WHEN** the agent is configured with a `packet_delay` of zero
- **THEN** configuration construction fails

#### Scenario: A malformed announced address pair is rejected at startup
- **WHEN** the agent is configured with two announced addresses of the same family, or with an ipv4-mapped ipv6 address
- **THEN** configuration construction fails before the agent announces itself

