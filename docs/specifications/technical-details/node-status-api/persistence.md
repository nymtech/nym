# Persistence

This covers the PostgreSQL store: pool setup and migrations, the table inventory and its writers, and the query semantics the HTTP surface and the background workers depend on. The store is the only contract between the workers in [Monitor cycle](monitor-cycle.md) and the read paths in [HTTP surface](http-surface.md), so its uniqueness constraints, refresh patterns and retention behaviour are part of what a reader of this API can rely on. The spine here is `openspec/specs/node-status-api-persistence/spec.md` (revised 2026-08-03).

## Pool and migrations

`Storage::init` (`db/mod.rs`) builds an `sqlx` PostgreSQL pool from `DATABASE_URL` with `sqlx_max_connections` (default 20), `sqlx_min_connections` (default 5) and an acquire timeout of `sqlx_busy_timeout_s` (default 5s), and disables statement logging. When `PG_CERT` is set it switches to `sslmode=require` with that file as the root certificate. It then runs the migrations embedded at compile time from `./migrations_pg`, 18 files as of this writing, unless `SKIP_MIGRATIONS` is set to exactly the string `true`; any other value, including `1` or `TRUE`, still runs them. PostgreSQL is the only supported engine: every query is a compile-time-checked `sqlx` macro against it.

## Table ownership

Most tables have one writer. `gateways` has three, and the split matters at column level rather than table level, because the three writers interleave with no cross-writer coordination:

- the monitor owns `bonded`, `self_described`, `explorer_pretty_bond`, `performance` (whole-table refresh per cycle);
- test-run submission owns `last_probe_result`, `last_probe_log`, `ports_check`, `last_ports_check_utc`, `last_testrun_utc`;
- the packet-stats scraper owns `bridges`;
- `last_updated_utc` is the one column both the monitor and test-run submission write, so it means "either a snapshot or a probe touched this row," not any single event.

| Table | Written by | Read by |
| --- | --- | --- |
| `gateways` | monitor (snapshot); test-run submission (probe result/log, ports check, timestamps); packet-stats scraper (bridges) | gateway, dVPN and services read paths |
| `gateway_description` | description scraper (entry/exit nodes) | `/v2/gateways` join |
| `nym_nodes` | monitor (snapshot) | dVPN and explorer read paths, both scrapers' work queue |
| `nym_node_descriptions` | description scraper | explorer read path |
| `nym_nodes_packet_stats_raw` | packet-stats scraper | daily-delta baseline only |
| `nym_node_daily_mixing_stats` | packet-stats scraper | `/v2/mixnodes/stats` |
| `node_families`, `node_family_members` | monitor (snapshot replace) | dVPN and explorer family enrichment |
| `gateway_session_stats` | metrics scraper | `/v2/metrics/sessions` |
| `summary`, `summary_history` | monitor | `/v2/summary`, `/v2/summary/history` |
| `testruns` | test-run queuer, ports-check scheduler, agent submissions | assignment and completion |
| `ecash_ticketbook`, `distributed_partial_ticketbook`, and the other ecash tables | ticketbook manager | test-run assignment |
| `mixnodes`, `mixnode_description`, `mixnode_daily_stats`, `mixnode_packet_stats_raw` | nothing (legacy, retained) | `mixnodes.historical.count` only |

Each writer uses its own transaction, so a reader can observe a row whose snapshot columns are from one monitor cycle and whose probe columns are from a different, later test run. That interleaving is by design.

## Type choices worth preserving

`nym_nodes.performance` is a `VARCHAR` holding the decimal string form of a percent, parsed back as a `Percent`. `nym_nodes.{ip_addresses,node_role,supported_roles,entry,self_described,bond_info}` and `gateways.{ports_check,bridges}` are `JSONB`, while `gateways.{self_described,explorer_pretty_bond,last_probe_result,last_probe_log}` remain `VARCHAR` holding JSON text. `gateways.self_described` is `NOT NULL`. Timestamps split between `BIGINT` (`gateways`, `summary`, `summary_history`, `testruns`, families) and `INTEGER` (`nym_nodes.last_updated_utc`, `nym_node_descriptions.last_updated_utc`, `nym_nodes_packet_stats_raw.timestamp_utc`), which truncates in 2038. `gateway_session_stats.day` is a real `DATE`. `nym_node_daily_mixing_stats.date_utc` and `summary_history.date` are `VARCHAR` holding `YYYY-MM-DD`.

