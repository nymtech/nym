All paths are under `nym-network-monitor-v3/nym-network-monitor-orchestrator/src` unless stated otherwise.

## 0. Skeleton

- [x] 0.1 Thread the submission through the code before filling it in, `todo!()` at the pure leaves: `Materialiser<C, S>` with `materialise_and_submit` and `delay_after` and the submission step wired (task 1.1's split and task 5.7's wiring, without their tests); `PerformanceSubmitter::submit`'s step sequence; the shared `SigningClient` newtype in `signing_client.rs` (task 5.4), built from the client config and mnemonic, replacing the bare `Arc<RwLock<..>>` in the orchestrator and `AppState`; `NetworkMonitorStorage`'s epoch-keyed reads taking and returning `EpochId`, with `http/state.rs` following; the config fields and CLI arguments (task 3.1); signatures only for the plan builder and the resume rule (tasks 5.1, 5.3), the shared fold (task 2.1) and `get_described_node_roles` (task 2.2). Signals are left as comments for task 4.1. `cargo check -p nym-network-monitor-orchestrator --all-targets` passes

## 1. Retry a failed materialisation pass (`aggregation/materialiser/mod.rs`)

- [x] 1.1 Split `Materialiser::run` into `materialise_and_submit(&interval) -> anyhow::Result<()>`, which stops at the first failed step (each step with its context; `run` logs the error once), and a pure `delay_after(succeeded, &interval)`: `MIN_CHECK_INTERVAL` after a failure, `until_epoch_end` after success. Update the docs of `run` and `MIN_CHECK_INTERVAL` to say a failed pass is retried at the floor
- [x] 1.2 Tests: the delay after a failure is the floor while the epoch's end is an hour away, and the delay after success is the time to the epoch's end; a pass whose config step fails, through a test type implementing `MixnetQueryClient` that fails every query, reports failure, and a second pass leaves the aggregates the first one stored unchanged

## 2. One fold of an epoch's rows, and the described nodes' roles

- [x] 2.1 Move the per-node union of config-score and aggregate rows out of `http/state.rs`'s `epoch_records` into `aggregation`, yielding per node in ascending order the optional `MixnetEpochConfigScore` and its `ReportedAggregates` (design Decision 6). `epoch_records` maps it to `NodeEpochAggregates`, and its existing tests pass unchanged
- [x] 2.2 In `storage` add `get_described_node_roles()`: node id, `mixnode_enabled` and `gateway_enabled` of every row of `nym_node_description`, one plain static query, with a manager test that an undescribed bonded node is absent and that both roles are read
- [x] 2.3 `cargo test -p nym-network-monitor-orchestrator`

## 3. Configuration and the startup check

- [x] 3.1 In `cli/env.rs` and `cli/run_orchestrator.rs` add `--performance-submission-enabled` (`bool` taking an explicit value like `--liveness-enabled`, default `false`), `--performance-submission-batch-size` (`NonZeroUsize`, default 200) and `--performance-submission-coverage-threshold` (`f64`, default 0.5), each with an env var in the existing naming and a doc saying what it bounds; carry all three into `Config` and `test_config`
- [x] 3.2 `Config::validate` rejects a coverage threshold outside `[0, 1]` with an error naming the value, beside the existing chain interactions penalty check, with a test
- [x] 3.3 In `orchestrator/mod.rs`, when `performance_submission_enabled` is set, verify right after `verify_orchestrator_chain_authorisation` and before `reconcile_announced_identity_key` that `get_network_monitor(address)` returns an entry, retrying with `chain_authorisation_check_max_attempts` and `chain_authorisation_check_retry_delay` and failing startup once they are spent, logging as the network-monitors check does (through the shared `SigningClient`, which implements `PerformanceQueryClient`). With submission off, log once that it is off
- [x] 3.4 Add `nym-contracts-common` with the `naive_float` feature to the orchestrator's `Cargo.toml`, for the percent conversion
- [x] 3.5 `cargo test -p nym-network-monitor-orchestrator`

## 4. Signals (`orchestrator/prometheus.rs`, `orchestrator/node_refresher.rs`)

- [ ] 4.1 Add to `PrometheusMetric`, each with its `help` property and its metric in `to_registrable_metric`: the int gauges `PerformanceSubmissionEpoch`, `PerformanceSubmissionLastCompleteEpoch`, `PerformanceSubmissionMixnodeLivenessOmitted`, `PerformanceSubmissionGatewayLivenessOmitted` and `PerformanceSubmissionStressOmitted`, and the counters `PerformanceSubmissionChunksLanded` and `PerformanceSubmissionChunkFailures` (design Decision 9); update the variant count in the existing test
- [ ] 4.2 In `get_node_details`, raise both describe-failure logs, the timeout and the error, from `debug` to `info`, keeping their node id and context
- [ ] 4.3 `cargo test -p nym-network-monitor-orchestrator`

## 5. The submission (`submission` module)

