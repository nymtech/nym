## 1. Shared types and messages (`common/cosmwasm-smart-contracts/nym-performance-contract`)

- [x] 1.1 Add `Measurements { liveness, stress, config: Option<Percent> }` (renamed `l`/`s`/`c`, `default`, absent omitted) and `KindMedians` (same fields, full names) in `types.rs`; unit-test the JSON shape, that absent kinds are omitted, and the round trip through the on-chain codec
- [x] 1.2 Add `NodeSubmission { node_id ("n"), measurements ("m"): Measurements }` and replace the old single-value `NodePerformance` submission payload in `ExecuteMsg::Submit` / `BatchSubmit`, `LastSubmittedData.data` and `InstantiateMsg` (add `initial_weights: Weights`)
- [x] 1.3 Change `NodeResults` to store `Vec<u8>` integer percents serialised as JSON integers, converting with `round_to_two_decimal_places().round_to_integer()` on insert and `Percent::from_percentage_value` on read while keeping `Percent` in its API (a lazy `values()` iterator); add `EpochNodeMeasurements { liveness, stress, config: Option<NodeResults> }` (renamed `l`/`s`/`c`, absent omitted) with `new` / `insert` taking `Measurements` by value (it is `Copy`) and merging field by field through `NodeResults::add_to`, and `medians() -> KindMedians`; unit-test the `[93,95,97]` JSON form and the `Percent` round-trip including `0` and `100`, the two-monitor merge, half-up rounding on insert, and the existing median cases
- [x] 1.4 Add `Weights { liveness: Percent, stress: Percent }` with `#[serde(default)]` fields and `validate()` enforcing at least one non-zero weight and an exact `Decimal` sum of one (zero means the kind does not contribute); add `EpochWeights { effective_from, weights }`; unit-test the all-zero and wrong-sum rejections, the `0.7 + 0.3` acceptance, and that a missing field deserialises as zero
- [x] 1.5 Add `Weights::score(&self, medians: &KindMedians) -> Option<Percent>` in the common crate implementing the renormalised weighted mean times config in `Decimal` with two-decimal rounding; unit-test every scenario of the score requirement (single kind, two kinds, config gate, unweighted kind, no applied kind, missing config)
- [x] 1.6 Add the response types: `EpochNodePerformance { epoch_id, medians, score }`, `NodePerformance { node_id, medians, score }`, `HistoricalPerformance { epoch_id, node_id, medians, score }`, `NodeMeasurements { node_id, measurements }`, `ResolvedMedians { epoch_id, medians }`, `RewardingInputsResponse`, `RewardingScoreResponse { score: Option<Percent> }`, `WeightsResponse`, `LastKnownEpochResponse`; keep every existing paged response wrapper
- [x] 1.7 Extend `ExecuteMsg` with `UpdateWeights { weights }` and `QueryMsg` with `RewardingInputs`, `RewardingScore`, `LastKnownEpoch`, `WeightsAt`, `CurrentWeights`; update the `schema`-feature `returns` annotations and the `msg.rs` imports
- [x] 1.8 Add error variants `EpochNotCurrent`, `EmptyNodeSubmission`, `EmptyWeights`, `WeightsDoNotSumToOne { total: Decimal }` to `error.rs`
- [x] 1.9 Add storage-key constants `PERFORMANCE_RESULTS = "pr"`, `LAST_KNOWN_EPOCH = "last-known-epoch"`, `WEIGHTS = "weights"` and the constant `MAX_FALLBACK_LOOKBACK_EPOCHS: EpochId = 24` to `constants.rs`; `cargo check -p nym-performance-contract-common` from the root workspace

## 2. Contract storage (`contracts/performance/src/storage.rs`)

