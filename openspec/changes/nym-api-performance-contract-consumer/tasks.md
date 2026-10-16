Paths are under `nym-api/src` unless stated otherwise. A task group compiles at its end; tasks inside a group need not.

## 0. Skeleton

- [ ] 0.1 Thread the change through before filling it in, with `todo!()` at the pure leaves:
  - the shared rule's signature in `common/cosmwasm-smart-contracts/nym-performance-contract/src/resolution.rs` (design Decision 2);
  - the window in `node_performance/contract_window/`: the `Clone` state newtype, the load and poll functions generic over `PerformanceQueryClient`, the resolution method, the refresher task (Decision 3);
  - the epoch operations' figure source, fixed at construction (Decision 5);
  - the `contract-divergence` handler and `build_response` signature (Decision 10);
  - the config field `read_performance_contract` (task 2.1).

  Delete `node_performance/contract_cache/`, `node_performance/provider/contract_provider.rs` and the `NodePerformanceProvider` trait, with its contract impl and the test of it; the refresher holds `LegacyStoragePerformanceProvider` directly (Decision 8). `cargo check -p nym-api --all-targets` passes

## 1. The shared resolution rule

- [ ] 1.1 In `nym-performance-contract-common`, `resolution.rs`: the pure function over `bundle_at(epoch)`, `newest_measured(at_or_before)` and `last_known()`, generic over the bundle and error types and returning `Result<Option<B>, E>`. Calls are lazy and in the contract's order: the bundle at X; then, only on a miss and for X > 0, R; then the hint; then the walk over `[R - MAX_FALLBACK_LOOKBACK_EPOCHS, top]` newest first, where `top` is the hint when it is at or below X, else `min(X - 1, R)`. Doc in one short line plus the reason the hint never changes the result. It must build on the contracts' MSRV (1.86)
- [ ] 1.2 Tests over a map-backed lookup: a hit at X; the newest earlier bundle within the lookback; nothing below `R - 24`; a global outage freezing at the last bundle; an audit across a long outage; X = 0; a hit at X never calls `newest_measured` or `last_known`. Plus the hint's equivalence: over every layout of one node's bundles across a few epochs, with other nodes' bundles fixing R, the result with the contract's pointer (the node's newest bundle, or none) equals the result without one
- [ ] 1.3 `contracts/performance/src/storage.rs`: `resolve_source_bundle` calls the shared function with closures over `may_load_bundle`, `newest_measured_epoch` and the pointer map. `newest_bundle_between` and the window arithmetic go
- [ ] 1.4 `cargo test -p nym-performance-contract-common`, `cd contracts && cargo test -p nym-performance-contract` (the existing `RewardingInputs`, `RewardingScore`, unbond and digest tests pass unchanged), `make docker-check-contracts`

## 2. Configuration (`support/config`)

- [ ] 2.1 `PerformanceProvider` gains `read_performance_contract: bool` (default `false`, a one-line doc). `PerformanceProvider::validate` rejects `use_performance_contract_data` without it, with an error naming the flag. `PerformanceProviderScoring::validate` loses its `use_performance_contract_data` parameter and the branch rejecting enabled properties, so the section is checked by its own rules in both modes
- [ ] 2.2 Remove `max_performance_fallback_epochs` and `max_epoch_entries_to_retain` from `PerformanceProviderDebug`, with their default constants. `contract_polling_interval`'s doc says it is the window's poll, applying when `read_performance_contract` is on
- [ ] 2.3 `template.rs`: the flag with a comment saying the switch requires it; the scoring comment says the section applies only while `use_performance_contract_data` is off, replacing "none may be enabled while use_performance_contract_data is set"; the switch's comment says it decides only rewarding and selection
- [ ] 2.4 Tests: the switch without the flag is rejected; the switch with the default scoring section is accepted; the scoring section's own rules (sum to one, routing or liveness enabled) still reject with the switch on; a config setting both removed knobs loads; the template round-trips the flag (beside the existing template test)
- [ ] 2.5 `cargo test -p nym-api` scoped to `support::config`

## 3. The window (`node_performance/contract_window/`)

- [ ] 3.1 The data: per epoch `HashMap<NodeId, KindMedians>` (medians only, an epoch read with no nodes not held), the weights at X with X, and the last submission seen. Methods: R at or before an epoch; trimming below `R - 24` except X; the figure of node n at X, which runs the shared rule (task 1.1) with a hint of `None` and `Infallible` errors and scores the source's medians with `Weights::score` under the weights at X, returning a named struct (source epoch, medians, `Option<Percent>` score) or `None` without a source; whether X holds data (for fault mode)
- [ ] 3.2 Loading, generic over `C: PerformanceQueryClient`: the startup load from `get_last_submission` (from its epoch down to the first epoch holding nodes, then to `R - 24`, then X and the weights at X), and one poll step (a new X: page it and read its weights; a changed last submission: re-page its epoch; then trim). An epoch is paged with `get_all_epoch_performance`. A `reload(X)` for the epoch operations re-pages X, and reads its weights if X is new
- [ ] 3.3 The refresher task: waits for the mixnet contract cache, then polls every `contract_polling_interval` under the shutdown token, taking X from the mixnet contract cache's current interval and logging a failed step without stopping. It records the time of its last successful step for `refreshed_at`
- [ ] 3.4 `support/cli/run.rs`: with the flag on, a missing performance contract address fails startup; the window is built over the query client and started. With the switch off a failed first load is logged and left to the refresher. With the switch on the first load must succeed, and a last submission with no data fails startup. `has_performance_data` is unchanged. Drop any `support/nyxd` wrapper left without a caller (`get_full_epoch_performance`, `get_last_performance_contract_submission`)
- [ ] 3.5 Tests through a test type implementing `PerformanceQueryClient`, answering `EpochPerformancePaged`, `WeightsAt` and `LastSubmittedMeasurement` from its fields through a `cosmwasm_std` JSON round trip (the PR3 `FakeContract` pattern): a start after an outage holds exactly `[R - 24, R]`, with X held as absent and the weights at X; a last submission whose epoch is empty walks down to R; a changed last submission re-pages X, which becomes R and trims; a new epoch adds X + 1 with its weights and drops nothing while R stays; a contract with no submission yields an empty window. A figure test over the bundle layouts and weights of the contract's own `RewardingInputs` tests asserts the source epochs, medians and scores those tests expect, for a hit, a fallback, a node beyond the lookback and an unscoreable source
- [ ] 3.6 `cargo test -p nym-api` scoped to `node_performance`

## 4. The node status refresher (`node_status_api/cache`)

- [ ] 4.1 With the switch on, `refresh` skips the provider reads, the chain-capability refresh and scoring, and writes an empty annotation map (Decision 8); `NodeStatusCacheConfig` carries the switch
- [ ] 4.2 With the switch off, `produce_node_annotations` takes the previous map and, for a node whose performance is `None`, carries its previous `detailed_performance` with the current role and chain capabilities only when stress was enabled but dropped by `component_applies` and the node is `stress_test_eligible`, or the same for liveness with `liveness_eligible` (Decision 7). Every other node without a performance is left out. The misleading comment goes. `NodeStatusCache::update` stays a whole-map replace
- [ ] 4.3 Tests: a node whose eligible component was dropped keeps its previous figures with a refreshed role; a node whose applied set is empty with nothing dropped (a described node declaring no role, liveness the only property) is left out; an undescribed node is left out even when a component was dropped; a node absent from the bonded list is left out; with the switch on a refresh over a non-empty previous map writes an empty one
- [ ] 4.4 `cargo test -p nym-api` scoped to `node_status_api`

## 5. Epoch operations (`epoch_operations`)

- [ ] 5.1 `EpochAdvancer` takes its figure source at construction: annotations, or the window when the switch is on. `perform_epoch_operations` computes the rewards only after the transition step (sent by this run or found begun by us), and in contract mode `reload`s X first, a failure returning the error so `run` retries after 30 s
- [ ] 5.2 Contract-mode rewards, as a pure function over the window's figures and the rewarded set: each node's score at X as its `Performance`, zero without a score. The legacy path keeps `load_performance` over the annotations
- [ ] 5.3 Contract-mode selection in `attach_performance_to_eligible_nodes`: candidates as today, the performance from the window's score at X, skipped without one. When X holds no data, every candidate scoring above 0 gets performance 1 and a zero stays 0, with one `warn!`. The fault-mode mapping is a pure function
- [ ] 5.4 Tests: the contract score is the paid performance with no config factor; a node without a source and one with an unscoreable source are paid zero; selection skips a candidate without a score; fault mode gives every candidate scoring above 0 factor 1, keeps a zero at 0, and still skips a candidate without a score; with X holding data, scores pass through unchanged
- [ ] 5.5 `cargo test -p nym-api` scoped to `epoch_operations`

## 6. The `contract-divergence` route

- [ ] 6.1 `nym-api/nym-api-requests/src/models/network_monitor.rs`, beside the liveness divergence types and with no TS derive: the category enum (snake_case), per-category counts (total, mixnode, entry, exit), the row (category, nym-api side, contract side, rewarded-set role, described details on missing rows) and the response (X, threshold, `refreshed_at`, counts, rows), with one-line docs and a TEMPORARY note on the response
- [ ] 6.2 `unstable_routes/v3/nym_nodes/contract_divergence/mod.rs`: a pure `build_response` over the bonded nodes, annotations, descriptions, rewarded set and the window's figures at X, applying the first-match categories of Decision 10; a pure availability check (flag off, switch on, window not loaded, each with its 503 message); the handler with the `threshold` query parameter (default 0.5, 400 outside `[0, 1]`) and `refreshed_at` as the oldest of the three caches; utoipa docs. Mount it beside `liveness-divergence` with the same TEMPORARY comment
- [ ] 6.3 `AppState` (`support/http/state/mod.rs`) gains the window (`Option`, present when the flag is on) and the switch, set in `support/cli/run.rs`
- [ ] 6.4 Tests on `build_response`: each category; fallback taking precedence over divergent; both sides zero is agree with no row; a node with neither side is not listed; a dual-role divergent node and an undescribed contract-only node in the counts; a rewarded-set node's role on its row; version and roles on missing rows only. On the availability check: each 503 case and the served case. On the threshold: 1.5 is rejected
- [ ] 6.5 `cargo test -p nym-api` scoped to `unstable_routes`, `cargo check -p nym-api-requests`

## 7. Final verification and hand-off

- [ ] 7.1 `cargo fmt`; then `cargo test -p nym-performance-contract-common`, `cd contracts && cargo test -p nym-performance-contract`, `cargo test -p nym-api` (scoped to the touched modules if the whole suite needs a database), `cargo check -p nym-api --all-targets`, `cargo check -p nym-api-requests`, and `make docker-check-contracts`
- [ ] 7.2 Read the full `git diff` once:
  - every scenario in the delta specs has a test, except those design Decision 12 leaves untested: the epoch operations' order ("Rewards follow the sealed epoch", "A failed read pays no one", "Legacy mode reads annotations after the transition"), the startup wiring ("With the flag off nothing reads the contract", "A missing address fails startup", "Reading is best effort while the switch is off", "A contract that never received a submission stops a contract-mode start"), and the `network-monitor` wording;
  - no stub or panic is left in non-test code;
  - nothing is committed, and no `Co-Authored-By` line is added
- [ ] 7.3 Archive the change in the same PR. `openspec archive` aborts on the `nym-network-monitor` MODIFIED requirement, which drops the scenario "A component with no data available does not freeze the annotations" on purpose. So sync the two main specs by hand (replace the two MODIFIED requirements whole, append the six ADDED ones to `nym-network-monitor`), then move the change with `openspec archive nym-api-performance-contract-consumer --skip-specs --yes`. Run `git diff openspec/specs`, account for every line, and `openspec validate --strict` both specs
- [ ] 7.4 Draft the PR description in a fenced markdown block
