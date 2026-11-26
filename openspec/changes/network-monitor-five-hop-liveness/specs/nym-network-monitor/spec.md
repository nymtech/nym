## MODIFIED Requirements

### Requirement: The node refresher builds the testable-node registry from the mixnet contract and each node's self-description

The node refresher SHALL source the node list from the MIXNET contract (all `NymNodeBond`s), NOT from nym-api. For each bonded node it MUST query that node's self-described HTTP endpoint directly (with host-info verification) to learn EVERY ip address the node announces, its announced mix port, its versioned x25519 noise key, its sphinx key and key-rotation id, and whether it reports the mixnode and the entry-gateway role. For a node that announces an entry-gateway interface it MUST additionally learn that interface's plain client websocket port and whether the gateway enforces zk-nyms, which the route test reads so as not to draw a gateway that would refuse a client without a ticket. It MUST NOT record whether the node also announces a wss entry: NOTHING reads that fact. The probe ignores wss entries by construction, the submission carries no such field, and the divergence surface deliberately does not bucket on it, so storing it here would be a copy with no consumer at all. Per-node queries MUST be bounded by `node_info_query_timeout` (default 10 seconds) and run with concurrency `number_of_concurrent_node_queries` (default 32).

A node MUST be described COMPLETELY or not at all: every self-described field comes from one reading of the node's endpoint, and a failure of any part of that reading - including the client websocket interface or the zk-nym enforcement of a gateway-capable node - MUST discard the whole reading rather than storing the fields that did answer. The refresher MUST record the bond of ALL bonded nodes, including unreachable ones, and MUST write a node's description only from a complete reading, replacing the previous description whole. A node that could not be described keeps its previous description untouched: deleting it would drop a merely slow node out of EVERY kind until a later cycle answered, which at the liveness cadence costs several test slots per incident. A missing description therefore means "never completely described" or "no longer bonded", never "did not answer this time". `identity_key` is never updated, since a `node_id` maps to exactly one identity and is never reassigned.

After every SUCCESSFUL read of the contract's bonds, the refresher MUST delete the description of every node absent from that read and keep its bond, so that an unbonded node stops being assigned while remaining visible on the read surface with its last-seen time. A failed contract read MUST write nothing, so it can never delete a description. Without this an unbonded node keeps its last reading forever and is tested indefinitely, since nothing else removes it.

The consequence, accepted deliberately, is that a node which stops answering keeps its last reading indefinitely while it remains bonded, and continues to be assigned against it. That is the intended behaviour: a probe against stale data fails, and for a liveness measurement an unreachable node failing its probe is the measurement, whereas silently not testing it is not.

The announced address set MUST be canonicalised (`IpAddr::to_canonical()`), deduplicated and sorted before being stored, because test runs rotate through it by position: a node is free to report its addresses in a different order on every refresh (a resolved hostname typically will), and a duplicate entry would stall the rotation on a subset of the set. The description MUST store the announced mix port rather than a socket address: the address a run targets comes from the rotation over the announced set, so an address stored beside the port would only ever be read for its port.

#### Scenario: A reachable node's keys are recorded
- **WHEN** the refresher queries a bonded node that answers its self-description
- **THEN** its description is stored with the node's announced address set, mix port, noise key, sphinx key, key-rotation id, and reported roles

#### Scenario: A gateway's client websocket port is recorded
- **WHEN** the refresher queries a node that announces an entry-gateway interface
- **THEN** the node's plain client websocket port is stored, and its wss announcement is not, no consumer anywhere in the subsystem reading it

#### Scenario: A gateway's zk-nym enforcement is recorded
- **WHEN** the refresher queries a node that announces an entry-gateway interface
- **THEN** whether that gateway enforces zk-nyms is stored with its description

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

A route run is not a test kind and is not work in this sense: it targets no single node, takes no in-flight row and has no staleness per node. `network-monitor-five-hop-liveness` specifies it. When one is due, the orchestrator MUST hand it to the requesting agent before choosing a kind, and the rest of this requirement applies only when no route run is handed out.

