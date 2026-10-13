## 1. Shared types and messages (`common/cosmwasm-smart-contracts/nym-performance-contract`)

- [x] 1.1 In `constants.rs` add `storage_keys::DIGEST_STATE = "digest_state"`, with the sibling contracts' doc on why it is load-bearing (used verbatim through `store.set`, holds the accumulator rather than its collapse, never changes)
- [x] 1.2 In `types.rs` add `LatestPerformance { node_id, epoch_id, score: Option<Percent> }`, with no medians. Give it a constructor from `(node_id, epoch_id, &EpochNodeMeasurements, Option<&Weights>)` that scores the bundle's medians, and `digest_leaf() -> [u8; 26]` with the layout from the leaf requirement (tag `1`, big-endian `u32` node and epoch, then presence byte plus big-endian `u128` atomics of the score). Unit-test the documented layout vectors, that every field is committed, absent against present zero, and `0.85` against `0.8549`
- [x] 1.3 In `types.rs` add `LatestPerformancePagedResponse { performance: Vec<LatestPerformance>, start_next_after: Option<NodeId> }` and `DigestResponse { digest: Binary }`, and delete `RemoveEpochMeasurementsResponse`
- [x] 1.4 In `msg.rs` add `ExecuteMsg::OnNymNodeUnbond { node_id }`, `QueryMsg::LatestPerformancePaged { start_after, limit }` and `QueryMsg::Digest {}` with their schema `returns`, and remove `ExecuteMsg::RemoveNodeMeasurements` and `ExecuteMsg::RemoveEpochMeasurements`
- [x] 1.5 In `error.rs` add `UnauthorisedMixnetCallback { sender: Addr }` and `CorruptDigestState`
- [x] 1.6 Document on `Weights::score` which score changes would alter stored bundles' scores and so need a formula version in the weights or a re-fold, point `NodeResults::median` at it, and pin sample outputs from stored bundles in `score_outputs_are_frozen`
- [x] 1.7 From the root workspace: `cargo test -p nym-performance-contract-common` and `cargo check -p nym-performance-contract-common --features schema`

## 2. Shared digest storage and contract storage (`contracts/performance/src/storage.rs`)

- [x] 2.1 In `nym-contracts-common` add a `digest` module: `DigestResponse { digest: Binary }` and a `CorruptDigestState` error, plus `DigestItem` behind a new `lthash` feature. `DigestItem::new(key)` reads and writes the raw key with `store.get` / `store.set`; `load` returns `LtHash16::new()` for an absent key and `CorruptDigestState` for a wrong length, `save` writes the full accumulator, and `query` returns the `DigestResponse` collapse. Unit-test absent, round trip, wrong length and the collapse
- [x] 2.2 Move the directory and geolocation contracts onto it: a `digest: DigestItem` field on each storage struct replaces `load_digest` / `save_digest` and their comments, `query_digest` goes through `DigestItem::query`, their common crates re-export `DigestResponse` and implement `From<CorruptDigestState>` for their error enums while keeping the unit `CorruptDigestState` variants. Stored bytes, messages and error strings are unchanged. Update the `load_digest` mentions in `nym-contract-anchor`. In `contracts`: `cargo test -p nym-directory-contract -p nym-geolocation-contract`
- [x] 2.3 In the performance contract: depend on `nym-lthash` and `nym-contracts-common` with `lthash`, add a `digest: DigestItem` field, re-export `DigestResponse` from the common crate in place of 1.3's own definition, implement `From<CorruptDigestState>`, and shorten the `DIGEST_STATE` doc to point at `DigestItem`, which now holds the rules
- [x] 2.4 Delete `remove_node_measurements`, `remove_epoch_measurements`, `EPOCH_PERFORMANCE_PURGE_LIMIT` and their tests, and reword the comments that explain behaviour by admin removals (`resolve_source_bundle`'s "if that bundle is gone an admin removed it", the pointer's doc)
- [x] 2.5 Make `submit_performance_data` a batch of one: it calls `batch_submit_performance_results` and turns a node listed in `non_existent_nodes` into `NodeNotBonded`, so the batch path is the only one that loads and saves the accumulator. Adjust `requires_associated_node_to_be_bonded`, which relied on a failed submission leaving the cursor alone (deps-level tests do not revert)
- [x] 2.6 Thread one accumulator through the batch path, loaded once before the per-node loop and saved once after it. Split the write from the digest update: `PerformanceResultsStorage::insert_performance_data` writes the bundle and returns what it replaced (the bundle as stored on a merge, the bundle the pointer named on a create, via a `last_known_bundle` helper the unbond operation shares) along with the bundle written, and `NymPerformanceContractStorage::record_submission` subtracts the replaced entry and adds the written one. Score entries with the `effective_from` shortcut against `weights_at(X)`, loaded once per transaction, reading `weights_at(E)` only for `E < effective_from`, uncached
- [x] 2.7 Test the submission scenarios: a node's first bundle adds its entry, the next epoch replaces it, a second monitor in the same epoch updates it, an entry added under earlier weights is subtracted exactly, and one batch gives the same accumulator as one `Submit` per node
- [x] 2.8 Rework `resolve_source_bundle` for R. Find R with one descending `keys` step over `pr` bounded inclusively at `(X, u32::MAX)` (no R means `None`), keep the one-load branch for `Some(p) if p <= X` with floor `R - 24`, and send an absent pointer or `p > X` to a walk from `min(R, X - 1)` down to the floor. Update the `MAX_FALLBACK_LOOKBACK_EPOCHS` doc. Rewrite "data older than the lookback" with node 8 measured from 10 to 35, delete the removed-bundle test, and add the global-outage and audit-across-an-outage tests
- [x] 2.9 Add the unbond storage operation: load the pointer; with `Some(p)`, load the bundle at `(p, node)`, subtract its leaf under `weights_at(p)`, remove the pointer, and save the accumulator; with `None`, write nothing
- [x] 2.10 Split `resolve_source_bundle` for readability: the R lookup and the walk become `PerformanceResultsStorage` helpers, as `insert_performance_data` / `last_known_bundle` did for the submission path

