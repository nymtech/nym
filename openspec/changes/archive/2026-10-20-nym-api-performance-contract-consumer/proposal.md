## Why

The orchestrator now submits every epoch's liveness, stress and config figures to the performance contract (`network-monitor-contract-submission`), but nym-api cannot use them. With `use_performance_contract_data` on, validation forbids every scoring property while the contract score only enters as one of them, so no node gets an annotation: every node is rewarded zero and the rewarded set comes out empty. The contract path also reads rewards before the epoch's transition seals the contract, falls back through a loop that never runs, and would multiply nym-api's own config score into a figure that already includes one. This change makes nym-api consume the contract correctly, behind an explicit read flag, and gives the shadow period a route that compares nym-api's figure with the contract's before anyone switches rewarding over.

## What Changes

- **New read flag** `[performance_provider].read_performance_contract`, off by default. With it on, nym-api keeps a local window of the contract's data and serves the contract side of the gap route, and a performance contract address missing from the network details fails startup. `use_performance_contract_data` (the switch) now requires the flag and decides whether nym-api uses its own figure or the contract's.
- **Contract mode computes no figure of nym-api's own.** With the switch on, nym-api's scoring is skipped and the refresher writes no annotations until `nym-api-performance-attestation` builds contract-mode ones. The `[performance_provider.scoring]` section is validated as in legacy mode and has no effect. The rule rejecting any enabled scoring property alongside the switch is dropped, so the flip and a rollback are one line each, and a rollback returns to the weights that ran before.
- **The contract's own resolution rule, shared.** The rule `RewardingInputs` uses (the bundle at X, else the node's newest bundle in `[R - 24, min(X - 1, R)]`) moves into `nym-performance-contract-common` as a pure function over a per-epoch lookup. The contract calls it with its storage reads and nym-api with its window, so the two cannot drift. The contract's behaviour is unchanged.
- **A local window** of the contract's per-epoch medians for `[R - 24, R]` plus the current epoch X, and the weights at X, loaded at startup and kept current by re-reading X whenever the contract's last submission changes. It replaces the per-epoch history cache and the contract performance provider, both deleted.
- **Rewarding in contract mode** reads the window after `BeginEpochTransition` (so after the contract has sealed X), re-reading X first, and pays each rewarded node the contract's score directly, with no routing component and no second config multiplier. A node without a figure is paid zero. A failed read fails the epoch operation, which is retried, and never pays zero. Legacy mode reads annotations at the same point.
- **Active-set selection in contract mode** weights every candidate by the same figure. When nothing was submitted at X, every candidate with a figure above 0 gets factor 1, so selection is by stake alone, while a node last measured as completely dead stays out. A candidate without a figure is skipped.
- **Annotations carried over only when the gap is ours** (legacy mode). A refresh carries a node's previous annotation over only when its applied set is empty because the coverage guard dropped a component the node is eligible for. Any other node without a score is dropped and served 0, so a node that shut down is no longer served its last figure. Today the whole map is replaced, despite a comment claiming otherwise.
- **Startup.** Flag off: no performance data is read from the contract. Flag on and switch off: failures to load the window are logged and retried. Switch on, whether or not the nym-api rewards: a window that cannot be loaded, or a contract that has never received a submission, fails startup.
- **New temporary route** `/v3/unstable/nym-nodes/contract-divergence`. It puts each bonded node in one category (missing, contract-only, unscoreable, fallback, divergent, agree) by comparing nym-api's annotation with the contract's figure at X, and gives counts per declared role and a row for each node not in agreement. It is served only while nym-api has both sides (read flag on, switch off). Otherwise, and before the window has loaded, it answers 503 naming what is missing.
- **Removed:** the dead knobs `[performance_provider.debug].max_performance_fallback_epochs` and `max_epoch_entries_to_retain`. Not breaking: nym-api denies no unknown fields, so a config that still sets them loads and the values are ignored.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `nym-network-monitor`: the consumer-surface requirement for nym-api scoring is limited to legacy mode, changes its annotation rule (carry over only on our fault) and drops the scenario naming the contract provider. New requirements cover the read flag and the switch's dependency on it, the local window and its resolution with the contract's shared rule, rewarding and active-set selection in contract mode, the failed-read rule, startup, and the `contract-divergence` route.
- `network-monitor`: the requirement naming the nym-api consumers of the v1 monitor's score says that in contract mode annotations come from the performance-contract cache and that rewarding and selection read annotations. Both stop being true here: in contract mode the monitor's score feeds nothing, and rewarding and selection read the window.

`performance-contract` needs no delta. The extracted rule gives exactly the resolution its `RewardingInputs` requirement specifies, `RewardingScore` still goes through the same function, and no contract requirement says where that function lives.

## Impact

**Code:**
- `common/cosmwasm-smart-contracts/nym-performance-contract`: the shared resolution function beside `MAX_FALLBACK_LOOKBACK_EPOCHS`, with its tests.
- `contracts/performance/src/storage.rs`: `resolve_source_bundle` calls the shared function. The contract tests stay as they are, and the wasm check passes (contracts are pinned to MSRV 1.86).
- `nym-api/src/node_performance`: the contract cache and contract provider are deleted, and with them the provider trait. The window and its refresher are new.
- `nym-api/src/epoch_operations`: when rewards are read, the contract-mode reward and selection figures, and fault mode.
- `nym-api/src/node_status_api/cache`: the carry-over rule, and nym-api's own scoring run in legacy mode only.
- `nym-api/src/support/config` and `support/cli/run.rs`: the flag, validation (the switch requires the flag, the scoring rule for contract mode is dropped), the removed knobs, startup wiring, and the template.
- `nym-api/src/unstable_routes/v3/nym_nodes`: the gap route. `nym-api/nym-api-requests/src/models/network_monitor.rs`: its response types, with no TS bindings.

**APIs:** one new unstable route. No change to any existing route, wire type, message or contract, and no database migration.

**Deployment:** nothing changes until the flag is set. The shadow period runs with the flag on and the switch off. The switch stays off until `nym-api-performance-attestation` (contract-mode annotations for published performance) and the orchestrator's aliveness follow-up are deployed too.
