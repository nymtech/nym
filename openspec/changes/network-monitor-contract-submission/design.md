## Context

This change is the third of four stacked PRs that get network-monitor-v3 measurements into the performance contract and serve them verifiably: the contract side (`performance-contract-verifiable-routing-view`, archived), the retrieval library (`performance-retrieval-client`, archived), this submission path, and the nym-api consumer (`nym-api-performance-contract-consumer`). They are reviewed separately and merged together; this one is deployed after the contract. The orchestrator's existing submission of raw results to nym-api is an intermediate step: a later iteration removes it, and the network monitor then submits only to the contract.

What the orchestrator has today:

- `Materialiser` (`aggregation/materialiser/mod.rs`) owns one task and reads the mixnet interval once per tick. It hands that reading to two members: `AggregateMaterialiser`, which files each epoch's aggregates per test kind (mixnode liveness, gateway liveness, stress) over the window preceding the epoch's start and backfills epochs missed while down, and `ConfigScoreMaterialiser`, which files a config-score snapshot under the epoch in progress only. Each member writes an epoch's rows in one transaction, and a repeated pass leaves stored rows as they were.
- After a pass the task sleeps until the epoch is due to end, even when a member failed. That is harmless for the HTTP endpoints, which the next pass backfills, but fatal for submission, because the contract rejects past epochs.
- The epoch-aggregates endpoint (`http/state.rs`, `epoch_records`) unions an epoch's config-score and aggregate rows per node and folds the two liveness test kinds into one figure by sample count (`ReportedAggregates::from_rows`).
- The scheduler's role predicates are per test kind (`select_candidates`): mixnode liveness and stress go to nodes declaring the mixnode role, gateway liveness to nodes declaring the gateway role.
- The node refresher describes each bonded node all-or-nothing (auxiliary details v2, roles, build information, a parseable chain address, a noise key, at least one announced IP, the client websocket port for gateways). A failed describe is logged at `debug` with the failing step's context, the node keeps any earlier description, and the per-cycle `bonded_unknown_nym_nodes` gauge counts the failures.
- One signing client, `Arc<RwLock<DirectSigningHttpRpcValidatorClient>>`, sends the agent authorisation transactions, taking the write lock per transaction.
- The aggregation windows already default to 6 hours for liveness and 24 hours for stress, which are the windows the submitted figures should cover. Nothing changes there.

What the performance contract does with a submission, as of the archived `performance-contract-verifiable-routing-view`:

