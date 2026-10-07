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

## 4. Reviewer pass (before implementation)

- [ ] 4.1 CTO: confirm the scope (fleet aggregates in `NetworkSummary`, tiered history, no hourly per-gateway history) and the storage budget (~50 MB steady state)
- [ ] 4.2 NS API maintainer: walk Decisions 1-7 and Open Questions 1-6 in `design.md`; record each as accepted, changed (edit the spec) or deferred (follow-on change)
- [ ] 4.3 NS API maintainer: confirm the derivation refactor (Decision 3) is acceptable on the dVPN read path, or propose an alternative that still guarantees one implementation

## 5. Implementation (draft for the implementer; refine after 4.x)

- [ ] 5.1 Move the weighted `performance_v2` score, the score tier and the load tier out of `http/models/gw_probe/mod.rs`, and the dVPN filter/enrich/sort pipeline out of `http/state.rs`, into a module callable from both the HTTP layer and the monitor; expose the numeric weighted score, not only its tier
- [ ] 5.2 Parity test: `/dvpn/v1/directory/gateways*` output is identical before and after 5.1
- [ ] 5.3 Add `NetworkStats` and `network: Option<NetworkStats>` on `NetworkSummary` (`db/models.rs`); `get_summary` reads the optional `network.stats` key (`db/queries/summary.rs`)
- [ ] 5.4 Compute the stats as monitor step 15 (`monitor/mod.rs`) and write `network.stats` with the summary keys (`db/queries/misc.rs`); skip steps 15-17 when any earlier step failed
- [ ] 5.5 Migration in `migrations_pg/`: `summary_history_hourly`, `country_stats_hourly`, `country_stats_daily`, `gateway_daily_stats` with the keys, types and cascading FK from the persistence delta, plus an index on `summary_history_hourly.timestamp_utc`
- [ ] 5.6 History snapshot (step 17): due check for the current UTC hour, the four writes and pruning in one transaction; a failure is logged and does not fail the cycle
- [ ] 5.7 Config: `history_hourly_retention_days` (default 90) and `history_daily_retention_days` (default 365) in `cli/mod.rs`
- [ ] 5.8 Routes in `http/api/summary.rs` and `http/api/gateways.rs`; `days` whitelist parsing with the 400 body; country parsing shared with the dVPN country routes; `offset` on `/v2/summary/history`
- [ ] 5.9 Keyed `moka` caches for the four history routes and the `offset`-keyed summary-history cache in `http/state.rs`; `Cache-Control: public, max-age=300` on all five history responses
- [ ] 5.10 `#[utoipa::path]` annotations and schemas for the new routes and `NetworkStats`
- [ ] 5.11 Tests: one snapshot per hour across twelve cycles; gap on a failed hour; daily rows freeze at the last snapshot; pruning boundaries for both retention windows; `summary_history` never pruned; whitelist 400s; unrecognised country 400; unknown gateway 400 echo; `network: null` before the first stats write; means `null` over empty sets

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
