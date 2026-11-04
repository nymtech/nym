## RENAMED Requirements

- FROM: `### Requirement: The orchestrator gates startup on chain balance, contract authorisation, identity-key reconciliation, and nym-api recognition`
- TO: `### Requirement: The orchestrator gates startup on chain balance, contract authorisation and identity-key reconciliation`

- FROM: `### Requirement: The orchestrator runs seven long-lived tasks and rehydrates its agent set from the contract at startup`
- TO: `### Requirement: The orchestrator runs six long-lived tasks and rehydrates its agent set from the contract at startup`

- FROM: `### Requirement: Every nym-api reads the performance contract, and only the rewarding one holds the window`
- TO: `### Requirement: Every nym-api reads the performance contract for its snapshots`

- FROM: `### Requirement: In contract mode nym-api publishes the verified routing figure as each node's performance`
- TO: `### Requirement: nym-api publishes the verified routing figure as each node's performance`

## MODIFIED Requirements

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

The node-status refresher SHALL build the annotation map from the latest settled performance snapshot (`performance-attestation-provider`): for every node in that snapshot's bond set with a committed entry, an annotation whose `performance_score` is the entry's committed score, or 0 when the entry has none, whose current role is the node's role in the current rewarded set, and which carries no chain capabilities. The topology routes publish a node's performance from `performance_score` alone, as its nearest whole percent, so every one of them publishes the value the contract's digest committed at that snapshot's height, which nym-api verified before signing it, to that precision. A bonded node without a committed entry MUST have no annotation, and is served 0 like a node never annotated. No describe-based filter applies: whether a node is alive is the network monitor's measurement. The annotations MUST NOT carry the rewarding figure, which differs from the routing figure only after a partial measurement gap longer than the contract's 24-epoch lookback or in the minutes after a weights update, and which is on chain through `RewardingScore`.

The remaining performance fields are served only by the per-node annotation and performance routes, and MAY therefore carry values the digest does not commit. The config, stress and liveness scores SHALL come from the medians of the entry's bundle that the producer holds beside that snapshot (`performance-attestation-provider`), each 0 when the kind has no median, with the stress and liveness reachability flags set exactly when their median is present. An entry from an epoch older than the one before the newest among the snapshot's entries has no medians, and its sub-scores SHALL keep their defaults. The config score's breakdown is not in the contract and SHALL be inferred from the score, since a node scores 0 unless it is described, runs nym-node and accepted the terms, and 1 only with no versions behind: a positive score SHALL mark the node described, running nym-node and having accepted the terms, and a score of 1 SHALL give 0 versions behind, the versions behind being absent otherwise; a score of 0, or no config median, SHALL leave the config score at the shape served for a node nym-api cannot describe. The config score's chain capabilities and the routing score SHALL keep their defaults. An entry without a score, a bundle the contract cannot score (no config median, or only zero-weighted kinds measured), SHALL be annotated with `performance_score` 0 and the sub-scores of its medians.

While no performance snapshot has settled, a refresh MUST fail and leave the annotation map as it is.

The annotations SHALL be persisted to the file the contract-built annotations have always used, so that an annotation file written by a version that computed its own figure is never loaded.

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

#### Scenario: No settled snapshot leaves the annotations as they are
- **WHEN** a refresh runs while no performance snapshot has settled
- **THEN** the refresh fails and the annotation map is left unchanged

## ADDED Requirements

### Requirement: nym-api keeps every monitor-era GET route mounted and deprecated over frozen data

