## 1. Shared `nym-config-score` crate

- [x] 1.1 Create `common/nym-config-score` and register it in the workspace members and `[workspace.dependencies]`. It depends only on `nym-mixnet-contract-common` (for `OutdatedVersionWeights`, `VersionScoreFormulaParams`, `HistoricalNymNodeVersionEntry`), `semver`, and the `Coin` type (Decision 1, Decision 4)
- [x] 1.2 `ConfigScoreCalculator` holds the policy plus the `Vec<HistoricalNymNodeVersionEntry>` release chain, constructed once per pass, taking `minimum_balance`, `chain_interactions_penalty` and the contract's own `ConfigScoreParams` (weights + formula params reused directly rather than re-wrapped) (Decision 2)
- [x] 1.3 Implement `has_sufficient_tokens(&self, balance: Option<&Coin>) -> bool` and `score(&self, node: &NodeConfigInputs) -> ConfigScoreOutcome { score, versions_behind }`. `score` computes `versions_behind` internally via `OutdatedVersionWeights::versions_behind_factor` over the held chain, applies the gates and the chain-interaction penalty, and returns the number plus `versions_behind` for the caller's decomposition (Decision 3)
- [x] 1.4 Take per-node data as `NodeConfigInputs { reported_version: Option<&semver::Version>, runs_nym_node, accepted_terms, balance: Option<&Coin>, is_feegrant_grantee }`, mapping `reported_version: None` to a zero score so the caller owns the no-describe versus unparseable-version distinction (Decision 2)
- [x] 1.5 Unit tests over literal inputs, no mocks: a latest, compliant, transacting node scores 1.0; unaccepted terms, a non-`nym-node` binary, and a `None` version each score 0.0; a version several releases behind scores below 1.0; a sub-minimum balance with no feegrant multiplies by `(1 - penalty)` while a feegrant rescues it. (nym-api has no config-score unit tests to port; parity is instead pinned by sharing the code and by task 2.3)

## 2. nym-api refactor onto the shared crate

- [ ] 2.1 Refactor nym-api's `calculate_config_score` to build a `ConfigScoreCalculator` from its `ConfigScoreData` (params plus version history) and its `minimum_on_chain_balance` and `chain_interactions_penalty` config, then call `.score()`; drop nym-api's local `has_sufficient_tokens` and `versions_behind_factor_to_config_score` in favour of the crate (Decision 1)
- [ ] 2.2 Keep nym-api's response assembly in the adapter: `ConfigScoreV2` wraps the shared `ConfigScoreOutcome` plus the flags nym-api already sets (`self_described_api_available`, terms, binary), passing `reported_version: None` for both the unavailable and bad-semver cases while still setting those flags itself
- [ ] 2.3 Confirm nym-api's existing config-score tests pass unchanged, establishing the refactor is behaviour-preserving and the produced score is identical

## 3. Describe-input capture in the orchestrator

- [ ] 3.1 Extend the describe extraction in `node_refresher` to read the reported version and binary name (a build-information read) and the terms acceptance and on-chain address (already present in the `auxiliary_details` the refresher fetches), carrying them on `SelfDescribedData` (Decision 8)
- [ ] 3.2 Migration adding nullable columns to `nym_node`: reported version, binary name, terms acceptance, declared chain address; persist them on the refresh sweep alongside the networking fields
- [ ] 3.3 Tests: a refreshed node stores the four inputs; a node whose describe omits them stores nulls rather than failing the refresh

## 4. Chain-capability cache

- [ ] 4.1 Migration adding `node_chain_capability(node_id, balance_amount TEXT, is_feegrant_grantee, refreshed_at)` keyed by node id, holding the RAW balance rather than a sufficiency decision (Decision 7)
- [ ] 4.2 `storage/manager.rs`: upsert a capability row, read capabilities for the whole population in one query, and select nodes whose row is due (stale or missing), applying the per-node jitter when judging due-ness
- [ ] 4.3 A warm-keeping background sweep querying `get_balance` and `allowances` for due nodes on its own interval with bounded concurrency, generic over the narrowest existing validator-client bound so tests inject a mock rather than an HTTP server (Decision 2, Decision 7). It stores the raw balance amount, and the epoch-transition materialiser only READS the cache - the sweep never runs on the materialisation path
- [ ] 4.4 Jitter each node's next-due time by `ttl + rand(0, jitter_span)` drawn per refresh, so a cold-start population does not all fall due together one TTL later (Decision 7). Store the jitter or the resulting due time on the capability row
- [ ] 4.5 Config knobs with orchestrator defaults matching nym-api: `chain_capability_refresh_interval` (24h), a capability refresh jitter span, capability query concurrency (8), `minimum_on_chain_balance` (1 NYM), `chain_interactions_penalty` (0.2), env-overridable and hidden from the primary CLI surface per the house convention for tuning knobs
- [ ] 4.6 Tests with a mock chain client and an in-memory sqlite pool: a refresh stores the raw balance and feegrant flag; a due row is re-queried while one not yet due is not; concurrency is bounded; two nodes refreshed together get spread rather than identical next-due times

