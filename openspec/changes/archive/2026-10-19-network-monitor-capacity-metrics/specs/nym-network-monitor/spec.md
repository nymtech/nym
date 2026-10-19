## RENAMED Requirements

- FROM: `### Requirement: The orchestrator runs six long-lived tasks and rehydrates its agent set from the contract at startup`
- TO: `### Requirement: The orchestrator runs seven long-lived tasks and rehydrates its agent set from the contract at startup`

## MODIFIED Requirements

### Requirement: The orchestrator runs seven long-lived tasks and rehydrates its agent set from the contract at startup

On a successful startup the orchestrator SHALL rebuild its in-memory agent registry from the contract (all agents whose `authorised_by` equals its own address) and then spawn exactly seven long-lived tasks under a shutdown manager: an HTTP server (serves continuously); a node refresher on `node_refresh_rate` (default 2 hours, first tick immediate); a result submitter on `result_submission_interval` (default 15 minutes, first tick offset by one interval); a stale-data eviction task driving two independent timers, `lease_release_interval` (default 30 seconds) and `retention_sweep_interval` (default 1 hour); a chain-capability refresher, waking every 5 minutes (first tick immediate) to re-query the nodes whose cached on-chain standing is missing or due; a materialiser, whose first pass runs immediately and each later pass when the epoch in progress is due to end, or after a one-minute floor when the previous pass failed, the interval could not be read, or the epoch is overdue; and a scheduling-telemetry task on `metrics_refresh_interval` (default 15 seconds, first tick immediate). Submission to the performance contract, when on, runs as the last step of the materialiser's pass rather than as a task of its own. The eviction task MUST additionally run one blocking sweep before serving, to release in-progress rows left behind by a prior crash.

#### Scenario: The full task set is spawned after gating
- **WHEN** an authorised orchestrator finishes its startup gates
- **THEN** it rehydrates its agent set from the contract and spawns the HTTP server, node refresher, result submitter, stale-data eviction, chain-capability refresher, materialiser, and scheduling-telemetry tasks

#### Scenario: A prior crash's in-flight rows are released on boot
- **WHEN** the orchestrator restarts with `testrun_in_progress` rows left over from a previous run
- **THEN** one blocking eviction sweep runs before serving to clear timed-out in-progress dispatches

#### Scenario: Telemetry does not depend on the eviction cadence
- **WHEN** the scheduling-telemetry task and the stale-data eviction task are both running
- **THEN** the telemetry task refreshes its series on its own interval, which is shorter than the shortest lease budget, rather than being driven by the eviction sweep

### Requirement: The subsystem's behaviour is governed by orchestrator and agent configuration surfaces with defined defaults

The orchestrator SHALL be configured with the following defaults: `test_interval` 2 hours, `test_timeout` 5 minutes, `node_refresh_rate` 2 hours, `node_info_query_timeout` 10 seconds, `testrun_eviction_age` 7 days, `result_submission_interval` 15 minutes, `result_submission_batch_size` 50, `number_of_concurrent_node_queries` 32, `chain_authorisation_check_max_attempts` 10, `chain_authorisation_check_retry_delay` 1 minute, `metrics_refresh_interval` 15 seconds, `agent_liveness_window` 15 minutes, `lease_release_interval` 30 seconds, `retention_sweep_interval` 1 hour, and an HTTP bind of `0.0.0.0:8080`; plus required secrets (`agents_token`, `metrics_and_results_token`, the bip39 `mnemonic`, and the base58 ed25519 `private_key`) and required endpoints (`nym_api_endpoint`, `rpc_url`, the mixnet and network-monitors contract addresses, and `database_path`). `metrics_refresh_interval` MUST NOT exceed the deployment's Prometheus scrape interval, or sampled series go stale between scrapes. `lease_release_interval` MUST NOT exceed the shortest configured lease budget, since every lease held past its deadline is a node excluded from assignment; configuration SHALL fail rather than accept a value that cannot satisfy this.

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

## ADDED Requirements

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
