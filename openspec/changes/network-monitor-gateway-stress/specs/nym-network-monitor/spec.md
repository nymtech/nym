## ADDED Requirements

### Requirement: Gateway stress is the gateway liveness test under its own stress profile, one target per assignment

The subsystem SHALL stress-test gateways through a kind of its own, `gateway_stress`. A gateway stress run SHALL be the test that "Gateway liveness is one indivisible test of two phases over a single client session" specifies, performed the same way in every respect: one agent, one client session established as "Gateway client sessions are established over plain ws at an announced ip address" specifies, the ingress phase followed by the egress phase, one measurement per phase (`client_ingest` and `client_delivery`) over a denominator fixed at two, and one announced address for both phases. It differs in two things only: it runs under the gateway stress profile rather than the liveness profile, and an assignment carries exactly ONE target.

Each phase MUST send the profile's packet count in full, that count being the profile's rate held for its sending duration. No packet may be sent ahead of a phase's load to test connectivity or the replay bypass: a phase that returns nothing is measured as a zero, exactly as in a liveness run.

The two gateway kinds are therefore one sequence at two volumes, which is the relation the two mixnode kinds have. A gateway cannot be given the mixnode stress probe, because a gateway-only node processes no forward hops. The load is one client session's worth in each direction: an agent holds one identity and a gateway accepts one session per identity, so a run does not measure many concurrent clients, and it does not exercise a gateway's exit services.

#### Scenario: A gateway stress run is the liveness sequence at stress volume
- **WHEN** a gateway stress assignment is executed
- **THEN** the agent establishes one client session, runs the ingress phase and then the egress phase within it under the gateway stress profile, and reports one measurement per phase

#### Scenario: A gateway stress assignment carries one target
- **WHEN** an agent is assigned gateway stress work and many gateways are eligible
- **THEN** the assignment carries exactly one target

#### Scenario: Each phase sends the profile's full count
- **WHEN** the gateway stress profile is 200 packets/second for 30 seconds
- **THEN** the ingress phase and the egress phase each send 6000 packets

#### Scenario: A phase that returns nothing is measured rather than skipped
- **WHEN** the gateway forwards none of the ingress phase's packets
- **THEN** the phase sends its whole load and records a zero, and the egress phase still runs

#### Scenario: The gateway stress profile is used rather than another
- **WHEN** a gateway stress run executes
- **THEN** it sends at the gateway stress profile's rate, duration and batch size, leaving the liveness profile and the mixnode stress profile untouched

### Requirement: A gateway phase stops sending when its send window ends and counts what arrived

Each phase of a gateway run, of either gateway kind, SHALL stop sending once its profile's send window has elapsed since the phase began sending, whether or not every packet has been handed to the gateway, and MUST do so even while a write is blocked by the gateway's back-pressure. The reported sent count MUST be the profile's expected count for the phase, so the packets the gateway did not take inside the window count as lost.

A phase that stops before completing its sends, because its window ended or because sending failed part-way, MUST still wait out its straggler window and count every packet that arrived, scored against the full expected count. A run cut off by its per-target deadline MUST count the packets that had already arrived for the phase then in flight, without waiting further. In neither case may arrivals be discarded because the phase did not complete. A phase ended by its window or by a failed send MUST NOT prevent the other phase from running; a phase the deadline left no time to run reports zero.

A gateway that cannot take the load inside the window is not sustaining it, which is what the delivery ratio exists to show. Without the bound a slow ingress path would stretch its phase until the run's deadline, and the egress phase would report zero without having been measured, so one interface's slowness would erase the other's result. Counting what arrived keeps the ratio proportional: a gateway that sustained four fifths of the rate scores about 0.8 on that interface rather than 0. The rule belongs to the sequence both gateway kinds share. At the liveness profile's values it changes an outcome only under back-pressure or a mid-phase failure.

#### Scenario: Packets not taken inside the window are lost
- **WHEN** a gateway takes forwards at half the profile's rate throughout the ingress phase
- **THEN** the phase stops sending at the end of its send window, reports the expected count as sent, and its delivery ratio is about one half