Every GET route nym-api served before the v1 monitor's removal SHALL stay mounted, marked deprecated in the OpenAPI document and on its handler, with its documentation saying that the data behind it no longer changes. The `/v1/status` monitor routes (per-node histories and core-status counts, the unstable per-node test results and the run details) SHALL be mounted unconditionally, there being no monitor flag, and SHALL read the v1 monitor's tables, which nothing writes any more and whose rows are never purged, so each answers what it answered at the cutover. The three `/v1/nym-nodes/*-history` routes SHALL read the historical-uptime tables the same way, a date query for a day after the cutover answering null. The POST routes that fed the storage (`submit-node-monitoring-results`, `submit-gateway-monitoring-results`, the stress and liveness `batch-submit`) SHALL NOT exist. Three GET routes are removed by explicit exception, none of them serving data that could be frozen: `/v3/nym-nodes/stress-testing/known-monitors/{identity_key}`, which answered only whether this nym-api would accept an orchestrator's submissions and whose one reader was the orchestrator's startup check, and `/v3/unstable/nym-nodes/liveness-divergence` and `/v3/unstable/nym-nodes/contract-divergence`, temporary gauges built under `/unstable` to be removed once they had informed the weighting of liveness and the switch to the performance contract, both of which have happened. Only the `*_v2` tables and the two v3 result tables, which no GET route reads, are dropped.

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

## REMOVED Requirements

### Requirement: Completed testruns are submitted to nym-api in signed, monotonic batches with at-least-once delivery
**Reason**: The orchestrator submits to the performance contract alone; nym-api no longer ingests results, having no figure of its own to feed.
**Migration**: Remove `nym_api_endpoint`, `result_submission_interval` and `result_submission_batch_size` from orchestrator configurations; the orchestrator's migration drops the watermark table.

### Requirement: nym-api accepts batches only from contract-authorised orchestrators after staleness, replay, and signature checks
**Reason**: The batch-submit routes are removed with nym-api's own figure, and with them the per-signer replay guard, the known-monitors route and the network-monitors cache behind both.
**Migration**: None; nothing submits to nym-api.

### Requirement: Per-entry validation drops invalid rows without failing the batch, and rows deduplicate at the database
**Reason**: The batch-submit routes and the stress and liveness result tables are removed.
**Migration**: None.

### Requirement: The signed submission payload is a JSON-signed SignedMessage envelope
**Reason**: The submission payload no longer exists; the `SignedMessage` envelope type stays for nym-api's other signed requests.
**Migration**: None.

### Requirement: Stored stress-test scores feed node performance and rewarding through a defined consumer surface
**Reason**: nym-api computes no figure of its own: the legacy provider, the scoring section, the switch and legacy rewarding are removed, and with them the temporary liveness divergence surface that compared its scores. Its annotations come from the attested snapshots as "nym-api publishes the verified routing figure as each node's performance" specifies; rewarding and selection are `nym-epoch-driver`'s.
**Migration**: Remove `[performance_provider]` from nym-api configurations; it is ignored if left.

### Requirement: nym-api ingests liveness batches on their own endpoint with their own replay state
**Reason**: The liveness batch-submit route is removed with the stress one.
**Migration**: None.

### Requirement: nym-api keeps a window of the performance contract's per-epoch medians
**Reason**: Moved to `nym-epoch-driver`, "The driver keeps a window of the performance contract's per-epoch medians"; no nym-api holds a window.
**Migration**: Remove `[rewarding]` from nym-api configurations; the rewarding host runs `nym-epoch-driver`.

### Requirement: nym-api resolves a node's contract figure with the contract's own resolution rule
**Reason**: Moved to `nym-epoch-driver`, "The driver resolves a node's contract figure with the contract's own resolution rule".
**Migration**: None.

### Requirement: In contract mode rewarding pays the contract's figure for the sealed epoch
**Reason**: Moved to `nym-epoch-driver`, "Rewarding pays the contract's figure for the sealed epoch"; there is no legacy mode.
**Migration**: The rewarding host runs `nym-epoch-driver` with the rewarding account's mnemonic, started only after nym-api's rewarding has stopped.

### Requirement: In contract mode the active set is selected by the contract's figure, and by stake alone when nothing was submitted at the epoch
**Reason**: Moved to `nym-epoch-driver`, "The active set is selected by the contract's figure, and by stake alone when nothing was submitted at the epoch", with candidates and roles from the directory contract instead of nym-api's describe cache.
**Migration**: None.

### Requirement: nym-api serves a temporary contract-divergence route while it holds both figures
**Reason**: nym-api holds neither side any more, no window and no figure of its own, and the switch the route was built to inform has happened, so the temporary route is removed as it was always meant to be.
**Migration**: None; the route lived under `/unstable` to carry no compatibility promise.