- `BatchSubmit { epoch, data }` checks, in order: the sender is an authorised monitor (`NotAuthorised`); the epoch is the current mixnet epoch (`EpochNotCurrent`) and its transition has not begun (`EpochInTransition`, raised once the mixnet's epoch status leaves "in progress", before the interval advances). An empty batch then returns success without touching anything, cursor included. Otherwise the first node is checked against the sender's cursor (`StalePerformanceSubmission`), and then each node in turn: its measurements carry at least one kind (`EmptyNodeSubmission`, a property of one node, not of the batch); node ids strictly ascend (`UnsortedBatchSubmission`); and the node is bonded and not unbonding (`check_node_existence`), else it is skipped. The cursor then moves to the batch's last node, even when that node was skipped, and `LastSubmission` is updated.
- A skipped node does not fail the transaction: it is only listed in the `BatchSubmissionResult` the handler returns. So a node that starts unbonding between the orchestrator planning a chunk and the chunk landing costs nothing. The single `Submit` is a batch of one that turns the skip into `NodeNotBonded` and reverts; the orchestrator never uses it.
- The cursor is `NetworkMonitor { address }` -> `info.current_submission_metadata { last_submitted_epoch_id, last_submitted_node_id }`. It is initialised when the monitor is authorised, to the epoch current at that moment and node 0, and `info` is `None` for an address that is not authorised, whether never or retired.
- Values are stored as whole percents (`to_stored`: rounded at the second decimal, then truncated).
- `Weights::score` requires a config value and gives a kind weighted zero no share. Rewarding uses the node's bundle at the requested epoch whenever one exists, even an unscoreable one (`resolve_source_bundle`), and only walks back to earlier epochs, at most 24 below R (the newest epoch holding any bundle), when there is none.

Fees, rough: about 12k gas per node, so about 12M gas per epoch at ~1000 nodes, about 0.3 NYM per epoch at 0.025 unym/gas, about 2.5k NYM per year.

## Goals / Non-Goals

**Goals:**

- Every epoch, the measured figures for the epoch in progress land in the contract within minutes of its start, resuming across restarts and failed transactions without any local record of what landed.
- A fault on the measuring side becomes missing data, which the contract renormalises over or falls back from, never zeros.
- An incomplete epoch and a guard trip are visible from the orchestrator's metrics, and every bonded node missing from a submission can be matched to a reason from its logs.

**Non-Goals:**

- **Past epochs.** The contract accepts only the epoch in progress; backfilled epochs stay served over HTTP only.
- **Weighting.** Combining the kinds into one figure is the contract's job, under its on-chain weights.
- **Changing what is materialised or served.** The aggregates, the config scores and the HTTP API are unchanged.
- **Contract or wire changes.** The contract is used as archived.
- **Watching the account balance.** An external watcher already alerts on the account, so the orchestrator neither logs nor exports its balance.
- **Warming the capability cache at startup.** A cold start has nothing to submit for its first epoch anyway (Decision 3).
- **Coordinating several orchestrators.** Each authorised monitor has its own cursor, and the contract combines monitors by median.
- **The consumer.** nym-api reads the submitted data in `nym-api-performance-contract-consumer`.

## Decisions

### 1. Submission is the last step of the materialiser's pass, while the epoch is in progress

`Materialiser` gains an optional third step, `Option<PerformanceSubmitter<S>>`, which is `None` when submission is off (Decision 8). The pass that read the interval runs it after both members, only when both returned `Ok`, for `interval.current_epoch_absolute_id()` of that same reading. Before submitting, the pass reads the mixnet's epoch status (`get_current_epoch_status`, through the query client it already holds), and submits only while the epoch is in progress. The materialiser keeps its name; its doc says it also submits what it materialised.

The figures for an epoch are produced by this pass as the epoch begins, and are only accepted by the contract while that epoch is in progress, so submission belongs right after them. Taking the epoch from the same reading means the submitted figures are exactly those filed under that epoch. Running only after both members succeeded means a submission is never built from half a materialisation: each member writes all of an epoch's rows or none, but a failed member has written none.

The status check matters because an epoch's transition can stall for hours when nym-api's epoch operations fail, and the contract rejects every submission for the sealed epoch throughout. Without it, every retry during that time would simulate a doomed transaction and log an error. With it, a pass that finds the epoch in transition skips the submission and has not failed: the epoch is lost either way, and the incomplete-epoch signal (Decision 9) already reports it. A status that cannot be read fails the pass, as any failed step does: treated like the in-transition skip, it would leave the next look at the epoch's end, and one failed read as the epoch begins would lose the epoch's submission. Submitting anyway and leaving the contract's own `EpochInTransition` check to reject it would end in the same one-minute retry when the RPC is down, since the cursor read fails next, and would blur "only while in progress" into "unless known to be in transition".

Alternative considered: a separate submission task, woken by the materialiser through a channel or polling storage for the epoch's rows. It needs its own loop, its own interval reading (which can disagree with the materialiser's across a transition), its own retry, and a readiness signal, and buys nothing, since submission happens once per epoch, straight after materialisation.

### 2. A failed pass is retried after the one-minute floor

Each pass reports whether every step succeeded. When one failed, the next look is after `MIN_CHECK_INTERVAL` (one minute), the floor already used when the interval cannot be read or the epoch is overdue, instead of at the epoch's end. Every step runs again on the retry:

- the aggregate range is empty once the epoch is stored, so a stored epoch is not recomputed;
- the config-score insert leaves stored rows as they were, and adds rows for nodes whose standing was cached since the previous pass, as any re-run does today;
- submission resumes from the contract's cursor (Decision 5).

Retries continue until a pass completes. Once the epoch ends, the next reading names the next epoch and the pass moves on. A pass that skipped submission because the epoch is in transition has completed; it still looks again after the floor, because an epoch past its due end is overdue.

A failure costs one interval read and a few queries a minute, and an error line. A transaction the contract would reject fails in simulation, before broadcast, so it costs no fee.