## 5. Config-score storage

- [ ] 5.1 Migration adding `mixnet_epoch_config_score` keyed `(mixnet_epoch, node_id)` with the score and the decomposition columns (versions behind nullable, terms accepted, runs nym-node, self-described available, sufficient tokens, feegrant grantee). It is not a `test_kind` and does not touch the aggregate table's kind CHECK (Decision 6)
- [ ] 5.2 Add the row type to `storage/models.rs` and the manager queries: an idempotent batch upsert (`ON CONFLICT DO NOTHING`), a read of every config score for an epoch, and a point read for one `(node, epoch)`
- [ ] 5.3 Tests: a repeated upsert neither duplicates nor alters; a read returns the full decomposition

## 6. Computation and materialisation

- [ ] 6.1 Add a config-score step to the materialiser that runs once per epoch (not per `TestKind`): query the config-score params and version history from the mixnet contract once, build a `ConfigScoreCalculator`, read node state and cached capabilities from storage, compute a score per bonded node, and batch-insert them (Decision 5)
- [ ] 6.2 Drive the step from the bonded-node registry so every node gets a row; a node missing describe data scores an unavailable zero rather than being skipped (Decision 9)
- [ ] 6.3 Add the mixnet-contract queries the step needs (`get_config_score_params`, `get_nym_node_version_history`), and run the step inside the existing epoch-transition and backfill loop, where a backfilled epoch is computed from current state since config score has no history to replay (Decision 5)
- [ ] 6.4 Tests with a mock `MixnetQueryClient` and an in-memory sqlite pool: an epoch materialises one config-score row per bonded node; a node without describe scores zero flagged unavailable; raising the minimum balance moves the next epoch's sufficiency using the cached balances without a re-query; a re-run is idempotent

## 7. Response type and HTTP serving

- [ ] 7.1 Add a `ConfigScore` type carrying the decomposition to `nym-network-monitor-orchestrator-requests`, and add a NON-optional `config_score` field to `NodeEpochAggregates` and its `::new`, leaving liveness and stress optional (Decision 9)
- [ ] 7.2 In `http/state`, drive the epoch response and the per-node read from the config-score rows, one per node, attaching the optional probe aggregates; source config score separately rather than through the score-shaped `kind_aggregate` fold. A node lacking a config-score row falls back to an unavailable zero
- [ ] 7.3 Tests: every node in an epoch response carries a config score; a node scored but not probed in the window appears with the probe fields absent; a node with no self-description is present with a zero flagged unavailable rather than omitted

## 8. Verification

Post-deploy runtime checks, not implementation. The change ships dark - a new field, table and task that nothing consumes - so these gate whether anything is built on the value and cannot run until it is live. Written up as a deploy runbook outside the repo and carried in the PR description; left unchecked here because they are genuinely not yet done.

- [ ] 8.1 Deploy and confirm the capability cache fills and the first epochs materialise one config score per bonded node without error, as the describe sweep and capability cache populate
- [ ] 8.2 Compare the orchestrator's config score against nym-api's `ConfigScoreV2` for a sample of nodes, read as steady-state agreement, since the orchestrator snapshots at the epoch transition while nym-api recomputes continuously and a node that changed its configuration within the epoch is expected to differ until the next snapshot
- [ ] 8.3 Confirm a change to the minimum balance moves scores on the following epoch using the cached balances, without a burst of re-queries
- [ ] 8.4 Confirm discarding the database does not block startup and that config scores recover as the capability cache and describe data refill, per Decision 7