## 3. Contract handlers and queries

- [x] 3.1 In `transactions.rs` add `try_handle_node_unbonding`: the sender must equal the stored mixnet address, else `UnauthorisedMixnetCallback { sender }`, and it returns an empty response. Delete `try_remove_node_measurements` and `try_remove_epoch_measurements`, and update the dispatch in `contract.rs`
- [x] 3.2 In `queries.rs` add `query_latest_performance_paged`, which ranges over `last_known_epoch` with an exclusive lower bound, takes new `retrieval_limits` of 100 by default and 200 at most, scores each entry through `EntryWeights` anchored at `EpochId::MAX` (the digest maintenance's own scoring, no per-page cache), and sets `start_next_after` to the last node returned or `None` for an empty page. Add `query_digest`, which returns `out()` of the loaded accumulator, and dispatch both
- [x] 3.3 Deps-level tests for the callback and queries: an unauthorised sender is rejected; a never-measured node writes nothing; the callback subtracts the entry and deletes the pointer; afterwards `LastKnownEpoch` is `None`, `NodePerformancePaged` is empty, `NodePerformance` still answers, and `RewardingInputs` / `RewardingScore` are unchanged at every epoch. For the bulk query: the paging example (nodes 1 to 5, node 2 called back, `limit: 2`), each entry's own epoch and weights, and the 200 cap. For the digest: a fresh contract has the empty digest, and the raw key holds 2048 bytes whose collapse equals `Digest {}`
- [x] 3.4 Add a harness helper to `testing/mod.rs` that pages `LatestPerformancePaged` to the end, folds every `digest_leaf`, and asserts the result equals the stored accumulator, unwrapping throughout. Add an operation-sequence test (several monitors, single and batch submissions, epoch advances, `UpdateWeights`, callbacks) that calls it after every step

## 4. Mixnet contract

- [x] 4.1 In `common/cosmwasm-smart-contracts/mixnet-contract`, add `ContractState.performance_contract_address: Option<Addr>`, documented like its siblings, a required `InstantiateMsg.performance_contract_address: String` beside the sibling addresses, and a required `MigrateMsg.performance_contract_address: String`
- [x] 4.2 In `contracts/mixnet`:
  - add the `nym-performance-contract-common` dependency;
  - validate the instantiate address and store it through `default_initial_state`, as the siblings are;
  - add `introduce_performance_contract` to `queued_migrations.rs`, called from `migrate()` inside `if !skip_state_updates` (reading the flag again, which #7220 stopped doing once the directory and geolocation migrations had run);
  - add a `performance_contract_address` getter in `mixnet_contract_settings/storage.rs`;
  - send `OnNymNodeUnbond { node_id }` from `try_remove_nym_node` as a fourth optional plain message
- [x] 4.3 Extend the fixtures: `base_init_msg` and `MixnetContractSiblings` in `testable_mixnet_contract.rs` (a performance field, `with_performance_contract`, `with_clear_all`, `set_mixnet_sibling_contracts`), the instantiate message and expected state in `contract.rs`, and `mixnet_contract_settings/queries.rs`. Test that instantiation stores the address, that `migrate` sets it, and that `unsafe_skip_state_updates: true` leaves it unchanged
- [x] 4.4 In `contracts`: `cargo test -p nym-mixnet-contract`, plus the directory, geolocation and node-families (package `node-families`) contract tests, whose harnesses call `with_clear_all`

## 5. App-level unbond tests (performance harness)

- [x] 5.1 Make `init_contract_tester` register the performance contract's own address with the mixnet contract through `with_clear_all().with_performance_contract(..)`, as the directory harness does
- [x] 5.2 Through a real `UnbondNymNode` executed on the mixnet contract (not the harness's `unbond_nymnode`, which advances the epoch), test that:
  - a real unbond removes the node from the digest;
  - an unbonded node cannot return (`Submit` fails with `NodeNotBonded`, `BatchSubmit` lists it in `non_existent_nodes`, the accumulator is unchanged);
  - a node's last epoch resolves after it began unbonding (bundle at 9, other nodes measured in 10, the unbond in 10 before any report, so `RewardingInputs { 10 }` returns source 9);
  - without the address no callback is sent

## 6. Validator client (`common/client-libs/validator-client/src/nyxd/contract_traits`)

- [x] 6.1 In `performance_signing_client.rs` add `on_nym_node_unbond`, delete `remove_node_measurements`, `partial_remove_epoch_measurements` and `remove_epoch_measurements`, and update `all_execute_variants_are_covered`
- [x] 6.2 In `performance_query_client.rs` add `get_latest_performance_paged` and `get_performance_digest` (not `get_digest`, which the directory query client already has), add a `get_all_latest_performance` collector through `collect_paged!`, re-export the new types, and update `all_query_variants_are_covered`
- [x] 6.3 From the root workspace: `cargo check -p nym-validator-client --tests`

## 7. Root-workspace consumers, schema and wasm checks

- [ ] 7.1 In `tools/internal/localnet-orchestrator/src/orchestrator/setup/cosmwasm_contracts.rs`, give `mixnet_init_message` a placeholder `performance_contract_address` like its siblings, and `mixnet_migrate_message` one too (state updates are skipped there, so it is never applied). In `common/commands/src/validator/cosmwasm/generators/mixnet.rs`, add a `--performance-contract-address` argument falling back to `var_names::PERFORMANCE_CONTRACT_ADDRESS`, like the siblings. `cargo check -p localnet-orchestrator -p nym-cli-commands`
- [ ] 7.2 `cargo check -p nym-validator-client -p nym-api` from the root workspace
- [ ] 7.3 Regenerate the schemas with `make -C contracts performance-schema mixnet-schema geolocation-schema`, and the directory's through its `schema` bin (it has no make target), since the shared `DigestResponse` doc reaches their response schemas
- [ ] 7.4 `make docker-check-contracts` for the wasm checks (locals limit included) on both contracts; local wasm builds are broken, so build release wasm only through `make publish-contracts`

## 8. Final verification and hand-off

- [ ] 8.1 `cargo fmt` across both workspaces. In `contracts`: `cargo test -p nym-performance-contract -p nym-mixnet-contract`. In the root: `cargo test -p nym-performance-contract-common` and `cargo check -p nym-performance-contract-common -p nym-validator-client -p nym-api -p localnet-orchestrator`
- [ ] 8.2 Read the full `git diff` against `develop` once:
  - every spec scenario has a test;
  - the storage namespaces match the public layout requirement;
  - nothing still mentions `RemoveNodeMeasurements`, `RemoveEpochMeasurements`, `EPOCH_PERFORMANCE_PURGE_LIMIT` or admin removals;
  - no commit or `Co-Authored-By` line was created, and the work is left uncommitted for review
- [ ] 8.3 Archive the change in the same PR. `openspec archive` aborts on the MODIFIED pointer and `RewardingInputs` blocks, because they deliberately drop main's "The pointer survives a purge of its epoch" and "A removed newest bundle yields nothing" scenarios, so sync those two requirements and the REMOVED admin-removals requirement by hand. Rewrite the main spec's `## Purpose` (digest over each node's latest bundle, append-only bundles, R-anchored fallback), then `git diff openspec/specs` and account for every removed line
- [ ] 8.4 Draft the PR description in a fenced markdown block