- [ ] 5.1 `submission/plan.rs`, pure: from the epoch's config-score and aggregate rows, the described roles and the threshold, compute each test kind's coverage over its eligible nodes (the role `select_candidates` assigns it to; coverage 0 for an empty population, never `0.0 / 0.0`), drop the rows of every test kind below the threshold, fold the rest with the shared union (task 2.1), and keep each node with a config score and at least one of liveness or stress, its values converted to `Percent` (clamped to `[0, 1]`, then `naive_try_from_f64`), in ascending node order. Return the plan, the omitted test kinds and each test kind's coverage
- [ ] 5.2 Tests for 5.1: an unmeasured kind is omitted; a measured zero is submitted; a node without a config score is not submitted; a config score alone is not submitted; a test kind below the threshold is omitted for every node; a gateway-only node is not eligible for mixnode liveness or stress; a zero is eligible but not reachable; once gateway liveness is omitted a dual-role node's liveness comes from its mixnode runs alone and a gateway-only node is not submitted; every test kind omitted leaves an empty plan; an empty population omits its test kind; the plan is in ascending order; a value a hair above 1.0 is clamped
- [ ] 5.3 The resume rule, pure: given the plan, the epoch being submitted and the cursor, keep the nodes above the cursor's node at the same epoch, every node for an earlier epoch, and fail for a later one; with tests for all three
- [x] 5.4 `signing_client.rs`: `SigningClient`, a `Clone` newtype over the shared `Arc<RwLock<DirectSigningHttpRpcValidatorClient>>` (done in 0.1), implementing `PerformanceSigningClient` (the write lock for one `execute_performance_contract`) and `PerformanceQueryClient` (the read lock for one `query_performance_contract`), through `async_trait` as the traits require
- [ ] 5.5 `PerformanceSubmitter<C: PerformanceQueryClient + PerformanceSigningClient>` holding storage, the orchestrator's own address, the batch size and the threshold, with `submit(epoch)`: read the cursor (no entry fails), read the epoch's rows and the roles, build the plan, apply the resume rule, send each chunk through `batch_submit_performance` without reading its response, stop at the first failed chunk with an error, and set the signals of design Decision 9: the omitted flags, the last complete epoch once nothing is left, the chunk counters, and one `info` line with the plan's facts
- [ ] 5.6 Tests through a test type implementing `PerformanceQueryClient` (answering `NetworkMonitor` from its fields) and `PerformanceSigningClient` (recording each `BatchSubmit` and returning a bare `ExecuteResult`), over in-memory storage: 450 nodes go out as chunks of 200, 200 and 50 in ascending order under the epoch; a cursor mid-epoch resumes after it; a failing second chunk stops before the third, and the next call resumes from the cursor the first left; no network-monitor entry fails without sending; a cursor at the last planned node sends nothing and returns `Ok`; a cursor at a later epoch fails without sending
- [ ] 5.7 Wire it in: `Materialiser<C, S>` gains `Option<PerformanceSubmitter<S>>`, run after both members only when both succeeded and `get_current_epoch_status` reports the epoch in progress, its failure failing the pass (task 1.1), a status that cannot be read failing it too, and an epoch in transition skipping it without failing. With a submitter present, the pass sets `PerformanceSubmissionEpoch` to the reading's epoch straight after reading the interval, before either member runs. `NetworkMonitorOrchestrator::run` builds it when `performance_submission_enabled` is set, over a clone of `self.client`. Update the materialiser's type doc and the `run()` step comments (the wiring and docs are done in 0.1; what remains is the signal line)
- [ ] 5.8 Test: a pass whose config step fails sends nothing through the test signer of 5.6
- [ ] 5.9 `cargo test -p nym-network-monitor-orchestrator`

## 6. Final verification and hand-off

- [ ] 6.1 `cargo fmt`, then `cargo test -p nym-network-monitor-orchestrator -p nym-network-monitor-orchestrator-requests`, `cargo check -p nym-network-monitor-orchestrator --all-targets` and `cargo check -p nym-network-monitor-orchestrator-requests --features openapi`
- [ ] 6.2 Read the full `git diff` once:
  - every scenario in the delta specs has a test, except those design Decision 10 leaves untested (the startup retry loop, the in-transition branch, the describe log level, the metric writes) and the `nym-network-monitor` task-list wording;
  - nothing is committed, and no `Co-Authored-By` line is added
- [ ] 6.3 Archive the change in the same PR (`openspec archive network-monitor-contract-submission`, which creates the `network-monitor-contract-submission` spec, appends the retry requirement to `network-monitor-epoch-performance` and replaces its HTTP API requirement, and in `nym-network-monitor` replaces the startup-gates requirement and renames and replaces the task-list one), write the new spec's `## Purpose` in place of the generated placeholder, then `git diff openspec/specs` and account for every line
- [ ] 6.4 Draft the PR description in a fenced markdown block