Alternatives considered: retrying only the failed step, which needs per-step state carried across ticks when every step is already safe to repeat; a configurable delay or a backoff, which buy nothing over a one-minute floor against a one-hour epoch.

### 3. Measured values only, with a config score and at least one of liveness or stress

A node's submission carries its config score, its liveness (the liveness test kinds the guard kept, combined as served) and its stress, each only when materialised for the epoch. An absent figure is omitted, never submitted as zero: an aggregate exists only where runs exist, and a run that failed already scores 0, so absence means unmeasured rather than broken.

A node is submitted only with a config score and at least one of liveness or stress. The contract uses a node's bundle at the epoch whenever one exists, even one it cannot score, so an unscoreable bundle would block the fallback to the node's last scored epoch.

This rule assumes no kind is weighted zero. `Weights::score` gives a zero-weighted kind no share, so under such weights a node carrying only that kind and its config score would be unscoreable; the initial weights weight both kinds, and if `UpdateWeights` ever sets a zero, this rule has to judge scoreability with `Weights::score` under the epoch's weights instead.

A node deferred for an uncached on-chain standing has no config score for the epoch and waits for the next one, unless a retry or restart reruns the config step and scores it while its id is still above the cursor. Deferral is not a failure and triggers no retry. It mostly follows a start against a discarded database, and such a start has nothing to submit for its first epoch anyway: that epoch's window precedes the start, when nothing had run. By the next epoch the node refresher has described the nodes and the capability sweep has cached them.

### 4. A coverage guard per test kind

For each test kind (mixnode liveness, gateway liveness, stress), coverage is the share of eligible nodes with a reachable aggregate for the epoch:

- eligible: described when the pass runs, with the role the scheduler assigns that test kind to (`select_candidates`), read through a new storage query of the described nodes' roles;
- reachable: the node's aggregate of that test kind scores above zero.

A test kind whose coverage is below `--performance-submission-coverage-threshold` (default 0.5) is omitted for every node: its rows are dropped before the shared fold (Decision 6), so a dual-role node's liveness then comes from its mixnode runs alone, and the node rule (Decision 3) applies to what remains. With every test kind omitted, nothing is submitted for the epoch and the contract's fallback serves. In code, coverage over an empty population is 0, never `0.0 / 0.0`, which is NaN and would compare as not below the threshold.

A fault on the measuring side, such as agents that cannot reach anything or an authorisation not yet picked up, records failures. They score zero and would look like a network-wide outage. Omitting the test kind turns broken measurements into missing ones, which the contract renormalises over. The guard works per test kind rather than per contract kind because the two liveness probes are separate failure domains: the mixnode probe loops sphinx packets through the node, the gateway probe opens a client session. With a per-kind guard, a broken gateway probe could leave combined liveness coverage above the threshold while every gateway reads zero. Eligibility follows the scheduler, so a node a test kind never tests cannot count against it. The default matches nym-api's availability threshold, one threshold for every test kind as there, validated to lie in `[0, 1]` at startup.

Eligibility is read from today's descriptions rather than those in force during the window, which are not kept. The difference is the nodes described or dropped within the last window, small next to a fleet-wide fault.

Omitting gateway liveness leaves gateway-only nodes with nothing to submit while mixnodes are still submitted, so R keeps advancing. The contract serves those gateways their last values for 24 epochs, after which they fall outside its lookback and get no rewarding score. Switching liveness off (`--liveness-enabled false`) has the same effect, since gateway liveness then has no figures and its coverage is zero. This is accepted: the omission is visible from its first epoch (Decision 9), which leaves about a day to fix a fleet-wide probe fault, and switching liveness off for longer than that is a deliberate choice with this consequence. Omitting mixnode liveness or stress never empties a node, because a mixnode keeps the other.

### 5. Order, chunks, and resume from the contract's cursor

- The epoch's plan lists its nodes in ascending node id.
- At the start of each submission step, the cursor is read with `NetworkMonitor { address }`. `info: None` fails the step (the account was retired after startup). With the cursor at `(E, n)`: when `E` is the epoch being submitted, nodes at or below `n` are dropped; when `E` is earlier, every node stays; when `E` is later, the step fails, because this orchestrator's interval reading is behind the chain.
- What remains goes out in chunks of `--performance-submission-batch-size` nodes (default 200), each one `BatchSubmit`, one after another. A failed chunk ends the step with an error, and the later chunks wait for the retry (Decision 2). A step with nothing left sends nothing and counts as complete.