When an agent requests work, the orchestrator MUST choose the kind, then select targets inside a `BEGIN IMMEDIATE` write transaction that: excludes any node with a `testrun_in_progress` row, REGARDLESS of which kind that row belongs to; requires the node to have a description reporting the role that kind probes; treats a node as eligible only if that kind has never tested it or last tested it before `now - staleness_age` for that kind; orders by that kind's test timestamp ascending with never-tested first; takes up to that kind's wave size, which is one for `mixnode_stress`; rotates each selected node onto the next address in its announced set FOR THAT KIND; records that address as that kind's rotation pointer; and atomically inserts a `testrun_in_progress` row for each, stamped with `started_at`, the kind, and an `expires_at` of `now` plus that kind's lease budget. The response MUST carry a route run, or the chosen kind and its per-target payload, or an empty assignment when there is neither a route run to hand out nor an eligible node.

The kind MUST be the one whose next assignable node is MOST OVERDUE across every enabled kind. A node is due at its last test by that kind plus that kind's staleness interval, and a node the kind has never tested outranks every measured one. Due times MUST be compared rather than raw last-tested timestamps, since kinds run at different cadences and a two-hour kind would otherwise always look older than a fifteen-minute one. Equally overdue kinds MUST resolve in the kinds' declaration order, which MUST place both liveness kinds ahead of `mixnode_stress`, so that a full cold start, where every kind is equally never-tested, begins with liveness. A disabled kind MUST NOT be considered, and when the chosen kind's assignment comes back empty because a concurrent request took its eligible nodes, the request MUST fall through to the next most overdue kind rather than being answered with no work.

The policy is deliberately not fair, and no kind is owed a share of requests: under sustained overload every kind falls behind by a similar absolute margin, and a backlog of never-tested nodes, from a cold start or from a newly added kind, is drained before any other kind's re-tests. No kind can be starved indefinitely, because a kind left unserved only grows more overdue until it is chosen.

Excluding any node that has an open in-progress row of ANY kind is required, not incidental: a node being stress-tested at high rate while a liveness probe measures it would bias both results. That lock is also the ONLY exclusion between kinds: a node is eligible again the moment its row clears, with no cooldown afterwards, because a stress test's 30000 packets at 1000 packets/second is around 3.1 Mbps at the probe's `AckPacket` sizing and so leaves nothing draining for a later probe to charge to the node.

The rotation MUST take the address following the previously handed-out one for that kind, wrapping around at the end of the set and restarting from the first address when the pointer is unset or no longer announced. It MUST advance when the assignment is handed out rather than when a result arrives, so a run that is abandoned still moves the node onto its next address.

The staleness gate is per NODE AND KIND while the rotation is per ADDRESS, so a node announcing N addresses has each individual address tested by a given kind roughly every N × that kind's `staleness_age` rather than every `staleness_age`.

#### Scenario: The oldest-tested eligible node is assigned
- **WHEN** an authorised, announced agent requests a testrun and eligible nodes exist
- **THEN** the orchestrator picks a kind and returns the never-tested-or-oldest node for that kind, inserting a `testrun_in_progress` row for it in the same transaction

#### Scenario: A due route run is handed out before any kind
- **WHEN** an agent requests work while a route run is due and a liveness kind is overdue
- **THEN** the agent is given the route run, and the overdue kind is assigned on a later request

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
- **WHEN** no route run is to be handed out and no enabled kind has an eligible node, every node each kind may assign being either in progress or tested by that kind more recently than its `staleness_age`
- **THEN** the agent receives an empty assignment and exits without testing

### Requirement: The agent is a one-shot job that announces, requests one assignment, tests, submits, and exits

The `run-agent` path SHALL be a run-to-completion job, NOT a long-lived daemon: it builds an orchestrator client with a bearer token, loads its x25519 noise key, announces itself, requests a single assignment, and - if one is returned - executes it and exits. An assignment is ONE stress target, a WAVE of liveness targets executed concurrently, or a ROUTE RUN; in the wave case each target's result MUST be submitted as soon as that target finishes rather than at the end of the wave, and a route run's sessions are submitted together when the run ends, as `network-monitor-five-hop-liveness` specifies. When the assignment is empty it MUST log that no work is available and exit without testing. Fleet scale is therefore achieved by running many short-lived agent invocations rather than one persistent process, with liveness sweep throughput coming from wave concurrency within an invocation rather than from a longer-lived process.

The agent MUST be able to execute every test kind, and the route run, that the orchestrator may assign, since the orchestrator is the party that chooses. The agent binary MUST also provide `build-info`, a `keygen` subcommand that generates ONLY an x25519 noise key (no ed25519 key), and a `test-node` subcommand that runs a single manual test against an explicitly-specified node bypassing the orchestrator (with no `node_id`).

