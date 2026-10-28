## Context

The `network-monitor-epoch-aggregates` change gave the orchestrator a per `(node, epoch)` view of the two probe-measured performance inputs, liveness and stress, materialised at each epoch transition and served from `/v1/aggregates`. Its Open Question 2 named the remaining input, config score, as the next to move, and the response type was shaped to receive it: `NodeEpochAggregates` carries named per-kind fields rather than a uniform map precisely so config score can arrive as a field of its own shape.

Config score is unlike the probe kinds in two ways that drive this design. First, it is not measured: it is a deterministic function of a node's self-reported configuration (reported version, binary name, terms acceptance) and its on-chain standing (balance, feegrant), scored against the version history and formula params held in the mixnet contract. Second, the orchestrator sees none of those inputs today. It fetches each bonded node's describe response on a refresh sweep but extracts only networking fields; it holds a nyxd client but queries only balances for its own account and the contracts it authorises against. So the bulk of the work is sourcing data, not extending aggregation.

The authoritative implementation is nym-api's `calculate_config_score`. The whole point of producing config score in the orchestrator is to compare it against nym-api's for the same nodes before anything depends on it, exactly as the parent change compares liveness and stress. That comparison is only meaningful if the two numbers are produced by the same code, so parity is a hard requirement rather than an aspiration.

## Goals / Non-Goals

**Goals:**

- Produce a per `(node, epoch)` config score identical to the one nym-api computes for the same node from the same inputs.
- Share the scoring logic between nym-api and the orchestrator so the two cannot silently diverge.
- Source the version, terms, binary and chain-standing inputs the orchestrator does not see today, without adding a per-node chain query to every epoch.
- Serve config score as a first-class field of the epoch aggregates, decomposed into the subcomponents that explain it.
- Keep the orchestrator's database disposable.

**Non-Goals:**

- Submitting config score to the performance contract. That is a later change, together with liveness and stress submission.
- Combining config score with the probe kinds into a single performance figure. That is done by a separate entity and is out of scope here; this change only produces and serves the per-epoch config score.
- Removing config score from nym-api. Both compute it in parallel until the comparison says otherwise.
- Any change to how config score is defined. This change relocates and shares the existing definition, it does not revise it.

## Decisions

### Decision 1: The scoring logic is extracted into a shared `nym-config-score` crate

**Choice.** Create `common/nym-config-score`, a non-wasm leaf crate depended on by both nym-api and the orchestrator. It exposes a `ConfigScoreCalculator` and the token-sufficiency predicate. nym-api's `calculate_config_score` and `has_sufficient_tokens` are refactored to call it, so a single function computes the number on both sides.

**Why.** The comparison the change exists for is only interpretable if any discrepancy points to a data difference, never to a scoring difference. A ported copy, however careful, reintroduces exactly the divergence risk the comparison is meant to expose. Sharing the code removes that class of error by construction, and config score is on a path to leave nym-api entirely, so the shared crate is where the definition will continue to live once nym-api stops computing it.

**Alternative considered.** Porting the ~15 lines of combining logic into the orchestrator and reusing only the version-distance math that is already shared. Rejected: cheaper now, but it puts the definition in two places during the precise window when they must agree exactly.

### Decision 2: The shared crate is pure - values in, score out, no I/O

**Choice.** The calculator holds the per-pass policy and chain state and takes per-node values:

```
ConfigScoreParams { minimum_balance, chain_interactions_penalty, version_weights, formula_params }
ConfigScoreCalculator { params, version_history: Vec<HistoricalNymNodeVersionEntry> }
  ::new(params, version_history)
  ::has_sufficient_tokens(&self, balance: Option<&Coin>) -> bool
  ::score(&self, node: &NodeConfigInputs) -> ConfigScoreOutcome   // { score, versions_behind }
NodeConfigInputs { reported_version: Option<&semver::Version>, runs_nym_node, accepted_terms, balance: Option<&Coin>, is_feegrant_grantee }
```

It performs no retrieval and defines no client traits.

**Why.** A pure function is testable with literal inputs and no mocks, which is strictly better than mockable I/O. Retrieval is not shared behaviour, only shared data shape: nym-api gathers from an in-memory describe cache and a lazy chain cache, the orchestrator from SQL tables filled by separate tasks, and a trait spanning both would be a leaky abstraction earning nothing the pure function does not already give. The testing seam belongs at each consumer, where the codebase already mocks the narrow client bound (the materialiser tests already mock `MixnetQueryClient`).

**Consequence.** Each consumer parses the reported version itself and owns the distinction between "no describe" and "unparseable version", passing `reported_version: None` for both, since the calculator maps None to a zero score and the flags that tell them apart live on each consumer's own response type.

### Decision 3: Version distance stays in the contract crate; the calculator holds the whole release chain

**Choice.** The calculator computes `versions_behind` internally by calling `OutdatedVersionWeights::versions_behind_factor` on the `Vec<HistoricalNymNodeVersionEntry>` it holds. It does not accept a pre-computed integer, and it does not slim the history to a list of versions.