The contract checks each batch's first node against the cursor and moves the cursor to the batch's last node, even when it skipped that node. So the cursor alone tells a retried or restarted step exactly what has landed, including a transaction that landed after the client gave up on it, and the orchestrator keeps no local record that could disagree with the chain. A transaction that succeeded means its whole chunk was taken, so the submitter reads nothing from its response.

At about 12k gas per node plus the digest load and save, a 200-node chunk costs roughly 2.5M gas, about 3.3M requested once the simulation multiplier applies, well below the block limit (the mixnet's rewarding transaction alone uses about 13M). That is about five transactions per epoch at ~1000 nodes, each waiting for its block. A larger chunk loses nothing on failure, since the next pass resumes from the cursor. The size is configurable; the sandbox drill measures it.

Alternatives considered: one transaction per epoch, near the block limit and all-or-nothing; one `Submit` per node, about 1000 transactions per epoch, and an unbonding node reverts it instead of being skipped.

### 6. The submission reads the same fold the HTTP API serves

The per-node union of an epoch's config-score and aggregate rows that `epoch_records` builds moves into `aggregation`, yielding per node in ascending order the optional config-score row and its `ReportedAggregates`. `epoch_records` maps that to the wire type, and the submission maps it to `NodeSubmission`s after the guard has dropped the omitted test kinds' rows. With nothing omitted, the submitted liveness is therefore the served one, which the shadow period compares against nym-api.

Values are converted at that boundary. Aggregates and config scores are `f64` in `[0, 1]` by construction; each is clamped to `[0, 1]` against float error and converted with `Percent::naive_try_from_f64` (`nym-contracts-common`, feature `naive_float`). The contract stores whole percents, so a stored value may differ from the served one by up to half a percent.

### 7. One signing account, locked per transaction

The submission sends from the orchestrator's existing account, through its one signing client: a second client on the same account would race the agent authorisations for the account sequence. That client becomes a `Clone` newtype, `SigningClient`, built from the client config and the mnemonic and held by the orchestrator, the agent-authorisation handler and the submitter in place of the bare `Arc<RwLock<DirectSigningHttpRpcValidatorClient>>`. Its `read` and `write` mirror the lock for the existing call sites, and it implements the validator client's `PerformanceSigningClient`, taking the write lock for one transaction, and `PerformanceQueryClient`, taking the read lock for one query. It implements no other contract's traits: nothing else is generic over them, and the agent authorisation sends two messages in one transaction, which no trait method covers. Each trait has a single required method; the provided ones the submitter uses (`batch_submit_performance`, `get_network_monitor`) come for free. The submitter is generic over `C: PerformanceQueryClient + PerformanceSigningClient`, so tests implement both traits on a fake (Decision 10). The lock is held per chunk, so an agent authorisation waits for at most one chunk's inclusion.

Alternatives considered: a local seam trait, like the nym-api submitter's `BatchSubmission`. The validator client's traits are already the narrowest seam, and a fake implementing them exercises the real message construction. Holding the write lock across every chunk of a step, so no other transaction lands between them: interleaving breaks nothing, since the lock per transaction already serialises the account sequence and an agent authorisation touches neither the performance contract nor its cursor, while an agent authorisation, whose HTTP request waits on the same lock, would wait for the whole submission (minutes when a chunk hangs until its broadcast timeout), and the submitter could no longer be tested through a fake implementing the traits.

### 8. On exactly when the configuration enables it, authorised at startup

Submission is on exactly when `--performance-submission-enabled` (environment `NYM_NETWORK_MONITOR_PERFORMANCE_SUBMISSION_ENABLED`) is `true`; it defaults to `false`, so existing deployments run unchanged until an operator turns it on. It takes an explicit value, like `--liveness-enabled`. The network details are assumed to carry the performance contract's address: whether to submit is an operator's decision, taken once the account is authorised and funded, not something inferred from which addresses a network happens to configure. A deployment that enables submission without the address fails the startup check below, as a misconfiguration.

Alternative considered: submission on exactly when the network details carry the address, with no flag. It ties an operational decision to network configuration, and makes every consumer of the address treat its absence as a mode rather than an error.

With submission on, startup verifies, right after the network-monitors contract check and before the identity-key reconciliation, that `NetworkMonitor { address }` returns an entry for the orchestrator's account. It retries with the same `chain_authorisation_check_max_attempts` and `chain_authorisation_check_retry_delay` as that check, for the same reason: the admin's authorisation transaction may land after the deploy. When the budget is exhausted, startup aborts before any task starts. A missing authorisation is then a failed deploy rather than an epoch of rejected transactions, and it fails before the reconciliation transaction and its wait. Aborting also stops agent testing and the nym-api feed, which is accepted: that feed is the intermediate path, and the contract is the one this deployment exists for.

The two orchestrator checks sit together because both ask whether the account is authorised on chain. The `nym-network-monitor` spec's startup requirement lists every gate in order, so it gains this one, pointing to this capability for the details. Its task-list requirement was already stale: it names four tasks where six run, missing the chain-capability refresher and the materialiser. It is corrected here, renamed accordingly, and says that submission runs inside the materialiser's pass rather than as a seventh task.

### 9. Signals where the code already has the facts

Every signal is opportunistic: each is emitted where its facts already exist, and one that would need code restructured, to emit it or to test it, is dropped instead. All carry the `nym_network_monitor_` prefix.

- **The incomplete epoch.** Gauges `performance_submission_epoch` (the epoch in progress the last pass read, set by the pass straight after it reads the interval, before either member runs, whenever submission is on) and `performance_submission_last_complete_epoch` (the newest epoch with nothing left to send, set by the submitter). The alert is `performance_submission_epoch > performance_submission_last_complete_epoch` for longer than the deployment tolerates. The first is set by the pass rather than by the submission step because the step runs only after both members succeeded and while the epoch is in progress: set by the step, an epoch whose config step failed throughout, or one first read during a stalled transition (after a restart), would never move the gauge, and the alert would stay quiet while the epoch is lost. The cost is one false alert: a restart inside a transition, after the epoch had fully landed, shows that epoch ahead of a last complete epoch of 0 until the next epoch completes, which takes minutes normally and lasts through a stalled transition, when nothing can be submitted anyway. Completeness is tracked by epoch rather than by chunks planned and landed, because a retried or restarted step re-chunks only what is left, so chunk counts do not add up per epoch. A step that finds the cursor already past its plan sets the last complete epoch too, so a restart restores both gauges on its first pass.
- **A guard trip.** One gauge per test kind, `performance_submission_mixnode_liveness_omitted`, `performance_submission_gateway_liveness_omitted` and `performance_submission_stress_omitted`, 1 when the guard omitted it for the epoch, set where the guard decides.
- **Chain outcomes.** Counters `performance_submission_chunks_landed` and `performance_submission_chunk_failures`, in the chunk loop.
- **The plan.** One `info` line per submission step with what the plan builder already knows: the epoch, each test kind's coverage, the nodes planned, and the nodes it skipped for no config score or no measurement, if those come out of the builder naturally. No gauges.
- **Describe failures.** Both per-node describe-failure logs, the timeout and the error, move from `debug` to `info`. They already carry the node id and the failing step's context, and `bonded_unknown_nym_nodes` already counts them per cycle. During the shadow period every node nym-api scores but the contract does not has to be matched to a reason before the operator notice goes out; that is a search per node, which logs serve better than counts.

### 10. Testing

Offline only, with in-memory storage and deterministic fixtures.

- **The plan**, as pure functions over rows and roles:
  - the node rule: an unmeasured kind is omitted, a measured zero is submitted, a node without a config score is not submitted, a config score alone is not submitted;
  - the guard: a test kind below the threshold is omitted for every node; a gateway-only node is not eligible for mixnode liveness or stress; a zero is not reachable; a dual-role node's liveness comes from its mixnode runs alone once gateway liveness is omitted; an empty population is omitted, pinning the NaN trap;
  - ascending order, and the conversion's clamp.
- **The resume rule:** a cursor at the same epoch drops nodes at or below it; one at an earlier epoch keeps every node; one at a later epoch is an error.
- **The submission step**, through a test type implementing `PerformanceQueryClient` (answering `NetworkMonitor`) and `PerformanceSigningClient` (recording each `BatchSubmit` and returning a bare `ExecuteResult`): 450 nodes go out as chunks of 200, 200 and 50 in ascending order under the epoch; a cursor mid-epoch resumes after it; a failing second chunk stops before the third, and the next step resumes from the cursor the first left; no network-monitor entry fails without sending; a cursor at the last planned node sends nothing and returns `Ok`; a cursor at a later epoch fails without sending. The blanket impls over `CosmWasmClient` do not conflict with impls on a type that is not one, as the `verify_chain_read` tests in `nym-contract-anchor` show for `PinnedMixnetQueryClient`.
- **The pass:** the delay after a pass is a pure function of its outcome and the interval: the floor after a failure however far the epoch's end, the epoch's end after success. A pass whose config step fails, through a test `MixnetQueryClient` failing every query, submits nothing and reports failure; its stored aggregates are unchanged by the retry.
- **Config and storage:** a coverage threshold outside `[0, 1]` is rejected; the described nodes' roles are read for every described node and for no other.

Not unit-tested: the startup check's retry loop, untested for the network-monitors contract too; the newtype's two one-line lock delegations; the in-transition branch, a single condition whose test would need a mixnet fake answering the config-score queries too; the describe log level; the metric writes, since the crate's metrics live in a process-wide registry with no read API, so the tests check the facts they are set from instead (the plan's omitted test kinds, `submit` returning `Ok` once nothing is left, a pass reporting failure).