#### Scenario: An assignment is tested once and submitted
- **WHEN** the agent receives a non-empty assignment
- **THEN** it executes every target it was given, submits each result, and exits

#### Scenario: No work available exits cleanly
- **WHEN** the agent receives an empty assignment
- **THEN** it logs that no work is available and exits without testing or submitting

#### Scenario: A wave's results are submitted as they complete
- **WHEN** one target of a liveness wave finishes while others are still running
- **THEN** its result is submitted immediately rather than being held until the wave ends

#### Scenario: A route run is executed and submitted once
- **WHEN** the agent receives a route run
- **THEN** it runs one client session per gateway of the run concurrently, submits them in one submission when the run ends, and exits

### Requirement: The orchestrator exposes agent and operator HTTP surfaces on separate bearer tokens

The orchestrator SHALL protect its HTTP API with static shared-secret bearer tokens, using two distinct tokens: `agents_token` for the agent surface (`POST /v1/agent/announce`, `POST /v1/agent/request-work-assignment`, and `POST /v1/agent/submit-work-result`, which takes a result of every kind of work: one one-hop target's result, or a route run's sessions) and `metrics_and_results_token` for the operator surface (seven read-only `GET /v1/results/*` endpoints and `GET /v1/metrics/prometheus`). The root path MUST redirect to Swagger. These bearer tokens authenticate the caller as "a trusted agent" or "a trusted operator"; they are NOT per-agent identities or signatures, and the on-chain allow-set is what actually authorises an agent to probe nodes (enforced at announce-time contract writes and at nym-api ingestion, not at this HTTP layer).

#### Scenario: The agent and operator surfaces require different tokens
- **WHEN** a caller presents the `agents_token` to a `/v1/results/*` endpoint
- **THEN** the request is rejected, because that surface requires the `metrics_and_results_token`

#### Scenario: Bearer auth is not an on-chain authorisation
- **WHEN** an agent presents a valid `agents_token`
- **THEN** it may call the agent endpoints, but whether nodes accept its probes still depends on its on-chain authorisation, not on the token

### Requirement: Completed testruns are submitted to nym-api in signed, monotonic batches with at-least-once delivery

The result submitter SHALL forward completed testruns to nym-api, in a SEPARATE STREAM PER TEST KIND. Each stream MUST read its own kind's results table and advance its own persisted watermark, because a testrun id is unique only within its kind and one watermark cannot describe two tables: advancing a shared watermark for one stream would skip unsubmitted rows of another. `mixnode_stress` results MUST be submitted to `POST /v3/nym-nodes/stress-testing/batch-submit`; the results of both liveness kinds MUST be submitted to the liveness endpoint, as two streams sharing one destination.

For each stream the submitter MUST read that stream's persisted watermark, fetch completed testruns of that kind after it in ascending id order, and send them in chunks of `result_submission_batch_size` (default 50). Each `mixnode_stress` run MUST be converted to a `StressTestResult` whose `test_performance` is `packets_received / packets_sent` (or `0.0` when `packets_sent` is zero or duplicates were seen) and whose `was_reachable` is `error.is_none()`. Each run of a liveness kind MUST be converted to a result whose performance is the average over that kind's fixed measurement set, with an interface that produced no measurement counted as zero, and MUST NOT carry the per-interface breakdown it averaged over. When five-hop liveness is enabled, that performance MUST then be multiplied by the node's five-hop liveness for the newest epoch the orchestrator has materialised, as `network-monitor-five-hop-liveness` defines it, so that nym-api's own liveness figure includes it for as long as this stream exists. A node the route test has never measured has a five-hop liveness of 1, so its posted performance is unchanged. Stress results are not scaled. The averaging MUST therefore happen here rather than at nym-api: the denominator is the set the run's role is expected to produce, and neither that role nor the interfaces reach the wire, so the receiver could not reconstruct it. An interface that saw duplicates MUST also count as zero, on the same reasoning as the stress path: a node that forwards one packet and replays it nine times counts ten received against ten sent, so scoring the ratio would hand a perfect result to a node delivering a tenth of the traffic. Each batch MUST be wrapped in a submission content carrying `{ signer, timestamp, results }`, given a timestamp that is strictly increasing across EVERY batch of one submission sweep (bumped by 1 nanosecond if the clock has not advanced since the previous batch), because nym-api's replay guard is per endpoint and two streams post to the liveness one, and signed with the orchestrator's ed25519 identity key. Each stream's watermark MUST be advanced only AFTER a successful POST, so a failed submission re-sends the same testruns on the next cycle (at-least-once delivery).

