## Context

Every input these aggregates need is already stored as current state and refreshed each monitor cycle (300 s): node counts in `summary`, mixnet performance in `nym_nodes.performance`, the latest probe per gateway in `gateways.last_probe_result`, location and ASN in `gateways.explorer_pretty_bond`, bridges in `gateways.bridges`, families in `node_families`/`node_family_members`, and build information in `nym_nodes.self_described`. No new raw data is collected; this change only aggregates what exists and materialises the aggregates over time.

`performance_v2` and load are not stored: they are derived at read time inside the dVPN directory rebuild (`http/models/gw_probe/mod.rs`, `http/state.rs`). Monitor cycles are not atomic (see `node-status-api-monitoring`), so a snapshot must not be taken from a partly refreshed store.

## Goals / Non-Goals

**Goals**

- Expose fleet aggregates as part of `NetworkSummary`, computed exactly as the dVPN directory computes per-gateway values.
- Keep hourly and daily history, globally, per country and per gateway, with bounded storage.
- Serve history over a small, cacheable URL space.

**Non-Goals**

- Hourly per-gateway history.
- New raw data collection or probe changes.
- Changing the behaviour of any existing route beyond additive fields and parameters.
- Prescribing module layout: the requirements are behavioural, so the monitor cycle, dVPN pipeline and caching components may be reworked or simplified rather than extended, as long as the specified behaviour holds.
- Serving current per-country tables or residential-gateway lists: consumers derive them from `/dvpn/v1/directory/gateways`, which already carries location, ASN kind, performance and `performance_v2`.

## Decisions

### 1. Tiered, materialised history

| Series | Resolution | Retention | Why |
| --- | --- | --- | --- |
| global `NetworkSummary` | hourly | 90 days | graph zoom down to 24 h; 90 days covers every hourly view a dashboard needs |
| global `NetworkSummary` | daily | forever | existing `summary_history`, unchanged semantics, now carrying `network` |
| per country | hourly | 90 days | same views per country |
| per country | daily | 365 days | year-scale country trends at negligible cost |
| per gateway | daily | 365 days | operator-facing trends; hourly per gateway is the one tier with real size (~150 MB) and no current consumer |

Queries only ever read a bounded window over an indexed time column, so read cost does not grow with retention. Retention is configurable (`history_hourly_retention_days`, `history_daily_retention_days`) with these defaults.

*Alternative considered*: re-keying `summary_history` by hour. Rejected: `/v2/summary/history` is an existing contract served to the explorer as 30 daily rows; a parallel hourly table keeps it untouched at the cost of one small duplicated series.

### 2. Per-country history in its own table, not embedded in the summary row

A separate table keyed `(cc, timestamp_utc)` answers one country's history from an index range. Embedding a per-country map in each hourly summary row is smaller (measured 25 MB/year vs 58 MB/year at hourly-365) but every per-country read would parse all hourly summaries in the window. With 90-day hourly retention the table costs ~16 MB, so the indexable layout wins.

### 3. Stats computed in the monitor, with the dVPN directory's own derivations

The weighted `performance_v2` score, the load tier and the dVPN filter pipeline currently live in the HTTP layer. The stats need the same derivations from the monitor, so they must have exactly one implementation used by both paths: two implementations of the same derivation would drift; one cannot. How that is structured is left to the implementation, including reworking the dVPN pipeline rather than extracting from it. Stats are computed after the gateway snapshot (step 12) and written with the summary keys.

### 4. Stored as one summary key, served as a nullable `network` object

`network.stats` holds the serialised stats in the existing `summary` key-value table, so no new current-state table is needed and the daily `summary_history` row picks it up automatically. It is optional: after deployment, before the first cycle completes, `/v2/summary` serves `network: null` rather than turning the existing 500-on-missing-key rule into an outage.

### 5. Definitions

Gateway-derived aggregates are over the **directory list**: the dVPN gateway list as served by the default directory routes, i.e. the directory pipeline's output with the default minimum-version filter (`1.6.2`, unparsable versions dropped) applied. Using the unfiltered list instead would count gateways no client is ever offered, and its numbers could not be reproduced from any public route, because unparsable versions are dropped even with `min_node_version=0.0.0`. The two node-wide aggregates deliberately skip that filter: `families` describes family membership across all roles, and `build_versions` exists precisely to show how much of the network runs outdated software, which the filter would hide.

| Field | Population | Definition |
| --- | --- | --- |
| `gateways` | directory list | gateways in the list |
| `locations` | directory list | distinct `two_letter_iso_country_code` values |
| `performance_mean` | directory list | mean mixnet performance, 0..1 (the value served as `performance_v2.uptime_percentage_last_24_hours`) |
| `performance_v2_score_mean` | directory list | mean of the numeric weighted score (40/30/30) before tier bucketing, over gateways with a parsed probe |
| `load_mean` | directory list | mean of `1 - ping_ips_performance_v4`, over gateways with a WireGuard result; 0 = unloaded |
| `performance_tiers`, `load_tiers` | directory list | counts per `performance_v2.score` / `performance_v2.load` value |
| `quic_bridges` | directory list | gateways with any `bridges.transports[].transport_type` starting `quic` |
| `residential` | directory list | `gateways`, `locations`, `load_mean` over gateways whose `location.asn.kind` is `residential` |
| `families` | all family members, any role or version | `active` (families with ≥ 1 member), `nodes`, and `gateways` / `mixnodes` split by declared role (entry or exit-IPR → gateway, else mixnode → mixnode) |
| `build_versions` | all described nym-nodes, any role or version | count per `build_information.build_version` |

