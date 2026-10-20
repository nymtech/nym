## Context

This change is the fourth of five stacked PRs that get network-monitor-v3 measurements into the performance contract and serve them verifiably: the contract side (`performance-contract-verifiable-routing-view`), the retrieval library (`performance-retrieval-client`) and the orchestrator's submission (`network-monitor-contract-submission`), all archived, then this nym-api consumer, then nym-api's attested snapshots and contract-mode annotations (`nym-api-performance-attestation`). They are reviewed separately and merged together. An orchestrator follow-up then adds a reachability input to the config score and faster liveness, and must be deployed before the switch is flipped.

What the performance contract offers, as archived:

- Per-epoch bundles `(epoch, node)` holding every monitor's values per kind (liveness, stress, config). Only the current mixnet epoch is writable, and only while it is in progress, so an epoch is frozen from the moment `BeginEpochTransition` for it executes. Bundles are never removed.
- `RewardingInputs { epoch_id: X, node_id }` resolves a source bundle: the bundle at X if it exists; otherwise, with R the newest epoch not after X holding any bundle, the node's newest bundle in `[R - 24, min(X - 1, R)]`. It scores the source's medians with `Weights::score` under the weights in force at X. Anchoring the lookback at R means a global outage of any length freezes every node at its last value instead of zeroing it. The walk starts at the node's last-known-epoch pointer when that is at or below X, which only saves loads (see Decision 2).
- `EpochPerformancePaged { epoch_id }` pages one epoch's nodes with their per-kind medians. `WeightsAt { epoch_id }` gives the weights in force at an epoch. Weights updates take effect from the next epoch, so an epoch's weights are fixed once it begins. `LastSubmittedMeasurement` returns the block, time and epoch of the newest submission and changes with every submission.

What nym-api does with it today, behind `[performance_provider].use_performance_contract_data` (default off, never enabled anywhere):

1. **Contract mode produces no annotations.** Validation forbids every scoring property, `legacy_v1_routing` included, alongside contract data, but the contract score only enters as the routing component. Every node's applied set is empty, so every node is skipped: zero rewards (`epoch_operations/helpers.rs`, a missing annotation rewards zero) and an empty rewarded set (`rewarded_set_assignment.rs`, a node without an annotation is skipped).
2. **Config would be applied twice.** The contract score already includes the config median, and nym-api multiplies the routing component by its own config score. This stays hidden only because of the defect above.
3. **The fallback never runs.** The contract provider's fallback loop (`start..threshold`, start above threshold) and the cache's warm-up loop (`current_epoch..last`, likewise) are empty ranges.
4. **Rewards are read too early.** `perform_epoch_operations` computes rewards before `BeginEpochTransition`, while the contract seals the epoch on that transaction.
5. **A skipped node loses its annotation.** The refresher's comment says a node with nothing applied keeps its previous annotation, but `NodeStatusCache::update` replaces the whole map. That node is served 0, rewarded zero and left out of the active set. This is latent while `legacy_v1_routing` is enabled, since routing applies to every node.

Scoring of nym-api's own figure is specified in `nym-network-monitor` ("Stored stress-test scores feed node performance and rewarding"), and the v1 monitor's consumers in `network-monitor`. Both say things about contract mode that this change makes untrue.

## Goals / Non-Goals

**Goals:**

- In contract mode, rewarding and active-set selection use exactly the figure the contract's own rule gives, for the sealed epoch, without hundreds of queries per run.
- A failed contract read never pays zero.
- During the shadow period (read flag on, switch off) one route shows, node by node, where nym-api's figure and the contract's disagree, and why.
- Legacy mode stops serving its last figure for a node whose gap is not our fault.

**Non-Goals:**

- **Published performance in contract mode.** Contract mode publishes no annotations until `nym-api-performance-attestation` builds them from its verified snapshots. The switch stays off until then.
- **Attested snapshots, `/v1/performance` routes, the generic snapshot machinery.** All in `nym-api-performance-attestation`.
- **Aliveness.** A dead node is brought down in contract mode by the orchestrator's follow-up (a reachability input to the config score, faster liveness), not by any describe-based filter in nym-api.
- **An outage guard in nym-api** (Decision 11).
- **Contract or wire changes.** The contract's behaviour and messages are unchanged. Its resolution code moves into the common crate.
- **Persisting the window.** It is reloaded at startup.

## Decisions