The scoring columns on `gateways` (`routing_score`, `config_score`, `routing_score_successes/samples`, `config_score_successes/samples`, `test_run_samples`) are dead: nothing writes them, so they hold their schema default of `0`. The helper still named `update_gateway_score` only stamps `last_testrun_utc` and `last_updated_utc` after a submission. This is why the read path reports `routing_score: 0.0` and `config_score: 0` unconditionally (see [HTTP surface](http-surface.md#gateway-and-services-endpoints)).

Uniqueness constraints: `gateways.gateway_identity_key` unique; `nym_nodes.node_id` primary key, with the identity and sphinx-key unique constraints deliberately dropped; `node_family_members.node_id` primary key, which enforces at most one family per node; `gateway_session_stats (node_id, day)` unique; `nym_node_daily_mixing_stats (node_id, date_utc)` unique; `summary.key` primary key; `summary_history.date` unique. Foreign keys from the description, raw-stats and daily-stats tables to `nym_nodes` cascade on delete, though nothing ever deletes a `nym_nodes` row, so the cascade never fires in practice.

## Refresh pattern: null then upsert

Refreshing `nym_nodes` sets `self_described = NULL` and `bond_info = NULL` on every row, then upserts the present nodes, all in one transaction. A node absent from the latest directory keeps its row but disappears from every "described and bonded" read. Refreshing `gateways` sets `bonded = false` on every row, then upserts the present gateways, also in one transaction; it does not clear a departed gateway's `self_described`, `explorer_pretty_bond`, `last_probe_result` or `ports_check`, so those keep serving their last known values alongside `bonded=false`. Node families are refreshed by deleting `node_families` (cascading to members) and re-inserting both tables via two batch inserts in the same transaction, so a reader never sees a partial families snapshot.

## Daily packet deltas

Per scraped node, the batch store inserts the raw counter row before computing the daily delta, and reads the delta baseline as the second-most-recent raw row for that node within the same transaction: the just-inserted row being the most recent is what makes "second row back" mean "the previous scrape." When no baseline exists, the delta is zero. When a counter has decreased since the baseline (a node restart), the current value is used as the delta instead of a negative number. The daily row is upserted on `(node_id, date_utc)`, accumulating the three packet counters and overwriting `total_stake`, which is read from `nym_nodes` with a query that expects exactly one row, so a node missing from `nym_nodes` aborts the whole batch transaction. Raw counter rows are append-only, with no deduplication.

## Session stats

Rows are inserted with `ON CONFLICT DO NOTHING` against `(node_id, day)`, so the first successful write for a node-day is final. Duration arrays and user hashes are stored as JSON text, `NULL` when empty, and parsed back leniently: an unparsable value becomes `null` rather than failing the request. The read query has no `ORDER BY`, so the order of `/v2/metrics/sessions` items, and therefore its pagination, is whatever PostgreSQL returns and is not stable across cache refills.

## Retention

The only rows the service ever deletes at runtime are the `node_families`/`node_family_members` snapshot (replaced each monitor cycle) and `gateway_session_stats` rows whose `day` is at or before one year ago. Every other table grows without bound: `nym_nodes_packet_stats_raw` gains a row per node per hourly scrape, `nym_node_daily_mixing_stats` gains a row per node per day, `testruns` retains every completed run, and `gateways`, `mixnodes` and `nym_nodes` retain rows for nodes that have left the network. `gateways.historical.count` and `mixnodes.historical.count` are literal `count(id)` reads over those unbounded tables.

## Concurrent claims

Test-run assignment claims work with a CTE that selects the oldest queued run of the requested kind joined to a bonded gateway with performance greater than zero, `FOR UPDATE ... SKIP LOCKED`, flipping its status to in-progress and stamping `last_assigned_utc` in the same statement, so concurrent agents never receive the same run. Spending a ticketbook is a single `UPDATE ... SET used_tickets = used_tickets + 1 WHERE id = (SELECT ... ORDER BY expiration_date ASC LIMIT 1 FOR UPDATE)` that returns the claimed book, taking the soonest-expiring usable one. Unlike test-run assignment, it does not use `SKIP LOCKED`, so simultaneous claimants queue on the same candidate row instead of moving on to the next. See [Test-runs](testruns.md) and [Ticketbook issuance and geodata](ticketbook-and-geodata.md).

## Summary reads require the full key set

`get_summary` validates that all eight summary keys are present and returns an internal error if any is missing, rather than a partial object. Counts parse from stored text with a `0` fallback, and each block's `last_updated_utc` renders from the `last_updated_utc` column of its own source row.

## The scraping work queue

`get_nodes_for_scraping` returns only nodes whose `nym_nodes` row has both `self_described` and `bond_info` set, converts each to a skimmed node (skipping and logging any that fail conversion), and classifies a node as an entry/exit nym-node when its identity appears among bonded gateways, otherwise as a mixing nym-node. Descriptions for entry/exit nodes are written to both `nym_node_descriptions` and `gateway_description`; because the latter has a foreign key to `gateways.gateway_identity_key`, the gateway row must already exist for that write to succeed.

## Ports-check normalisation and its migration history

The `gateways.ports_check` JSONB column is served in the canonical four-key form `{ all_pass, error, port_check_target, failed_ports }`, normalising on read both the legacy dedicated shape and an intermediate shape carrying `ports_tested`. A `null` JSON value is treated as absent. This column was historically extracted out of `last_probe_result` by a migration that ran row by row and skipped any row whose stored probe text was not valid JSON, so some gateways legitimately have `ports_check = NULL` while still carrying ports data embedded in their raw probe text.

## Technical notes

- **Implementation**: `db/mod.rs` (`Storage::init`, the pool, `MIGRATOR`), `db/models.rs` (DTOs and insert records, `TestRunStatus`/`TestRunKind`, `NymNodeInsertRecord::new`, `ScraperNodeInfo::contact_addresses`, ports-check normalisation), `db/queries/*` (one module per table area), `migrations_pg/*.sql`.
- **Engine**: PostgreSQL only, via `sqlx` with compile-time-checked macros; migrations embedded via `sqlx::migrate!("./migrations_pg")`.
- **Legacy surface**: the `mixnodes*` tables are no longer written by any worker; they survive only so `mixnodes.historical.count` keeps reporting a number, and `/v2/mixnodes/stats` in fact reads the nym-node daily table despite its name.