## Risks / Trade-offs

- [The account runs out of fees] -> Every transaction fails, the last complete epoch stops advancing, and the contract freezes every node at its last value (its R-anchored fallback) rather than zeroing it. The external account watcher alerts before that.
- [A persistent failure retries every minute until the epoch ends] -> Each attempt costs one interval read and a few queries, and a transaction the contract would reject fails in simulation, before any fee; the cost is an error line a minute. A stalled transition does not count: the status check skips the submission without an error.
- [A gateway-liveness omission or liveness switched off for more than 24 epochs] -> Gateway-only nodes lose their rewarding score (Decision 4). The guard's gauge reports the omission from its first epoch.
- [A retry adds config scores for nodes deferred on the first pass] -> The existing re-run semantics. Such a node joins the plan when its id is above the cursor, and otherwise waits for the next epoch.
- [A cold start submits nothing for its first epoch] -> That epoch has no runs in its window; the next one is normal unless the start fell within minutes of the transition, and the contract's fallback serves meanwhile.
- [The guard's population is today's descriptions, not the window's] -> Only nodes described or dropped within the last window differ, which a fleet-wide fault outweighs.
- [A kind is weighted zero] -> The node rule would then submit bundles the contract cannot score; the rule records the assumption (Decision 3), and any `UpdateWeights` with a zero has to revisit it.
- [A cursor read from a lagging RPC node] -> The batch fails simulation with `StalePerformanceSubmission` and the retry reads again.
- [Submission in the materialiser's task delays its next look] -> By the time the chunks take, about half a minute at ~1000 nodes, against a next look scheduled at the epoch's end anyway.
- [The gas per chunk is an estimate] -> The batch size is configurable, and the sandbox drill measures it.

## Migration Plan

No schema migration and no wire change; the cursor lives on chain.

Deployment, after the contract: instantiate it with the initial weights and run the mixnet migration, authorise the orchestrator's account (`AuthoriseNetworkMonitor`) and fund it, make sure the orchestrator's network details carry `PERFORMANCE_CONTRACT_ADDRESS`, set `--performance-submission-enabled true`, and restart. Startup refuses to run if the authorisation has not landed within the retry budget.

Drills, sandbox first: restart the orchestrator mid-submission (it resumes from the cursor); stop the agents for longer than the liveness window (the guard trips, nothing is submitted, the contract's fallback serves).

Rollback: set `--performance-submission-enabled false` and restart. Submission stops and the contract falls back to last-known values.

## Open Questions

None for this change.