**Why.** `versions_behind_factor` reads each entry's precomputed `difference_since_genesis` cache, not just its semver - that cache is what the contract crate's own comment calls the "external caching" the function relies on to stay efficient. A slimmed history would force reimplementing that cache and reintroduce the divergence Decision 1 removes. Since the distance step is unavoidable and the calculator already depends on the contract crate for the weights and formula params, holding the entries too is one more type from a crate it already imports, and it keeps the whole computation in one place so consumers pass a version and get a score back with `versions_behind` for their decomposition.

**Alternative considered.** Splitting the distance step out to the consumer and passing `versions_behind: u32` into the calculator. Rejected: it does not avoid the computation, it duplicates the two-step across both consumers and spreads the definition.

### Decision 4: The crate does the float scoring the contract crate deliberately avoids

**Choice.** The `penalty ^ (versions_behind ^ scaling)` formula, the gates and the chain penalty live in `nym-config-score`, not in `nym-mixnet-contract-common`.

**Why.** The contract crate keeps float math out for wasm determinism - which is why it stores the formula params as fixed-point `Decimal` and leaves the `powf` conversion to its non-wasm consumers. Putting the scoring there would drag float math into a wasm-compiled crate. A separate non-wasm leaf crate that depends on the contract crate for the types and the integer distance is the natural home.

### Decision 5: Config score is a snapshot materialised once per epoch, not a windowed replay

**Choice.** At each epoch transition the config-score step reads the current node state and chain-capability cache, computes a score per bonded node, and persists it. It is served thereafter, never recomputed.

**Why.** Materialise-once matches the parent change's stability rule (its Decision 4): a value handed to a consumer must not change underneath it. Config score has no sample window to replay - it is a function of state as it stands - so the snapshot is taken from the latest available inputs at the moment the epoch opens.

Anchoring the snapshot at the epoch's START, rather than recomputing continuously as nym-api does, is the config-score equivalent of the parent change's start-anchored window (its Decision 1). It is required for the same reason: a value for epoch E must exist before E closes, because the eventual consumer reads it the moment E ends and cannot wait for it to be produced. An end-anchored snapshot that would match nym-api exactly is rejected for the same reason the parent rejected an end-anchored window. The divergence this introduces is smaller than for the probe kinds, since config inputs are slow-moving - a version, a terms flag, and a coarse balance threshold - so the only nodes that differ from nym-api mid-epoch are those that actually changed their configuration within the epoch.

**Consequence.** Backfill differs from the probe kinds. Liveness and stress replay retained samples, so a backfilled epoch reconstructs the value that epoch would have had. Config score has no such history, so a backfilled epoch is computed from current state rather than the state at the time. This is accepted while nothing consumes the value, and it is bounded: backfill only runs for epochs missed while down. The parent capability's backfill path is reused, so config score is materialised for the same epochs as the probe aggregates.

### Decision 6: Config score has its own table and its own decomposed shape

**Choice.** A new table `mixnet_epoch_config_score` keyed `(mixnet_epoch, node_id)` stores the score and its subcomponents: `versions_behind` (nullable), `accepted_terms_and_conditions`, `runs_nym_node_binary`, `self_described_available`, `has_sufficient_tokens`, `is_feegrant_grantee`. It does not reuse `mixnet_epoch_aggregate` and is not a `TestKind`, so it never touches that table's `('stress','liveness')` CHECK.

**Why.** Config score does not have the score-and-count shape the probe kinds share, and forcing it into `KindAggregate` would discard the subcomponents that explain the number. A node scores zero for several distinct reasons - stale version, unaccepted terms, wrong binary, no describe - and the decomposition is what lets a consumer or operator tell them apart, mirroring nym-api's `ConfigScoreV2`.

### Decision 7: Chain capabilities are cached with a TTL, storing raw balance rather than a sufficiency bool

**Choice.** A `node_chain_capability` table holds, per node, the raw on-chain balance amount and the feegrant flag with a refresh timestamp. A dedicated background task keeps the cache warm on its own interval (default 24 hours) with bounded concurrency (default 8), decoupled from the describe refresh, and the epoch-transition materialiser only ever READS the cache. The `has_sufficient_tokens` decision is derived at score time by comparing the cached balance against the current `minimum_on_chain_balance`.

**Why.** Balance and feegrant are per-node chain queries that do not scale to the whole fleet every hourly epoch, which is why nym-api caches them behind a 24 hour TTL. The orchestrator refreshes them with a warm-keeping background sweep rather than lazily on demand, because materialisation needs every node's capability at once: a lazy cache would trigger a burst of per-node chain queries at the epoch transition, the very moment that already depends on the chain for epoch timing. Storing the raw balance rather than a precomputed bool keeps the cached fact separate from the policy: a change to the token threshold takes effect on the next epoch against the cached balances, with no re-query. The feegrant flag stays a bool because it is inherently boolean.