#### Scenario: A blocked write does not outlast the window
- **WHEN** a write to the gateway blocks for longer than the phase's send window
- **THEN** the phase stops sending when the window ends rather than when the run's deadline passes

#### Scenario: A slow ingress phase leaves the egress phase its measurement
- **WHEN** the ingress phase is ended by its send window with packets unsent
- **THEN** the egress phase runs in full and is measured

#### Scenario: A phase whose sending fails part-way keeps what arrived
- **WHEN** the gateway closes the session after 5000 of an ingress phase's 6000 forwards, 4900 of which reached the agent
- **THEN** the ingress measurement reports 6000 sent and 4900 received

#### Scenario: A run cut off by its deadline keeps what arrived
- **WHEN** the per-target deadline passes during the egress phase
- **THEN** the run reports the ingress measurement it completed and the egress packets that had already arrived, with the deadline recorded as the run's error

### Requirement: Gateway stress is scheduled on the stress cadence behind its own switch

`gateway_stress` SHALL use the staleness interval and the lease budget of `mixnode_stress` (`test_interval` and `test_timeout`) and a wave size of one. It SHALL be declared after `mixnode_stress`, so that equally overdue kinds resolve as the liveness kinds, then mixnode stress, then gateway stress. The orchestrator MUST carry an enable flag for gateway stress alone; while it is off the scheduler MUST NOT consider `gateway_stress`, and every other kind continues unaffected.

The stress kinds share their cadence as the liveness kinds share theirs, which also keeps a dual-role node's two stress kinds at comparable sample counts when they are combined. The kind has a switch of its own because its pass mark is provisional. A newly enabled kind has never tested any node, so under the scheduler's rule it is offered on every request until each gateway has been tested once.

#### Scenario: Gateway stress follows the stress interval
- **WHEN** a gateway was last tested by `gateway_stress` 2 hours 5 minutes ago against a 2-hour `test_interval`
- **THEN** it is due for `gateway_stress`

#### Scenario: Gateway stress can be disabled without a new build
- **WHEN** the orchestrator's gateway stress enable flag is set to off
- **THEN** no gateway stress assignment is handed out, and mixnode stress and both liveness kinds continue unaffected

#### Scenario: A cold start reaches gateway stress last
- **WHEN** no kind has yet tested any node
- **THEN** requests are assigned the liveness kinds, then `mixnode_stress`, and `gateway_stress` only once none of those has a never-tested node left to hand out

## MODIFIED Requirements

### Requirement: The subsystem's behaviour is governed by orchestrator and agent configuration surfaces with defined defaults

The orchestrator SHALL be configured with the following defaults: `test_interval` 2 hours, `test_timeout` 5 minutes, `node_refresh_rate` 2 hours, `node_info_query_timeout` 10 seconds, `testrun_eviction_age` 7 days, `number_of_concurrent_node_queries` 32, `chain_authorisation_check_max_attempts` 10, `chain_authorisation_check_retry_delay` 1 minute, `metrics_refresh_interval` 15 seconds, `agent_liveness_window` 15 minutes, `lease_release_interval` 30 seconds, `retention_sweep_interval` 1 hour, and an HTTP bind of `0.0.0.0:8080`; plus required secrets (`agents_token`, `metrics_and_results_token`, the bip39 `mnemonic`, and the base58 ed25519 `private_key`) and required endpoints (`rpc_url`, the mixnet and network-monitors contract addresses, and `database_path`). There is no nym-api endpoint and no result-submission interval or batch size: results reach the performance contract through the materialiser, as `network-monitor-contract-submission` specifies. `metrics_refresh_interval` MUST NOT exceed the deployment's Prometheus scrape interval, or sampled series go stale between scrapes. `lease_release_interval` MUST NOT exceed the shortest configured lease budget, since every lease held past its deadline is a node excluded from assignment; configuration SHALL fail rather than accept a value that cannot satisfy this.