#### Scenario: A posted liveness run carries the node's five-hop liveness
- **WHEN** a mixnode liveness run scoring 1.0 is posted for a node whose five-hop liveness is 0.9
- **THEN** the performance posted to nym-api is 0.9

#### Scenario: A posted stress run is not scaled
- **WHEN** a mixnode stress run is posted for a node whose five-hop liveness is 0.9
- **THEN** its posted performance is the run's own

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
- **WHEN** two batches are produced within the same clock tick, including two from different streams posting to the same endpoint
- **THEN** the second batch's timestamp is bumped so it is strictly greater than the first, satisfying nym-api's replay check

The submitter MUST NOT treat a successful POST as proof that the batch was stored: rows deduplicate at the database, so an accepted batch can store nothing. It MUST therefore read the per-result counts nym-api returns (`accepted`, `duplicates`, `rejected`), record each as a counter, and log a warning whenever `duplicates` or `rejected` is non-zero. A count that is absent (an older nym-api that does not report them) MUST be treated as "not reported" rather than as zero, and so neither logged nor counted.

#### Scenario: A batch that stored nothing is reported rather than silently accepted
- **WHEN** nym-api accepts a batch but deduplicates every result in it away
- **THEN** the submitter records the duplicate count and logs a warning, rather than inferring success from the 200 response

#### Scenario: Absent counts are not mistaken for zero
- **WHEN** the submitting orchestrator is talking to a nym-api that does not report the per-result counts
- **THEN** submission proceeds normally and no count is recorded or warned about, because "not reported" is distinct from "nothing stored"

### Requirement: Orchestrator state is a per-kind SQLite schema and the agent registry is in-memory only

The orchestrator SHALL persist state in a SQLite database of: `nym_node_bond`, the bonded-node registry (node id, identity key, and when the node was last seen bonded); `nym_node_description`, each node's latest complete self-description (mix port, announced address set, noise key, sphinx key and key-rotation id, the mixnode and entry-gateway role flags as the node reports them, plain client websocket port, and whether the gateway enforces zk-nyms), at most one per bonded node; a work-state table keyed `(node_id, test_kind)` holding that kind's last-tested timestamp and address rotation pointer; one results table PER KIND (`mixnode_liveness_testrun`, `gateway_liveness_testrun`, `mixnode_stress_testrun`), each row recording the node, the address that was tested, the run's timing and error, and one column group per interface that kind exercises; a submission-watermark table with one row per kind; and `testrun_in_progress` (the in-flight dispatch lock set, keyed by `node_id` alone so that only one test of any kind runs against a node at a time, and carrying a materialised `expires_at` plus the kind it was dispatched for). It SHALL also hold `route_run`, one row per route run handed out, with its start, its expiry and when its sessions were submitted, and `route_session`, one row per gateway of a submitted run, with the gateway, the mixnet epoch its batch names (none for a session stored with an error, which has no batch), its record count, its error if it had one, and the batch itself as it was received; `network-monitor-five-hop-liveness` specifies both.

The set of kinds MUST be listed once, in a `test_kind` table keyed by the kind's name that every kind column references, so that adding a kind inserts a row rather than rebuilding each table naming one, SQLite being unable to alter a `CHECK` constraint.

Every column of a description MUST be non-null except the client websocket port, which MUST be null exactly when the node does not report the entry-gateway role, and the zk-nym enforcement flag, which MUST be null for a node that does not report that role and stays null on a description written before the column existed, until the next complete reading replaces it. The roles MUST be stored as the two flags the node reports rather than folded into a combined classification, since every reader asks about one role at a time. A node therefore has either a complete description or none, so no consumer has to reason about a partially described node, and eligibility for every kind requires one. Results and work state MUST reference the bond rather than the description, so deleting a description loses neither.

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

#### Scenario: An unread enforcement flag is not read as "does not enforce"
- **WHEN** a gateway's description was written before the zk-nym enforcement column existed
- **THEN** the column is null until the next complete reading of that gateway replaces the description

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
- **THEN** its node bonds and descriptions, per-kind work state, per-kind completed testruns, and per-kind submission watermarks are loaded from SQLite

#### Scenario: The agent set is rebuilt from the contract, not from disk
- **WHEN** the orchestrator restarts
- **THEN** its agent registry is rehydrated from the network-monitors contract rather than read from local storage

