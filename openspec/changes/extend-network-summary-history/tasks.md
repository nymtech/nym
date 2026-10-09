## 1. Research inputs

- [x] 1.1 Read `node-status-api-monitoring`, `-persistence` and `-http` specs and confirm every aggregate input is already stored as current state (`summary`, `nym_nodes`, `gateways`, `node_families`/`node_family_members`)
- [x] 1.2 Locate the per-gateway derivations the stats must reuse: `calc_gateway_visual_score` / `calculate_weighted_score` and `calculate_load` in `http/models/gw_probe/mod.rs`, and the dVPN list build in `http/state.rs`
- [x] 1.3 Locate the summary write (`db/queries/misc.rs`, `INSERT INTO summary_history`) and read (`db/queries/summary.rs`, `get_summary`) paths and `NetworkSummary` in `db/models.rs`
- [x] 1.4 Measure storage on PostgreSQL 16 with production column conventions for every candidate layout and retention (results in `design.md`)

## 2. Author artifacts

- [x] 2.1 Write `proposal.md` (Why / What Changes / Capabilities / Impact)
- [x] 2.2 Write `design.md` (Context / Goals-Non-Goals / Decisions / Storage estimate / Risks / Migration / Open Questions)
- [x] 2.3 Write spec deltas for `node-status-api-monitoring`, `node-status-api-persistence` and `node-status-api-http`

## 3. Validate via openspec tooling

- [x] 3.1 Run `openspec validate extend-network-summary-history` and confirm it reports valid (`--strict` only adds requirement-length warnings, which the existing `node-status-api-*` specs also raise)
- [ ] 3.2 Run `openspec show extend-network-summary-history` and review the rendered output

## 4. Review (before implementation)

- [ ] 4.1 Get review of the scope (fleet aggregates in `NetworkSummary`, tiered history, no hourly per-gateway history) and the storage budget (~50 MB steady state)
- [ ] 4.2 Get review of Decisions 1-7 and Open Questions 1-6 in `design.md`; record each as accepted, changed (edit the spec) or deferred (follow-on change)
- [ ] 4.3 Get review of the single-implementation requirement for the per-gateway derivations (Decision 3) and of which components to rework rather than extend

## 5. Implementation (iterations on a topic branch)

Each iteration is one reviewable slice merged into the topic branch with its own tests; the topic branch merges to `develop` once all slices are done. Requirements are behavioural, so each iteration may rework the components it touches.

### Iteration 1 - network stats

- [ ] 5.1 One implementation of the weighted `performance_v2` score, the score tier, the load tier and the default-filtered directory list, used by both the dVPN directory and the stats
- [ ] 5.2 Parity test: `/dvpn/v1/directory/gateways*` output is byte-identical before and after 5.1
- [ ] 5.3 Monitor step 15 computes the stats; step 16 writes `network.stats`; steps 15-17 skipped when an earlier step failed
- [ ] 5.4 `/v2/summary` serves `network` (`null` when absent); the daily `summary_history` row carries it
- [ ] 5.5 Tests: field populations and definitions, means `null` over empty sets, outdated-gateway exclusion, `network: null` before the first stats write

### Iteration 2 - global hourly history

- [ ] 5.6 `summary_history_hourly` table; step 17 writes one row per UTC hour after a completed cycle, in one transaction with hourly pruning; a failure does not fail the cycle
- [ ] 5.7 `history_hourly_retention_days` config (default 90)
- [ ] 5.8 `GET /v2/summary/history/hourly` with the `days` whitelist, oldest-first, keyed cache, `Cache-Control`
- [ ] 5.9 `offset` on `GET /v2/summary/history`, `offset`-keyed cache, `Cache-Control`
- [ ] 5.10 Tests: one row per hour across twelve cycles, gap on a failed hour, pruning boundary, whitelist 400, `summary_history` never pruned

### Iteration 3 - per-country history

- [ ] 5.11 `country_stats_hourly` and `country_stats_daily` tables, written in the step 17 transaction; `history_daily_retention_days` config (default 365)
- [ ] 5.12 Hourly and daily country routes with country parsing shared with the dVPN country routes, `days` whitelists, keyed caches, `Cache-Control`
- [ ] 5.13 Tests: daily rows freeze at the last snapshot of the day, both pruning boundaries, unrecognised country 400, recognised country without rows `[]`

### Iteration 4 - per-gateway daily history

- [ ] 5.14 `gateway_daily_stats` table with cascading FK to `nym_nodes`, written in the step 17 transaction
- [ ] 5.15 `GET /v2/gateways/{identity_key}/history` with the `days` whitelist, keyed cache, `Cache-Control`
- [ ] 5.16 Tests: unknown gateway 400 echo, gateway that left the network keeps rows until retention

### Every iteration

- [ ] 5.17 OpenAPI annotations and schemas for what the iteration adds
- [ ] 5.18 Spec deltas updated if the iteration changes any specified behaviour

## 6. Open questions follow-up

- [ ] 6.1 Resolve Open Question 5 (`routing_score` / `config_score`): restore scoring and add `config_score` to `gateway_daily_stats`, or remove the dead fields and columns, in a follow-on change
- [ ] 6.2 Record outcomes for Open Questions 1-4 and 6 in `design.md`

## 7. Consumers

- [ ] 7.1 Switch `load.nymte.ch` (graphs, top bar, country pages) to `/v2/summary`, the four history routes and `/dvpn/v1/directory/gateways`
- [ ] 7.2 Switch `export_nym_network_stats.py` to `/v2/summary` and `/v2/summary/history/hourly?days=7`
- [ ] 7.3 Retire the interim `load.nymte.ch/api/v0` and its Swagger page

## 8. Archive the change

- [ ] 8.1 After implementation lands on `develop`, run `openspec archive extend-network-summary-history` so the deltas merge into `openspec/specs/`
- [ ] 8.2 Update the codex Node Status API pages from the archived specs