Where a knob governs a per-kind behaviour it MUST be expressible per kind, except that the two liveness kinds share one staleness interval, one lease budget and one enable flag, and that the two stress kinds share one staleness interval and one lease budget. The orchestrator MUST additionally carry, for the liveness kinds: a staleness interval (defaulting well below the stress `test_interval`, so that liveness tracks v1's cadence) of 15 minutes, a lease budget used as the in-progress `expires_at` (which MUST bound one concurrent wave, not the sum over its targets, and MUST therefore cover the slower of the two probes) of 1 minute, a wave size PER KIND - 100 for `mixnode_liveness` and 50 for `gateway_liveness`, since a wave is one concurrent batch and a gateway target costs the agent a live client session where a mixnode target costs a Noise connection - and an enable flag allowing both liveness kinds to be switched off without a code change, i.e. by configuration and a restart rather than by a build, defaulting to ON and therefore expressible as an explicit value rather than as a bare presence flag, since a flag that can only be switched on cannot be switched off by a deployment. `test_interval` and `test_timeout` are the staleness interval and lease budget of both stress kinds, so the lease MUST cover the slower of the two runs, which is the gateway one. The orchestrator MUST also carry an enable flag for `gateway_stress` alone, defaulting to ON and expressible as an explicit value for the same reason.

The agent SHALL be configured with the following defaults: `mixnode_stress_sending_duration` 30 seconds, `mixnode_stress_waiting_duration` 5 seconds, `packet_delay` 50 milliseconds (which MUST be non-zero), `mixnode_stress_target_rate` 1000 packets/second, `reuse_header` true, `egress_connection_timeout` 5 seconds, `noise_handshake_timeout` 3 seconds, `mixnode_stress_sending_batch_size` 50, and a listener bind of `[::]:9000`; plus the required orchestrator URL, orchestrator bearer token, announced ipv4 host address, announced ipv6 host address, shared announced port, and noise-key path. Of these, the four `mixnode_stress_*` knobs are the MIXNODE stress profile, named for it so that none can be read as a gateway stress knob. The agent MUST additionally carry a liveness profile: a per-target packet count, a PER-TARGET send rate (never an aggregate budget divided across the wave), a straggler wait, and a per-target timeout; and a gateway stress profile of its own: a target rate, a sending duration, a batch size, a straggler wait and a per-target timeout, each a knob separate from the mixnode stress one of the same meaning. All knobs MUST be overridable by CLI flag or environment variable.

The liveness profile's initial values are PROVISIONAL, chosen for score granularity rather than measured against agent hardware: a per-target packet count of 50 (giving 2% granularity against v1's three packets per route), a per-target rate of 50 packets/second, a straggler wait of 5 seconds, a per-target timeout of 30 seconds, and wave sizes of 100 mixnode targets and 50 gateway targets. The rate is well below the stress profile's 1000 packets/second, so no target is under load, and it puts a target's send window at about one second, which is as long as a probe asking whether a node forwards at all has any reason to hold one open. A full mixnode wave therefore emits 5000 packets/second for roughly that second, about 15 Mbps at the probe's `AckPacket` sizing, while totalling 5000 packets against a stress run's 30000. The packet count is sized for the interpretability of a SINGLE run on the operator read surface, not for statistical power, which comes from the cadence: at a 15-minute interval a node contributes on the order of a hundred runs to each daily aggregation window. Because they are provisional, every one of them MUST be tunable in a deployment without a code change, and no behaviour may depend on a specific value: an agent host that cannot sustain a full wave MUST be correctable by configuration alone.

The gateway stress profile's initial values are PROVISIONAL too, and deliberately lenient: a target rate of 200 packets/second held for 30 seconds, so 6000 packets per phase, in batches of 10 so that a dispatch goes out every 50 milliseconds as in the mixnode stress profile, a straggler wait of 5 seconds, and a per-target timeout of 3 minutes. Gateways have never been tested under load, so the rate starts at a fifth of the mixnode profile's and is meant to be raised progressively as runs against real gateways show what they sustain. Every one of these values MUST be tunable in a deployment without a code change, and no behaviour may depend on a specific value. The per-target timeout has to sit inside the stress lease; as with the other per-target timeouts, nothing validates it against the profile or the lease.

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

#### Scenario: Retuning one stress profile leaves the other untouched
- **WHEN** the agent's gateway stress target rate is overridden
- **THEN** gateway stress runs at the overridden rate and mixnode stress keeps its own configured rate

