## 1. nym-api: remove the v1 monitor and the legacy figure

- [x] 1.1 Delete `nym-api/src/network_monitor/` and its wiring in `support/cli/run.rs` (the monitor start, the uptime updater start, the `network_monitor` argument of `RouterBuilder::with_default_routes`), the monitor's credentials storage paths, and `node_status_api/uptime_updater.rs`
- [x] 1.2 Delete `node_performance/provider/` (the legacy storage provider and `NodePerformanceProvider`), the legacy branch of the node-status refresher (`legacy_annotations`, `produce_node_annotations`, the chain-capability lookups, the coverage guards and the carry-over), so `update_node_annotations` is the contract branch alone; keep the `contract_node_annotations` file name
- [ ] 1.3 Remove the config sections `[network_monitor]` and `[performance_provider]` (switch, scoring, debug), their template entries, `PerformanceProvider::apply_deprecated_fields`, the related validation, the `--enable-monitor` and `--monitor-credentials-mode` flags of both `init` and `run` with their `OverrideConfig` fields, and the `[node_status_api.debug]` chain-capability knobs (`minimum_on_chain_balance_amount`, `chain_capabilities_retrieval_concurrency`, `chain_capabilities_refresh_interval`) that only the legacy branch read; `[network_monitors_cache]` stays; existing files with the removed sections still load
- [ ] 1.4 Remove the POST handlers (the two v2 submit routes of `node_status_api/handlers/without_monitor.rs`, the two v3 batch-submit handlers) with their OpenAPI entries and the request models only they used; keep every GET handler: `handlers/{network_monitor,unstable}.rs` mounted unconditionally (the `status_routes` flag already went in 1.1, with the router argument), the three history handlers, the known-monitors handler, and the two divergence handlers reduced to their unconditional 503; mark each kept GET route `#[deprecated]` and deprecated in OpenAPI with a doc line saying its data is frozen
- [ ] 1.5 Delete `LastNMSubmissions` and its `AppState` field; keep `NetworkMonitorsCache` for the known-monitors route
- [ ] 1.6 Storage: delete the writers (`insert_monitor_run_*`, `submit_*_statuses_v2`, `insert_nym_node_*_results`, `update_historical_uptimes`, `purge_old_statuses`) and the aggregations only the legacy provider and the uptime updater used; keep every reader a GET route uses; the two `core-status-count` readers anchor their default 30-day window on the last recorded monitor run instead of the current time, so they keep answering the cutover's counts rather than decaying to 0; add one migration dropping the `*_v2` tables and the two v3 result tables, every v1 monitor table staying frozen
- [ ] 1.7 Remove the `v2-performance` cargo feature with its storage branch
- [ ] 1.8 `cargo check -p nym-api --all-targets`, nym-api tests, fmt; `nym-api-requests` with `mock-fixtures`, `nym-api-requests` ts bindings unaffected or regenerated
- [ ] 1.9 Audit every `nym-api` dependency (normal, dev, build) and remove those nothing uses any more, including ones unused before this change

## 2. nym-api: remove rewarding

- [ ] 2.1 Remove the `[rewarding]` section, `--enable-rewarding` from `init` and `run` with its `OverrideConfig` field, `ensure_rewarding_permission`'s call, the window start (`start_performance_contract_window`) and the epoch advancer start from `run.rs`, the `performance_contract_window` and `use_performance_contract_data` fields of `AppState`, `PerformanceSource`; the `contract-divergence` handler loses its window branch and answers its "does not reward" 503 unconditionally
- [ ] 2.2 Remove the epoch transactions and the `nyxd_signing!` uses they alone needed from `support/nyxd/mod.rs` (keep what ecash and the DKG use), and delete `support/sampling.rs` from nym-api
- [ ] 2.3 Delete `nym-api/src/epoch_operations/` and `nym-api/src/node_performance/contract_window/` from nym-api as `git mv` into the driver (group 4) rather than a delete, so the move commits read as renames
- [ ] 2.4 Update `support/caching/startup.rs` docs and the `nym-api-startup` task list wording; `cargo check -p nym-api --all-targets`, tests, fmt

## 3. Shared cadence helper

- [ ] 3.1 Move the boundary arithmetic of `SnapshotCadence` (`newest_height`, `retained_heights` over the snapshot interval) from `nym-api/src/support/attested_snapshots/producer.rs` into a helper on the directory side (the directory common crate or `nym-directory-client`), with its tests
- [ ] 3.2 nym-api's producer calls the shared helper; `cargo check -p nym-api --all-targets`, the producer's tests pass

## 4. The driver crate