### 1. An explicit read flag, which the switch requires

`[performance_provider].read_performance_contract` (default `false`) turns on the window (Decision 3) and the contract side of the gap route (Decision 10), and later the attested producer. With it on, the network details are assumed to carry `PERFORMANCE_CONTRACT_ADDRESS`, and a missing address fails startup. `use_performance_contract_data` (the switch) keeps its name and default. Validation rejects it without the read flag, and it decides whether nym-api uses its own figure or the contract's (in contract mode nym-api computes none of its own, Decision 8).

Today's rule that no `[performance_provider.scoring.*]` property may be enabled alongside the switch is dropped. It existed because the old contract provider could not supply stress or liveness. In this design nym-api computes no figure of its own in contract mode (Decision 8), so the scoring section is simply unused there. It is validated exactly as in legacy mode and has no effect while the switch is on, which its template comment says. The flip is then one line, and so is a rollback, which returns to exactly the weights that ran before rather than to weights restored from memory. Keeping the rule was considered: the config would never show a property as enabled while it does nothing, but the flip would also mean disabling `legacy_v1_routing` (on by default), and a rollback would mean re-enabling it with its old weights.

Enablement is never inferred from the address being configured, as for the orchestrator's submission. Whether nym-api reads the contract is an operator's decision, taken once the contract is deployed and fed, and making it a flag also keeps mainnet nym-api booting before the contract exists (mainnet's address is empty).

Alternative considered: reading the contract whenever an address is configured, the position before this change. It makes every consumer of the address treat its absence as a mode rather than an error.

### 2. The contract's resolution rule, shared through the common crate

The source-bundle resolution moves from `contracts/performance/src/storage.rs` (`resolve_source_bundle` and its helpers' window arithmetic) into `nym-performance-contract-common`, beside `MAX_FALLBACK_LOOKBACK_EPOCHS` and `Weights::score`, as one pure function, `resolution::resolve_source`. It takes the requested epoch and three lookups, each called only when the rule needs it:

- `medians_at(epoch) -> StdResult<Option<KindMedians>>`: the per-kind medians of the node's bundle at an epoch;
- `newest_measured(at_or_before) -> StdResult<Option<EpochId>>`: R;
- `last_known() -> StdResult<Option<EpochId>>`: the node's newest-bundle hint.

It returns `StdResult<Option<ResolvedMedians>>`, the common crate's existing `{ epoch_id, medians }`, built from the epoch at which it found the medians. It is not generic. The bundle is passed as medians because that is all either caller needs: the contract's only caller, `resolve_rewarding_inputs`, turns the found bundle into `ResolvedMedians` anyway, and the window holds medians only. The contract computes medians only for the bundle the walk stops at, as today. The error is `cosmwasm_std::StdError` because the contract's three reads are cw-storage-plus calls that already return it, and its call site converts it with `?` through `NymPerformanceContractError::StdErr`.

The order of calls is the contract's today: the bundle at X, then (only on a miss, and only for X > 0) R, then the hint, then the walk. A hit therefore still costs one load, which matters once the mixnet contract reads `RewardingScore` itself. `newest_measured` takes the epoch and `last_known` takes nothing, so the two `Option<EpochId>` lookups cannot be swapped without a compile error. The contract passes closures straight over its storage (the bundle map's `may_load` mapped to its medians, its one descending key step for R, the pointer map). nym-api passes closures over its window that never fail, with a hint that is always `None`, and discards the error type with `.ok().flatten()` under a one-line comment saying so. Scoring stays where it is: both callers score the result with `Weights::score` under the weights at X.

Alternative considered: generic bundle and error types (`B` carrying its own epoch, `E` per caller), so that nym-api could pass `Infallible`. It buys nym-api an irrefutable match at the cost of two type parameters and a type annotation at its call site, and the contract, the only fallible caller, has `StdError` natively.

The hint never changes the result. With a pointer `p <= X`, the bundle at X was checked first, so `p < X` and `p <= R`, and the hint-free walk from `min(X - 1, R)` meets `p` first because `p` is the node's newest bundle. With `p` below the floor, neither walk finds anything. With `p > X` or no pointer, the contract already walks without it. A common-crate test pins this over a small exhaustive space of bundle layouts and pointers.

The contract's behaviour does not change, so the `performance-contract` spec needs no delta. Its `RewardingInputs` requirement specifies the resolution, `RewardingScore` still goes through the same function, and no requirement says where the function lives. The contract's existing resolution tests are the regression suite for the move, followed by the wasm check (contracts are pinned to MSRV 1.86, so the shared function must build there).

Alternatives considered: per-node `RewardingScore` or `RewardingInputs` queries, which run the contract's own code but cost a few hundred queries per rewarding run and about a thousand per gap-route refresh, and even "page X, single queries for the misses" bursts about a thousand at every epoch start, when no node has been submitted yet; and a nym-api re-implementation of the rule, which drifts, as today's dead fallback loop already has.

### 3. A local window of the contract's per-epoch medians

The window holds, per epoch, each node's per-kind medians from `EpochPerformancePaged`, for `[R - 24, R]` plus the current epoch X, together with the weights at X. It does not keep the pages' scores, which are under each epoch's own weights. An epoch read with no nodes is held as absent, so it never counts as R. It replaces the per-epoch history cache (`node_performance/contract_cache`) and `ContractPerformanceProvider`, which are deleted along with their dead loops.

- **Startup load.** Read `LastSubmittedMeasurement` and the weights at X. With data, let E be its epoch. Submissions go only to the current epoch, so every bundle lies at or below E. Page E, then earlier epochs until the first one at or below X holding nodes, which is R, and continue down to `R - 24`. E is paged even when X is lower (the mixnet contract cache an epoch behind): X later reaches E without a new submission, and nothing would page E then. E normally holds data. It is empty only if every node of the newest batch was skipped as not bonded, and the walk down covers that. Without data every epoch is empty, so there is nothing to walk. X is the mixnet contract cache's current epoch (the window waits for that cache, as the old refresher did). No epoch below `R - 24` is ever needed, however long an outage lasted, because the walk is anchored at R. The walk is capped: if none of the 25 epochs from its start holds nodes, the contract is not being fed (a whole batch skipped is rare, 25 in a row are not), and the load fails instead of walking on, which could reach epoch 0 at one query per epoch. Alternatives considered: walking from X, which reads as R's definition but pays one empty page per epoch of an outage at every startup and walks to epoch 0 when nothing lies at or below X; and paging X separately when it lies above E, which is known to be empty.
- **Keeping current.** Every `contract_polling_interval` (150 s, unchanged) the refresher runs one poll step: it reads `LastSubmittedMeasurement` and takes the current epoch. When the epoch has moved on, the weights at the new X are read; X itself needs no page, since any submission to it changes the last submission. When the last submission has changed, every epoch from the recorded submission's epoch to the new one's is re-paged. Only the current epoch takes submissions, but it can move on between two polls, so each epoch that was current at some point in between may have received one: a late chunk for X followed by the first for X + 1, or several epochs while this nym-api's polls failed. That is normally one epoch, and two at an epoch boundary. A window that has seen no submission is loaded afresh instead. R is then the newest epoch holding nodes, and epochs below `R - 24` are dropped (X never is, since R is at most X). The last submission recorded is the one read before the pages it triggered, so a submission landing while they are read shows up as a change at the next step instead of being masked. A step reads everything before it changes anything, so a failed step leaves the window as it was: applying half of one could, for instance, record a new X whose weights were never read. X never moves backwards: the epoch operations poll with the chain's X and then read, and a refresher poll slipping in between with a mixnet contract cache an epoch behind would otherwise move the rewards being computed to the previous epoch. Re-paging only the new submission's epoch was rejected. It is exact on the nym-api that advances the epochs, whose epoch operations poll X once it is sealed while the chain cannot move past X before that poll succeeds, but any other nym-api (the gap route in shadow mode) would miss those chunks.
- **Exactness.** Submissions go only to the current epoch, so every epoch below X is frozen once paged, and the epoch operations run one more poll step once X is sealed, before rewarding (Decision 5). The pages are not pinned to one height. A submission landing between two pages of X is picked up when the next step sees the last submission change, and after the transition nothing is left to race.
- **Cost.** At the validator client's default page size (100), about 10 queries per epoch at ~1000 nodes: about 260 once at startup, one `LastSubmittedMeasurement` read per poll, about 10 more per submission chunk that lands (about five per epoch), about 10 more at each epoch boundary to re-page the previous epoch, and one weights read per epoch. Rejected: pinned paging, which only matters for the step after the transition, where X is already frozen.
- **Shape.** The state is a `Clone` newtype over `Arc<RwLock<..>>`, shared by the refresher, the epoch operations and the gap route. A poll step holds the write lock throughout, so the refresher's and the epoch operations' steps never interleave, and readers wait for a step in progress (in shadow mode the gap route waits out the startup load instead of answering 503). The window records the time of its last successful step as its `refreshed_at`, whichever caller ran it. Resolution is a pure method on the data: for node n at epoch X it runs the shared rule (Decision 2) over the held epochs' medians and scores the resolved medians under the weights at X, returning the source epoch, its medians and the score, or nothing when there is no source. Loading is written against the validator client's `PerformanceQueryClient`, so tests drive it through a fake implementing that trait. The trait's blanket impl over `CosmWasmClient` does not conflict with an impl on a test type.

The window has no age limit and no outage handling of its own: the rule's R anchoring is the outage handling.

### 4. Startup per mode

- Read flag off: no performance data is read from the contract, and the switch cannot be on.
- Read flag on, switch off: a missing address fails startup. A window that cannot be loaded is logged, and the refresher keeps retrying at its polling interval. Until it loads, the gap route answers 503.
- Switch on, whether or not this nym-api rewards: the window must load before the epoch operations start, and startup fails otherwise. A contract that has never received a submission (`LastSubmittedMeasurement` with no data) fails startup too. A contract-mode nym-api has no other figure: an empty window would leave it nothing to reward, select or serve by (with `nym-api-performance-attestation` every nym-api serves performance), and an empty contract would pay every node zero and, since a node without a figure is skipped, leave the active set empty. It cannot become empty again at runtime, because bundles are never removed and R holds the last data.

### 5. Rewarding in contract mode reads the sealed epoch

`perform_epoch_operations` computes the rewards after the epoch's transition has begun, whether this run sent `BeginEpochTransition` or found it begun by itself after a restart. Before that, in contract mode, it runs the window's poll step at X, now sealed, so every submission that landed before the transition is in the window. The step reads `LastSubmittedMeasurement` and, only when it differs from the one the window recorded, re-pages every epoch from the recorded submission's epoch to X (Decision 3): normally X alone, or X - 1 and X when X's first chunk landed after the window's last poll. Submissions land in the epoch's first minutes, about an hour before its transition, so the preparation is usually that one query, and a re-page of X alone (about 10 queries) when a late chunk landed. Older epochs and the weights are never re-read. Alternative considered: re-paging X unconditionally, about 10 queries per epoch operation to learn what one query already tells. Each rewarded node is then paid the window's score for X as its `Performance`, directly: never through a routing component and never multiplied by nym-api's config score, since the contract score already includes the config median. A node without a score, whether it has no source or its source cannot be scored, is paid zero.

A failed read fails the epoch operation, which `run` retries after its 30 seconds. The epoch stays marked as being advanced by this nym-api, which the restart branch already handles. Nothing is ever paid zero because of a read failure. Today a missing annotation pays zero, and in contract mode a read failure would have looked exactly like that.

Legacy mode keeps reading annotations, at the same later point, so both modes follow one order. Reading annotations after the transition changes nothing there.

The epoch operations take their figure from a source fixed at construction: annotations in legacy mode, the window in contract mode. They hold no window when the switch is off, even though the window runs for the gap route.

### 6. Active-set selection in contract mode, with a fault mode

Selection weights every candidate by the same figure as rewarding, the window's score at the sealed X, in the existing `saturation x performance^20`. Candidates are chosen as today: bonded, described, and declaring at least one role. A candidate without a score is skipped. Selection keeps requiring nym-api's description because that is where it gets the roles a node can be assigned, and nym-api has no other source for them (the mixnet bond carries none). The rule that the contract path adds no describe-based filter concerns published performance (`nym-api-performance-attestation`), not selection. As a side effect, a node nym-api can no longer describe drops out of selection within the describe cache's 75-minute rebuild, until the orchestrator's reachability input lands. Taking roles from the directory contract instead is later work.

Fault mode: when nothing was submitted at X (R < X), every candidate with a score above 0 gets factor 1, so selection among them is by stake alone. Any constant reduces `saturation x performance^20` to proportional-to-stake sampling. A candidate whose frozen score is exactly 0 keeps it: a node that was offline when last measured scores exactly 0 (failed runs score 0, and so does their weighted mean), and giving it factor 1 would bring every node last measured as dead back into the stake-weighted draw for the whole outage. It stays available to the uniform filler, as in normal operation. One warning line is logged; there is no metric. Alternatives considered: factor 1 for every scored candidate, zeros included, which is pure stake but reselects dead nodes; and selecting on the scores frozen at R, the ones rewarding pays meanwhile.

A single test kind down for more than 24 epochs is not a fault. The orchestrator's guard omits it, R keeps advancing on the other kinds, and nodes measured only by the missing kind lose their figure and are skipped. Their role then falls to the sampler's uniform filler (`support/sampling.rs`), which fills a role from zero-weight candidates when positive-weight ones run short. This is accepted, and the orchestrator's omitted-test-kind gauge reports it from its first epoch.

### 7. A node's annotation is carried over only when the gap is ours

When the refresher computes no performance for a node (its applied set is empty), it carries the node's previous figures over only when the gap is our fault: this refresh's coverage guard dropped a component that the node is eligible for by nym-api's current description. Concretely, stress was enabled but dropped and the node is stress-eligible, or liveness was enabled but dropped and the node is liveness-eligible. A carried node keeps its previous `detailed_performance`, with its current role and chain capabilities refreshed. Every other node without a performance is left out of the new map and served 0. That covers a node nym-api cannot describe, which is never eligible for anything and whose config score is `ConfigScoreV2::unavailable()` (zero) anyway, and a node no longer bonded.

The refresher builds the complete new map from the previous one, and the cache keeps replacing the whole map on update, so `NodeStatusCache::update` is unchanged. The misleading comment goes with the old behaviour.

No age limit and no knob. A carried node still answers describe, and nym-api rebuilds its describe cache from scratch every 75 minutes, so a node that stops answering drops out within that and is no longer carried. A cap would instead empty every annotation at once during an orchestrator outage longer than the cap (the contract's R anchoring has no cap for the same reason). The rule changed from an earlier "keep the last annotation for up to 24 h" because a node that has shut down must not be served its last, perhaps perfect, figure. No timestamp is recorded in this change, since nothing would read it, and `computed_at` arrives with its wire field in `nym-api-performance-attestation`. It is a legacy-mode rule and retires with legacy scoring.

### 8. Contract mode computes no figure of nym-api's own, and writes no annotations in this change

nym-api's own scoring (the provider reads, the coverage guards, the config score, the carry-over of Decision 7) runs in legacy mode only. In contract mode the node status refresher skips all of it and writes an empty annotation map on every refresh. It skips the chain-capability refresh (each node's balance and feegrant) too: that only feeds nym-api's own config score, and the contract's config median already covers chain standing, the orchestrator scoring it with the same `nym-config-score`. It writes rather than skipping the update, because the annotation cache is persisted: after a flip, a refresher that wrote nothing would leave the legacy annotations loaded from disk in place, served indefinitely. Published performance in contract mode therefore reads 0 until `nym-api-performance-attestation` builds annotations from its verified snapshots, replacing this branch. The switch stays off until then, so that state is never live.

The old contract mode is not preserved anywhere: it never worked (Context), so the window, rewarding and selection are new code, and nothing in contract mode goes through the provider abstraction any more. `NodePerformanceProvider` is left with a single implementation, so the trait goes. The refresher holds `LegacyStoragePerformanceProvider` directly, and the `empty()` constructors only the contract provider used go too.

### 9. Configuration

- New: `[performance_provider].read_performance_contract` (Decision 1), in the template with a comment that the switch requires it.
- Changed: the scoring section is no longer rejected alongside the switch (Decision 1), and the template's scoring comment says it applies only while the switch is off, replacing its line "none may be enabled while use_performance_contract_data is set".
- Removed, silently: `[performance_provider.debug].max_performance_fallback_epochs` and `max_epoch_entries_to_retain`. Neither did anything an operator relied on (the fallback loop never ran). nym-api denies no unknown fields, so a config that still sets them loads, and neither is in the template, so no fixup or deprecation warning is needed.
- `contract_polling_interval` stays as the window's poll, its doc saying it applies when the read flag is on.

### 10. The temporary `contract-divergence` route

`GET /v3/unstable/nym-nodes/contract-divergence`, next to `liveness-divergence` and, like it, temporary: it exists for the shadow period, to decide the flip and to find the operators to notify, and is deleted afterwards. It compares, per bonded node, N, nym-api's annotation `performance_score`, with C, the window's resolution at the current epoch X.

Each bonded node lands in exactly one category, the first that matches:

1. **missing**: N > 0, and C has no source;
2. **contract-only**: no annotation, and C has a score;
3. **unscoreable**: C has a source but no score;
4. **fallback**: C's source epoch is below X;
5. **divergent**: |N - C| is at least the threshold, a fixed 0.2 (a constant, not a request parameter: the route is a temporary gauge, and rows carry both figures, so a reader wanting another cut can apply it to them);
6. **agree**: everything else.

A node with no annotation or N = 0, and no source in C, is not listed: neither side has anything for it. A node both sides score 0 reads as agree. Fallback comes before divergent so that a stale contract figure is never reported as a measurement disagreement; the row still carries both figures, so its divergence stays visible. Early in each epoch, before the orchestrator's submission for X has landed, most nodes read as fallback, which is the honest answer.

The response carries X, the threshold, `refreshed_at` (the oldest of the status cache, the describe cache and the window), and per category a count in total and per declared role (mixnode, entry, exit, from nym-api's describe cache; a node counts under each role it declares, and a node nym-api has not described counts in the total only). It has one row per node in every category except agree, carrying:

- the category;
- nym-api's side when it has an annotation: N, the routing score and the config score;
- the contract's side when C has a source: the source epoch, the score, and the liveness, stress and config medians;
- the node's role in the current rewarded set, if any, so that "rewarded-set nodes whose figure would drop" can be read straight off the rows;
- on missing rows only, the node's version and declared roles, which is what identifies the known groups the orchestrator cannot describe (pre-2026.12 nodes, invalid chain addresses).

It is served only when nym-api has both sides: the read flag on (the contract side) and the switch off (nym-api's own figure, which contract mode does not compute, Decision 8). Otherwise it stays mounted and answers 503 (`AxumErrorResponse::service_unavailable`) naming the missing side. It also answers 503 until the window has loaded. A 404 would read as a wrong URL, and with the route always mounted the OpenAPI document is the same in every mode.

As with `liveness-divergence`, the categorisation is a pure `build_response` over the bonded nodes, annotations, descriptions, rewarded set and the window's resolutions, unit-tested without a server. The response types live in `nym-api-requests` (`models/network_monitor.rs`, beside the liveness ones), with no TS derive.

### 11. No outage guard in nym-api

Broken measurements are handled where they are made: the orchestrator's coverage guard per test kind turns a fleet-wide probe fault into missing data. Missing data (nothing submitted, a partial epoch, skipped nodes) is handled by the contract's R-anchored fallback, which freezes values instead of zeroing them. In contract mode nym-api has no other figure to fall back to, so a guard there could only freeze values, which the fallback already does.

Accepted gap: the orchestrator's guard divides by the nodes it has described, and nym-api's legacy guard by the nodes nym-api has described, so a node the orchestrator never described cannot trip it. The gap is narrow. Descriptions persist until a node unbonds, so a regression in describing does not shrink the population the guard divides by. Never-described nodes are unscored by design. After the flip that group is visible only through the orchestrator's `info`-level describe-failure logs, and before it through the gap route's missing category.

### 12. Testing

Offline only, with deterministic fixtures.

- **The shared rule** (common crate), over a map-backed lookup: a hit at X; the newest earlier bundle within the lookback; nothing beyond `R - 24`; a global outage freezing at the last bundle; an audit across a long outage; X = 0; and the hint's equivalence over an exhaustive small space (every bundle layout of one node over a few epochs, other nodes' data fixing R, with and without the pointer the contract would hold). The lookups are called lazily: a hit at X never asks for R or the hint.
- **The contract**: its existing `RewardingInputs`, `RewardingScore`, unbond and digest tests pass unchanged, and the wasm check passes.
- **The window**, through a test type implementing `PerformanceQueryClient` (answering `EpochPerformancePaged`, `WeightsAt` and `LastSubmittedMeasurement` from its fields, through a JSON round trip): the startup load after an outage holds `[R - 24, R]` and pages nothing above R; a last submission whose epoch is empty walks down to R; a start with X behind the newest submission holds that epoch once X reaches it; a load finding nothing in the 25 epochs from its start fails; a contract without a submission yields an empty window, and its first submission loads it afresh; a new submission re-pages from the recorded submission's epoch to its own; an unchanged last submission reads nothing else; a submission landing between the last-submission read and the pages leaves a change for the next step; a new epoch reads its weights, pages nothing and drops nothing while R stays; a late chunk for X and the first for X + 1 between two polls are both held; a failed step leaves the window as it was; a poll naming an older epoch keeps the current one; resolution at X equals what the contract's own `RewardingInputs` gives for the same bundles (hit, fallback, beyond the lookback, weights of the requested epoch, unscoreable).
- **Rewarding and selection**, as pure functions over a window and the candidate set: the contract score is the paid performance with no config factor; a node without a score is paid zero; selection skips a candidate without a score; fault mode gives every candidate scoring above 0 factor 1, keeps a zero at 0, and still skips a candidate without a score.
- **Carry-over**, as the pure decision of whether this refresh's dropped components affect a node: a dropped component affects the nodes eligible for it and no other; nothing dropped affects no node, a described node declaring no role included; a node eligible for nothing (undescribed, or declaring no role) is never affected, even with every component dropped.
- **Config**: the switch without the read flag is rejected; the switch with the default scoring section (`legacy_v1_routing` enabled) is accepted; the scoring section's own rules still apply with the switch on; the template renders the flag; a config still carrying the removed knobs loads.
- **The gap route**, through its pure categorisation and `build_response`: each category; fallback taking precedence over divergent; both sides zero is agree; the unlisted case; a difference below the threshold agreeing and one above it diverging; role counts with a dual-role and an undescribed node; a rewarded-set node's role on its row; version and roles on missing rows only.

Not unit-tested, each a few lines over tested pieces:

- the order of the epoch operations, which needs a chain: the scenarios "Rewards follow the sealed epoch", "A failed read pays no one" and "Legacy mode reads annotations after the transition";
- the startup wiring per mode: "With the flag off no performance data is read", "A missing address fails startup", "Reading is best effort while the switch is off" and "A contract that never received a submission stops a contract-mode start" (the empty window itself is tested);
- the refresher's polling loop;
- the gap route's handler: its three 503 checks sit inline, each beside its message ("The route names the side it lacks"), and the read flag off with the switch on cannot pass validation, so no two of them overlap;
- the node status refresher around the carry-over decision, which needs a nyxd client, the storage-backed provider and the caches: contract mode's empty write ("Contract mode writes no annotations"), a carried annotation taking the current role and chain capabilities, and a node no longer bonded being left out (the loop only visits bonded nodes);
- the `network-monitor` requirement, a description of the consumers that the tests above cover.

## Risks / Trade-offs

- [A query after `BeginEpochTransition` served by an RPC node that has not yet seen the last blocks] -> It could miss a submission that landed just before the transition. Submissions land within minutes of an epoch's start, about an hour before its transition, so only a retried chunk landing in the last seconds is exposed. Accepted.
- [Pages of the current epoch read across a submission] -> Unpinned pages can miss a node written between two of them. The next poll sees the last submission change and re-pages, and the poll before rewarding runs once the epoch is frozen.
- [A single test kind down for more than 24 epochs] -> Its nodes lose their figure and their role falls to the uniform filler (Decision 6). The orchestrator's omitted-test-kind gauge reports it from the first epoch.
- [The window cannot load at startup with the switch on] -> nym-api does not start, which is the intent: rewarding without the contract would pay zero.
- [A carried annotation outlives a node's death] -> Only while the node still answers describe, at most about 75 minutes after it stops (Decision 7).
- [The startup burst] -> About 260 queries once. The per-epoch steady state is about 50.
- [The gap route recomputes per request] -> About 1000 nodes against in-memory caches: cheap, and the route is temporary.

## Migration Plan

No schema migration, no wire change, and no contract migration (the contract's change is a code move behind identical behaviour; it ships with the not-yet-deployed contract).

Deployment: with the read flag off nothing changes, and every existing nym-api config keeps loading. For the shadow period, set `read_performance_contract = true` with the performance contract's address in the network details (env file or a `CONFIGURED` environment, as for the orchestrator) and leave the switch off. The gap route is then live, and rewarding and selection still use nym-api's own figure. The flip (`use_performance_contract_data = true`, the scoring section left as it is) waits for `nym-api-performance-attestation` and the orchestrator's aliveness follow-up.

Rollback: set the switch off (back to nym-api's own figure under the unchanged scoring section), or the read flag off too (no contract reads at all), and restart.

## Open Questions

None for this change.