### Requirement: Liveness is tested by two kinds of its own, and the mixnode probe is the stress probe at a low-volume profile

The subsystem SHALL support liveness testing alongside stress testing. A test kind is ONE probe against ONE role, and the kinds are `mixnode_liveness`, `gateway_liveness`, `mixnode_stress` and `gateway_stress`; "liveness" names the family of the first two and "stress" that of the last two, rather than kinds of their own. Every unit of work MUST be identified by exactly one kind, and the orchestrator MUST be the party that decides which kind a given assignment carries; an agent MUST support every kind. Where this specification says "a stress test" or "the stress profile" without naming a role, it describes `mixnode_stress`; `gateway_stress` is specified by requirements of its own.

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

### Requirement: Each test kind defines which nodes it assigns and how their results are typed

Each test kind SHALL declare the role a node must report to be eligible for it. `mixnode_stress` and `mixnode_liveness` MUST assign only nodes reporting the mixnode role, and `gateway_liveness` and `gateway_stress` MUST assign only nodes reporting the entry-gateway role. A dual-role node, reporting both, is therefore eligible for all four kinds, each with its own staleness; its liveness score MUST be the average over the runs of both liveness kinds, and its stress score the average over the runs of both stress kinds.

A node with no description, because it has never been completely described or is no longer bonded, MUST be ineligible for every kind, as MUST a described node that reports no roles.

#### Scenario: A gateway-only node is assignable for the gateway kinds only
- **WHEN** the orchestrator selects work for a gateway-only node
- **THEN** it may assign `gateway_liveness` and `gateway_stress` and never `mixnode_liveness` or `mixnode_stress`

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

The orchestrator SHALL persist state in a SQLite database of: `nym_node_bond`, the bonded-node registry (node id, identity key, and when the node was last seen bonded); `nym_node_description`, each node's latest complete self-description (mix port, announced address set, noise key, sphinx key and key-rotation id, the mixnode and entry-gateway role flags as the node reports them, and plain client websocket port), at most one per bonded node; a work-state table keyed `(node_id, test_kind)` holding that kind's last-tested timestamp and address rotation pointer; one results table PER KIND (`mixnode_liveness_testrun`, `gateway_liveness_testrun`, `mixnode_stress_testrun`, `gateway_stress_testrun`), each row recording the node, the address that was tested, the run's timing and error, and one column group per interface that kind exercises; and `testrun_in_progress` (the in-flight dispatch lock set, keyed by `node_id` alone so that only one test of any kind runs against a node at a time, and carrying a materialised `expires_at` plus the kind it was dispatched for). The submission-watermark table of the former nym-api result stream SHALL be dropped by a migration.

The set of kinds MUST be listed once, in a `test_kind` table keyed by the kind's name that every kind column references, so that adding a kind inserts a row rather than rebuilding each table naming one, SQLite being unable to alter a `CHECK` constraint. `gateway_stress` SHALL be added that way, by a migration that inserts its row and creates its results table and touches no other table.

Every column of a description MUST be non-null except the client websocket port, which MUST be null exactly when the node does not report the entry-gateway role. The roles MUST be stored as the two flags the node reports rather than folded into a combined classification, since every reader asks about one role at a time. A node therefore has either a complete description or none, so no consumer has to reason about a partially described node, and eligibility for every kind requires one. Results and work state MUST reference the bond rather than the description, so deleting a description loses neither.

Each kind's results table MUST carry the same column group for every interface it exercises: one `mix_forwarding` group for the two mixnode kinds, and a `client_ingest` and a `client_delivery` group for the two gateway kinds. A result MUST be stored only under the kind recorded on its in-flight row, and a result shaped for any other kind MUST be rejected; because a result carries exactly one measurement per interface its kind exercises, every column group is written from the submission and none can be missing or extra. A testrun id is unique only within its kind, so the operator read surface MUST address a completed run by its kind as well as its id.

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

#### Scenario: The two gateway kinds' results are stored apart
- **WHEN** a gateway has completed runs of both `gateway_liveness` and `gateway_stress`
- **THEN** each kind's runs are read from that kind's own table, and a gateway stress result submitted for a node dispatched for `gateway_liveness` is rejected although the two share a shape