**Jitter.** Each node's next refresh is due at `refreshed_at + ttl + jitter`, where the jitter is a bounded random offset drawn per refresh. Without it a cold-start population, refreshed together in the first sweep, would all fall due again at the same instant one TTL later and produce a synchronised thundering-herd refresh; the jitter spreads due-times across a window so the sweep's load stays smooth.

**Consequence.** The cache is a rebuildable cache like the aggregate table (Decision 8 of the parent), so an empty cache after a database wipe means config scores are briefly computed as if no node can transact, recovering as the refresh sweep fills it. The `has_sufficient_tokens` bool is still written into each epoch's snapshot row, since that row records the decision made for that epoch under the threshold then in force.

### Decision 8: Describe inputs are captured on the existing refresh sweep

**Choice.** The reported version, binary name, terms acceptance and on-chain address are extracted from the describe response the refresher already fetches, and stored as columns on the `nym_node` record. Terms acceptance and address are already in the `auxiliary_details` the orchestrator fetches today; version and binary name need one added build-information read.

**Why.** These are per-node describe-derived attributes like the networking fields already on the record, refreshed on the same cadence, and cheap since the describe call is already made. Keeping them on `nym_node` and the chain capabilities in a separate table reflects their separate writers and cadences: describe refresh writes the former, the capability task writes the latter.

### Decision 9: Config score is a non-optional field and drives the per-node record set

**Choice.** `NodeEpochAggregates.config_score` is `ConfigScore`, not `Option<ConfigScore>`. Liveness and stress stay optional. When building an epoch's response, config score rows are the primary set - one per bonded node - and the optional probe aggregates attach to them.

**Why.** Config score is computed for every bonded node deterministically; there is no "not measured" case the way there is for a probe kind, because "no self-description" is itself a valid score of zero (nym-api's `unavailable()`). Making it non-optional states that invariant in the type, and it gives the response a natural primary key: a node with a config score but no probe runs this window is still a node with a record, which the previous score-shaped fold could not represent.

**Consequence.** A node that somehow lacks a computed config-score row falls back to an unavailable score of zero rather than being dropped from the response.

## Risks / Trade-offs

- **Backfilled config scores reflect current state, not historical state**, since config score has no sample window to replay. → Accepted while nothing consumes the value, and bounded to epochs missed while the orchestrator was down; the probe kinds' backfill is unaffected.
- **Per-node balance and feegrant queries add chain load.** → Bounded by the TTL cache and capped concurrency, decoupled from the epoch cadence, mirroring nym-api's existing approach.
- **The feegrant check is coarse** - it does not verify the grant is unexpired or covers execute messages, matching nym-api's "good enough first iteration". → Kept identical to nym-api so the two do not diverge; a finer check is a shared improvement in the crate later, applying to both at once.
- **Refactoring nym-api onto the shared crate touches a rewarding-adjacent path.** → The extraction is behaviour-preserving and verified by nym-api's existing config-score tests plus the parallel comparison; the score it produces is unchanged.
- **A node self-reports its version, terms and address**, so it can misreport. → This is nym-api's existing trust model, not a new exposure; the orchestrator reads the same self-reported describe data nym-api does.

## Migration Plan

1. Land `nym-config-score` and refactor nym-api onto it. This is behaviour-preserving and independently verifiable: nym-api's config score is unchanged and its tests confirm it.
2. Land the orchestrator's additive pieces - describe columns, the capability cache and its task, the config-score table, the materialiser step, the response field. Inert: nothing consumes config score, and the probe aggregates are untouched.
3. Deploy and let the capability cache fill and the first epochs materialise. A freshly deployed orchestrator computes config scores as its describe sweep and capability cache populate.
4. Compare the orchestrator's config score against nym-api's `ConfigScoreV2` for the same nodes. Because the scoring is shared, they should agree exactly for a node whose inputs both systems read identically; the expected difference is timing, since the orchestrator snapshots at the epoch transition while nym-api recomputes continuously.
5. Once the comparison holds, the submission change follows, carrying config score to the performance contract alongside the probe kinds. Combining the three into a single performance figure is a separate entity's concern.

Rollback is dropping the two new tables and not reading the response field. nym-api's refactor stands on its own and does not need reverting, since it is behaviour-preserving.

## Open Questions

The three questions raised during design are resolved. The epoch-transition (`E_start`) snapshot is confirmed as the anchor (Decision 5). The capability refresh is a jittered warm-keeping background sweep on a configurable TTL (Decision 7). Combining config score with the probe kinds into a single performance figure is out of scope and done by a separate entity (Non-Goals).

What remains is operator tuning rather than open design: the capability TTL and its jitter span, the minimum balance and the chain-interaction penalty are all configuration, defaulting to nym-api's values and movable without a code change. The comparison in Migration step 4 is read as steady-state agreement, since a node that changes its configuration within an epoch is expected to diverge from nym-api's continuous value until the orchestrator's next snapshot.
