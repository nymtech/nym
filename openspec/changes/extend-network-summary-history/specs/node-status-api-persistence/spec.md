# node-status-api-persistence Specification (delta)

## RENAMED Requirements

- FROM: `### Requirement: Retention SHALL be limited to two deletions, leaving the remaining tables unbounded`
- TO: `### Requirement: Retention SHALL be limited to these deletions, leaving the remaining tables unbounded`

## MODIFIED Requirements

### Requirement: Each table SHALL have documented writers, column ownership, and these type choices

The store MUST consist of these tables with these writers. Most tables have a single writer; `gateways` has three, and their ownership MUST be understood at column level rather than table level, because they interleave without any cross-writer coordination:

- the monitor owns `bonded`, `self_described`, `explorer_pretty_bond`, `performance` (whole-table refresh per cycle);
- test-run submission owns `last_probe_result`, `last_probe_log`, `ports_check`, `last_ports_check_utc`, `last_testrun_utc`;
- the packet-stats scraper owns `bridges`;
- `last_updated_utc` is the one shared column, written by both the monitor refresh and test-run submission, so it means "either snapshot or probe touched this row" rather than any single event.

Each writer MUST use its own transaction, so a reader can observe a row whose snapshot columns are from one cycle and whose probe columns are from a different test run. That interleaving is by design and MUST NOT be assumed atomic.

| Table | Written by | Read by |
| --- | --- | --- |
| `gateways` | monitor (snapshot), test-run submission (probe result/log, ports check, test-run timestamps), packet-stats scraper (bridges) | gateway + dVPN + services read paths |
| `gateway_description` | description scraper (entry/exit nodes) | `/v2/gateways` join |
| `nym_nodes` | monitor (snapshot) | dVPN + explorer read paths, both scrapers' work queue |
| `nym_node_descriptions` | description scraper | explorer read path |
| `nym_nodes_packet_stats_raw` | packet-stats scraper | daily-delta baseline only |
| `nym_node_daily_mixing_stats` | packet-stats scraper | `/v2/mixnodes/stats` |
| `node_families`, `node_family_members` | monitor (snapshot replace) | dVPN + explorer family enrichment |
| `gateway_session_stats` | metrics scraper | `/v2/metrics/sessions` |
| `summary`, `summary_history` | monitor | `/v2/summary`, `/v2/summary/history` |
| `summary_history_hourly` | monitor (history snapshot) | `/v2/summary/history/hourly` |
| `country_stats_hourly` | monitor (history snapshot) | `/v2/summary/history/hourly/country/{cc}` |
| `country_stats_daily` | monitor (history snapshot) | `/v2/summary/history/daily/country/{cc}` |
| `gateway_daily_stats` | monitor (history snapshot) | `/v2/gateways/{identity_key}/history` |
| `testruns` | test-run queuer, ports-check scheduler, agent submissions | assignment + completion |
| `ecash_ticketbook`, `distributed_partial_ticketbook`, and the other ecash tables | ticketbook manager | test-run assignment |
| `mixnodes`, `mixnode_description`, `mixnode_daily_stats`, `mixnode_packet_stats_raw` | nothing (legacy, retained) | `mixnodes.historical.count` only |

The following type choices MUST be preserved because reads and writes depend on them: `nym_nodes.performance` is a `VARCHAR` holding the decimal string form of a percent (parsed back as a `Percent`); `nym_nodes.{ip_addresses,node_role,supported_roles,entry,self_described,bond_info}` and `gateways.{ports_check,bridges}` are `JSONB` while `gateways.{self_described,explorer_pretty_bond,last_probe_result,last_probe_log}` remain `VARCHAR` holding JSON **text**; `gateways.self_described` is `NOT NULL`; timestamps are split between `BIGINT` (`gateways`, `summary`, `summary_history`, `summary_history_hourly`, `country_stats_hourly`, `testruns`, families) and `INTEGER` (`nym_nodes.last_updated_utc`, `nym_node_descriptions.last_updated_utc`, `nym_nodes_packet_stats_raw.timestamp_utc`), which truncates in 2038; `gateway_session_stats.day` is a real `DATE`; `nym_node_daily_mixing_stats.date_utc`, `summary_history.date`, `country_stats_daily.date_utc` and `gateway_daily_stats.date_utc` are `VARCHAR` holding `YYYY-MM-DD`; `summary_history_hourly.hour` is `VARCHAR` holding `YYYY-MM-DDTHH`; stats values in the history tables are `REAL` (`NULL` for a mean over an empty set) and counts are `INTEGER`.