- [ ] 2.1 Replace `results` with `Map<(EpochId, NodeId), EpochNodeMeasurements>` under `pr`, add `last_known_epoch: Map<NodeId, EpochId>` and `weights: Map<EpochId, Weights>` to the storage struct, and extend `initialise` to validate and store `initial_weights` under the creation epoch
- [ ] 2.2 Add `ensure_current_epoch(deps, epoch)` returning `EpochNotCurrent`, and call it in both submit paths immediately after the authorisation check and before the cursor check; port the existing "past epochs" tests to expect `EpochNotCurrent` and add the future-epoch and frozen-bundle tests
- [ ] 2.3 Rewrite `insert_performance_data` to reject an empty map with `EmptyNodeSubmission`, merge the submission into the bundle, and write the pointer as a max only when the bundle was created; test the pointer under a first and a second monitor, and that an empty map leaves the cursor unchanged
- [ ] 2.4 Update `submit_performance_data` and `batch_submit_performance_results` to the `NodeSubmission` payload, keeping the cursor, bonded-check, batch ordering and last-submission semantics; port the existing cursor, ordering, authorisation and bonded tests and add the empty-batch no-op test
- [ ] 2.5 Add `update_weights(deps, sender, weights)` (admin check, validate, store under current epoch + 1) and `weights_at(storage, epoch) -> Option<EpochWeights>` (descending bounded range, take one); test next-epoch effect, same-epoch overwrite, non-admin rejection and pre-creation `None`
- [ ] 2.6 Add `resolve_rewarding_inputs(deps, epoch, node) -> RewardingInputsResponse` implementing the per-bundle fallback (direct hit, `min(pointer, X-1)` walk down to `X.saturating_sub(L)`, `X == 0` guard) and scoring with `weights_at(X)`; test every scenario of the RewardingInputs requirement including the role-switch case, the weights-of-requested-epoch case, the dangling pointer, and repeated-call equality
- [ ] 2.7 Update `try_load_performance` to return `EpochNodePerformance` (medians plus score under that epoch's weights) and keep `remove_node_measurements` / `remove_epoch_measurements` on the plain map, asserting in tests that neither touches the pointer or the weights

## 3. Handlers, queries and entry points (`contracts/performance/src/{transactions,queries,contract}.rs`)

- [ ] 3.1 Add `try_update_weights` emitting the `weights_update` event with `effective_from` and `weights`, and dispatch `ExecuteMsg::UpdateWeights` in `execute`
- [ ] 3.2 Add `query_rewarding_inputs`, `query_rewarding_score` (a projection of the same resolution, never a second code path), `query_last_known_epoch`, `query_weights_at`, `query_current_weights` and dispatch them in `query`; test that `query_rewarding_score` equals `query_rewarding_inputs(..).score` in a direct-hit, a fallback and a no-data case
- [ ] 3.3 Rewrite `query_node_performance_paged` to end at the node's pointer, skip epochs without a bundle, and set `start_next_after` only below the pointer; port the existing paged test to the gap-omitting semantics and add the start-past-pointer case
- [ ] 3.4 Update `query_epoch_performance_paged` and `query_full_historical_performance_paged` to return medians plus score, resolving weights once per page for the epoch page and per distinct epoch for the full-history page; test the epoch page under `Liveness: 100%` and the key-order full-history case
- [ ] 3.5 Update `query_node_measurements` and `query_epoch_measurements_paged` to the `EpochNodeMeasurements` / `NodeMeasurements` shapes
- [ ] 3.6 Add the instantiation test for invalid initial weights persisting nothing, alongside the existing admin-is-sender test

## 4. Test harness (`contracts/performance/src/testing/mod.rs`)

- [ ] 4.1 Extend `base_init_msg` and `init` with a valid default `initial_weights` (`Liveness: 100%`), and add `PerformanceContractTesterExt` helpers to build a `NodeSubmission` from `(node_id, &[(MeasurementKind, &str)])` and to submit it for the current mixnet epoch
- [ ] 4.2 Replace `insert_raw_performance` / `insert_epoch_performance` / `dummy_node_performance` with `NodeSubmission`-based equivalents and fix every call site in the storage, transaction and query tests
- [ ] 4.3 Run `cargo test -p nym-performance-contract` from the `contracts` workspace and get it green

## 5. Validator client (`common/client-libs/validator-client/src/nyxd/contract_traits`)

- [ ] 5.1 In `performance_signing_client.rs` change `submit_performance` / `batch_submit_performance` to `NodeSubmission` and add `update_weights`; extend `all_execute_variants_are_covered`
- [ ] 5.2 In `performance_query_client.rs` add `get_rewarding_inputs`, `get_rewarding_score`, `get_last_known_epoch`, `get_weights_at`, `get_current_weights`, re-export the new types, keep the paged collectors, and extend `all_query_variants_are_covered`
- [ ] 5.3 `cargo check -p nym-validator-client --tests` from the root workspace

## 6. Root-workspace consumers

- [ ] 6.1 Minimal nym-api edit: in `nym-api/src/node_performance/contract_cache/data.rs` map each `NodePerformance` to its `score`, skipping nodes with `None`, so `PerformanceContractEpochCacheData` keeps its `HashMap<NodeId, Performance>` shape; `cargo check -p nym-api`
- [ ] 6.2 In `tools/internal/localnet-orchestrator/src/orchestrator/setup/cosmwasm_contracts.rs` supply `initial_weights` (`Liveness: 100%`) to the performance contract `InstantiateMsg`; `cargo check -p localnet-orchestrator`

## 7. Schema and build wiring

- [ ] 7.1 Add a `performance-schema` target to `contracts/Makefile` (`$(MAKE) -C performance generate-schema`, which `contracts/performance/Makefile` already provides), list it in the `schema` aggregate it is currently missing from, and regenerate `contracts/performance/schema/` via `make contract-schema`
- [x] 7.2 Confirm `common/cosmwasm-smart-contracts/nym-performance-contract` builds with `--features schema` from the root workspace

## 8. Final verification and hand-off

- [ ] 8.1 `cargo fmt` across both workspaces; `cargo test -p nym-performance-contract` in `contracts`; `cargo check -p nym-performance-contract-common -p nym-validator-client -p nym-api -p localnet-orchestrator` in the root; no clippy pass required
- [ ] 8.2 Read the full `git diff` against `develop` once, checking every spec scenario has a test, storage namespaces match the public layout requirement, and no `Co-Authored-By` or commit was created; leave the work uncommitted for review
- [ ] 8.3 Close PR #6282 with a pointer to the new PR and credit for the multi-kind direction (user action), and draft the PR description in a fenced markdown block