Both performance means are exposed because they measure different things: mixnet performance is the long-running uptime signal, the `performance_v2` score adds the WireGuard probe. Load becomes numeric because the tier mapping (low/medium/high) collapses most of the signal; tier counts are kept alongside.

"Total nodes" keeps its existing meaning: `nymnode.total.count`, the number of bonded nym-nodes. No new definition is introduced.

### 6. Snapshot timing and atomicity

After a cycle completes successfully, the monitor writes the hourly snapshot if none exists yet for the current UTC hour. The hourly global row, the hourly per-country rows, the daily per-country and per-gateway upserts and the pruning run in one transaction, so a snapshot is never partial. Daily rows are upserted at every hourly snapshot and freeze after the last one of the UTC day, mirroring `summary_history`'s "refreshed until midnight". If every cycle in an hour fails, that hour has no row: gaps are honest and graphable.

### 7. Whitelisted windows and HTTP caching

`days` accepts only `{1, 7, 30, 90}` on hourly routes and `{30, 90, 365}` on daily routes, matching the retention of each tier; other values return 400. This keeps the key space closed: 4 keys for global hourly, ~1000 for per-country routes, ~3 per gateway. Each history route gets a keyed `moka` cache sized to its key space (the existing caches have capacity 1 and cannot hold parameterised responses), and responses carry `Cache-Control: public, max-age=300` (one monitor interval) so anything in front of the API can serve repeats. Responses are ordered oldest-first, as `/v2/mixnodes/stats` is, because they feed graphs.

## Storage estimate

Measured on PostgreSQL 16 with production column conventions (`SERIAL`/`BIGINT`/`VARCHAR` JSON text as in `summary_history`), a live `/v2/summary` extended with the `network` object (~1.1 KB JSON), 75 countries, ~620 gateways. Sizes include heap, TOAST and indexes; random values were used, which compress worse than real data, so these are upper bounds.

| Table | Rows at steady state | Size |
| --- | --- | --- |
| `summary_history_hourly` (90 d) | 2,160 | ~3 MB |
| `country_stats_hourly` (90 d) | ~162,000 | ~16 MB |
| `country_stats_daily` (365 d) | ~27,000 | ~3 MB |
| `gateway_daily_stats` (365 d) | ~226,000 | ~20 MB |
| `summary_history` (daily, forever) | +365 / year | ~0.5 MB / year |
| **Total, bounded tables** | | **~42 MB, ~50 MB with pruning bloat** |
| **Plus, unbounded** | | **`summary_history`, ~0.5 MB / year** |

A delete-and-refill test settled ~25 % above the one-year size because indexes do not fully shrink; pruning every hour rather than in bulk keeps this lower. The four new tables scale with countries and gateways, not with time; only the existing `summary_history` grows with time, as it does today, now with larger rows. For comparison, the existing unpruned `nym_nodes_packet_stats_raw` grows by ~750 MB/year at 860 nodes (measured 100 MB per million rows with its production schema).

## Risks / Trade-offs

- **Derivation rework**: sharing scoring and the dVPN pipeline with the monitor touches the most consumer-sensitive path. Mitigation: parity tests asserting the directory output is byte-identical before and after.
- **Monitor cycle length**: stats add one pass over ~620 gateways and ~860 nodes already in memory; negligible next to the network fetches.
- **Gaps on failure**: an hour with no successful cycle has no hourly row, and consumers must tolerate gaps.
- **Stale daily rows**: a day whose last cycles fail freezes at its last successful snapshot.

## Migration Plan

1. Work lands on a topic branch in iterations, one capability slice each (`tasks.md` section 5), each slice reviewed and merged into the topic branch on its own; the topic branch merges to `develop` once the slices are complete.
2. Migrations add the history tables slice by slice; no backfill (history accrues from deployment, exactly as it did for `summary_history`).
3. Deploy; `network` is `null` until the first completed cycle, then populated.
4. `load.nymte.ch` and its exporter switch to the new routes; its interim API is retired.

Rollback: drop the routes and stop writing; the tables can stay or be dropped without affecting existing routes.

## Open Questions

1. **Why is `summary_history` daily?** If there is a reason beyond the first implementation (explorer needs, a past storage incident), it may also apply to the hourly tier.
2. **Embedded per-country map (Decision 2)**: a valid alternative if one table is preferred over the indexable layout.
3. **Day-partitioned URLs** (e.g. `/v2/summary/history/hourly/2026-10-06`): completed days never change and could be cached as `immutable` forever by a CDN, but the key space is unbounded, which does not fit the in-process cache model. Worth it only if a CDN fronts the API.
4. **Daily aggregation**: last snapshot of the day (proposed, matches `summary_history`) or the mean of the day's hourly snapshots.
5. **`routing_score` / `config_score`**: both are hardcoded (`0.0` / `0`) and their columns are dead. Request: restore real per-gateway scoring so `config_score` can join the per-gateway daily stats, or remove the fields and columns.
6. **Existing unbounded growth**: `nym_nodes_packet_stats_raw` and `testruns` grow without limit; out of scope here, noted because storage is the constraint this change was sized against.