The scoring columns on `gateways` (`routing_score`, `config_score`, `routing_score_successes/samples`, `config_score_successes/samples`, `test_run_samples`) MUST be treated as dead: nothing writes them any more, so they hold their schema default of `0`, and the helper still named `update_gateway_score` only stamps `last_testrun_utc` and `last_updated_utc` after a submission. This is why the read path reports `routing_score: 0.0` and `config_score: 0` unconditionally.

Uniqueness MUST be: `gateways.gateway_identity_key` unique; `nym_nodes.node_id` primary key with the identity/sphinx-key unique constraints deliberately dropped; `node_family_members.node_id` **primary key**, which enforces that a node belongs to at most one family; `gateway_session_stats (node_id, day)` unique; `nym_node_daily_mixing_stats (node_id, date_utc)` unique; `summary.key` primary key; `summary_history.date` unique; `summary_history_hourly.hour` unique; `country_stats_hourly (cc, timestamp_utc)`, `country_stats_daily (cc, date_utc)` and `gateway_daily_stats (node_id, date_utc)` primary keys. Foreign keys from the description, raw-stats and daily-stats tables (including `gateway_daily_stats`) to `nym_nodes` MUST cascade on delete, though nothing ever deletes a `nym_nodes` row so the cascade never fires in practice.

#### Scenario: A node cannot join two families
- **GIVEN** a families snapshot listing the same node in two families
- **WHEN** the members batch is inserted
- **THEN** the insert violates the `node_id` primary key and the whole snapshot transaction aborts

#### Scenario: Performance survives a round trip
- **GIVEN** a node whose performance is written as its decimal string form
- **WHEN** the row is read back into a skimmed node
- **THEN** the string parses back into the same percent, and an unparsable value makes that node's conversion fail and be skipped

#### Scenario: One hourly row per hour
- **GIVEN** an existing `summary_history_hourly` row for an hour
- **WHEN** a second insert for the same hour is attempted
- **THEN** it violates the unique `hour` constraint and the history transaction rolls back

### Requirement: Retention SHALL be limited to these deletions, leaving the remaining tables unbounded

The only rows the service ever deletes at runtime MUST be:

- the `node_families`/`node_family_members` snapshot (replaced each monitor cycle);
- `gateway_session_stats` rows whose `day` is at or before one year ago;
- `summary_history_hourly` and `country_stats_hourly` rows older than `history_hourly_retention_days` (default 90);
- `country_stats_daily` and `gateway_daily_stats` rows older than `history_daily_retention_days` (default 365).

History pruning MUST run inside the history-snapshot transaction, so it happens at most once per hour and never leaves a window partly deleted. Every other table MUST grow without bound: `nym_nodes_packet_stats_raw` gains a row per node per hourly scrape, `nym_node_daily_mixing_stats` gains a row per node per day, `summary_history` gains a row per day, `testruns` retains every completed run, and `gateways`, `mixnodes` and `nym_nodes` retain rows for nodes that have left the network. The all-time size of `gateways` and `mixnodes` MUST remain observable through the `gateways.historical.count` and `mixnodes.historical.count` summary keys, which are literal `count(id)` reads over those tables.

#### Scenario: Old raw stats are never pruned
- **GIVEN** a year of hourly packet-stats scrapes
- **WHEN** the service continues running
- **THEN** every raw counter row is still present, since only the listed tables are ever pruned

#### Scenario: Sessions pruned at one year
- **GIVEN** session rows older than one year
- **WHEN** a metrics-scraper cycle finishes storing
- **THEN** those rows are deleted

#### Scenario: Hourly history pruned at the retention window
- **GIVEN** `history_hourly_retention_days` of 90 and hourly rows older than 90 days
- **WHEN** the next history snapshot is written
- **THEN** those rows are deleted in the same transaction and rows within 90 days are kept

#### Scenario: Daily summary history is never pruned
- **GIVEN** `summary_history` rows older than every retention window
- **WHEN** history pruning runs
- **THEN** every `summary_history` row is still present

### Requirement: Summary reads SHALL require the full set of summary keys

`get_summary` MUST validate that all eight summary count keys are present and MUST return an internal-error result if any is missing, rather than a partial `NetworkSummary`. Counts MUST be parsed from the stored text with a `0` fallback, and each block's `last_updated_utc` MUST be rendered from the `last_updated_utc` column of its own source row. The `network.stats` key MUST be read when present and deserialised into the `network` object; when it is absent or fails to deserialise, `network` MUST be `null` and the read MUST still succeed.

#### Scenario: Missing summary key
- **GIVEN** the `summary` table is missing one required key
- **WHEN** `get_summary` runs
- **THEN** it returns an internal error and logs which keys were missing

#### Scenario: Stats key not yet written
- **GIVEN** all eight count keys present and no `network.stats` key
- **WHEN** `get_summary` runs
- **THEN** it returns a complete `NetworkSummary` with `network: null`