- [ ] 4.1 Create `nym-epoch-driver/` (workspace member, binary) with `cli/{init,run,build_info}.rs`, `config/` (file under `~/.nym/nym-epoch-driver/` with no `--id`, template, env then flag overrides in nym-api's precedence, required mnemonic, nyxd URL, metrics token, validation refusing a missing or empty token; listener bind default `0.0.0.0:8090`; hidden debug knobs: window polling interval, tip polling interval, settle lag), `main.rs`, and a signing `nyxd` client wrapping the moved epoch transactions
- [ ] 4.2 `git mv` the epoch operations, the contract window and the sampler into the crate; adjust `use` paths; the window's refresher reads the current interval live instead of the mixnet cache; the rewarded-set read loses its cache fallback; `nodes_to_reward` reads the rewarding parameters live
- [ ] 4.3 Network view: a refresher producing, at each newly settled cadence boundary, the node details and the verified directory at that height through the shared verified read (`verify_chain_read` over a `ProvenChainAnchor` on the driver's own nyxd, always trusted, no light client, as nym-api's producer calls it, since `DirectoryClient::verified_directory` reads the bonds itself) with the identity map from the details; the details come from a new `get_all_nymnodes_detailed_at_height` on `PinnedMixnetQueryClient`, paged at one height like `get_all_nymnode_bonds_at_height`, since no pinned query returns every node's stake; the previous view stays on failure, logged; the first production runs at startup and fails startup on error
- [ ] 4.4 Roles: decode each node's `NodeInformation` label from the verified directory (prost) and map `modes` to the three selectable roles, `modes.exit` alone for exit; no record or no modes means no role
- [ ] 4.5 Selection candidates come from the network view (bond, stake, roles) instead of the mixnet cache and the describe cache; `update_rewarded_set_and_advance_epoch` takes the view
- [ ] 4.6 Startup checks in order: rewarding permission, directory and performance contract addresses present, window load (failing on no submission), first network view production
- [ ] 4.7 Metrics: three `nym-metrics` gauges, each a Unix timestamp in seconds: `current_epoch_end_timestamp_seconds` and `epoch_loop_timestamp_seconds`, set at each interval read of the loop (retries included), and `network_view_timestamp_seconds`, set at each successful production; written where the loop and the refresher already have the facts, metric values left untested (the decisions they are set from are tested); a one-route axum listener bound to `0.0.0.0` by default, behind the orchestrator's bearer-token auth layer
- [ ] 4.8 Tests: the moved window, resolution, rewarding and selection tests pass in the crate; new tests for the network view (one read serves stake and identities, a failed production keeps the previous view, selection reads the view as held), the roles mapping (modes, exit alone, no record), the startup checks (another account refused, no submission, missing address), config overrides precedence, a missing or empty metrics token refusing startup, metrics auth
- [ ] 4.9 `cargo check -p nym-epoch-driver --all-targets`, tests, fmt

## 5. Orchestrator

- [ ] 5.1 Delete `orchestrator/result_submitter.rs`, the nym-api client construction, `nym_api_endpoint` (config, CLI flag, env), `result_submission_interval` and `result_submission_batch_size`, and `verify_orchestrator_nym_api_authorisation` with its startup gate; with them the `nym-api-requests` dependency, the submission metrics (`SubmissionOutcome`, `SubmittedResultsTotal`) and the submission projections in `storage/models.rs`
- [ ] 5.2 One migration dropping `submission_watermark`; delete the watermark storage methods; the task count in docs and the spec's six tasks
- [ ] 5.3 `cargo check -p nym-network-monitor-orchestrator --all-targets`, tests, fmt

## 6. Standalone v2 crate and clients

- [ ] 6.1 Delete `nym-network-monitor/`, its workspace entry, `nym-network-monitor.dockerfile`, the `.gitignore` and README lines, and the CI path trigger in `.github/workflows/ci-build.yml`
- [ ] 6.2 Validator client: delete the client methods and route constants of the removed POST routes (submit routes, batch-submit); mark the methods of the kept GET routes `#[deprecated]`; delete `nym_types::monitoring` (`MonitorMessage`, `NodeResult`), unused once the v1 submit routes and the v2 crate are gone; the wallet is untouched
- [ ] 6.3 `cargo check --all-targets` for `nym-validator-client`, `nym-wallet` (src-tauri), `nym-node-status-api`, `nyx-checkpoint-updater`

## 7. localnet

- [ ] 7.1 Remove `insert_fake_network_monitor_runs` and its call, and the `--enable-monitor` / `--enable-rewarding` flags from the nym-api launch; update the localnet README
- [ ] 7.2 `cargo check -p localnet-orchestrator --all-targets`

## 8. Tests, docs and close-out

- [ ] 8.1 Live public-api tests: the tests of the kept GET routes assert the response shape rather than non-empty data; delete the tests of the removed POST routes
- [ ] 8.2 Full verification: fmt in the workspace, `cargo check --all-targets` of every touched crate, the nym-api, driver, orchestrator, validator-client and directory-side tests; contracts workspace untouched
- [ ] 8.3 Docs: the driver's README (init, config, overrides, metrics token, the cutover order: upgrade the orchestrator first or with nym-api, stop nym-api's rewarding before starting the driver, copy the SQLite files before the drop migrations, drop the three removed flags from nym-api's service units), nym-api's config docs for the removed sections and flags, including the generated `nym-api-init-help.md` command output
- [ ] 8.4 Spec sync and archive: a scratch-copy `openspec archive` dry run; the `network-monitor` capability's spec is removed by hand, the RENAMED and MODIFIED blocks applied, dropped scenarios restored where the archive would abort and then removed by hand; `openspec validate --specs --strict`; `git diff openspec/specs` reviewed line by line
- [ ] 8.5 PR description in a fenced markdown block: summary, what moves, what is removed, the kept history routes, deployment order and rollback, the untested list
