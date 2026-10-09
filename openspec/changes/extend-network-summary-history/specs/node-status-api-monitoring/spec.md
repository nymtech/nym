# node-status-api-monitoring Specification (delta)

## REMOVED Requirements

### Requirement: The monitor SHALL execute one strictly ordered cycle per iteration and retry the whole cycle on failure
**Reason**: Its step list still contains the ipinfo allowance check and the per-node geodata refresh, which the geolocation-contract migration removed from the cycle (location now comes from the geolocation snapshot), and it has no stats or history steps.
**Migration**: Replaced by "The monitor SHALL execute one strictly ordered cycle per iteration, retry the whole cycle on failure, and derive stats only from a completed cycle", which carries the current step order plus the stats and history steps.

## MODIFIED Requirements

### Requirement: The summary keys SHALL carry these exact meanings

Each cycle MUST upsert the eight summary keys with a single shared timestamp, sourced as follows:

| Key | Source |
| --- | --- |
| `nymnode.total.count` | number of nodes in the basic-nodes (bonded) response |
| `assigned.mixing.count` | number of active mixing-assigned nodes |
| `nymnode.described.count` | number of described nodes |
| `gateways.bonded.count` | number of **described nodes declaring entry or exit-IPR**, regardless of bonding |
| `assigned.entry.count` | basic nodes whose role is `EntryGateway` |
| `assigned.exit.count` | basic nodes whose role is `ExitGateway` |
| `mixnodes.historical.count` | `count(id)` over the whole `mixnodes` table |
| `gateways.historical.count` | `count(id)` over the whole `gateways` table |

`gateways.bonded.count` MUST keep its historical name despite counting described-role gateways rather than bonded ones. The two `historical` counts MUST be all-time row counts of never-pruned tables, so they only ever grow.

The same upsert MUST also write a ninth key, `network.stats`, whose value is the JSON-serialised network stats computed in step 13, with the same shared timestamp. Unlike the eight count keys it is optional for readers: its absence MUST NOT make the summary incomplete.

The same cycle MUST also upsert one `summary_history` row per UTC date (refreshed on every cycle until midnight, then frozen) whose `value_json` is the serialized `NetworkSummary`, including its `network` object. In that stored object the `last_updated_utc` fields MUST be **unix-seconds strings** (e.g. `"1751894748"`), whereas the same fields on `GET /v2/summary` are rendered as RFC3339: the two representations of the same data legitimately differ and both MUST be preserved.

#### Scenario: Historical counts never decrease
- **GIVEN** gateways that have left the network in the past
- **WHEN** the summary is written
- **THEN** `gateways.historical.count` still counts their rows

#### Scenario: History snapshot timestamp format differs from the live summary
- **WHEN** a cycle writes the summary and its history row
- **THEN** `GET /v2/summary` shows `last_updated_utc` as RFC3339 while the matching `GET /v2/summary/history` entry carries it as a unix-seconds string inside `value_json`

#### Scenario: Daily history carries network stats
- **WHEN** a completed cycle writes the summary-history row
- **THEN** its `value_json` contains the same `network` object that `GET /v2/summary` serves

## ADDED Requirements

### Requirement: The monitor SHALL execute one strictly ordered cycle per iteration, retry the whole cycle on failure, and derive stats only from a completed cycle

The monitor loop MUST call one cycle, sleep the failure-retry delay (60s, fixed) when the cycle returns an error, and sleep `monitor_refresh_interval` (default 300s) when it succeeds. Any step that returns an error MUST abort the remaining steps, and the next attempt MUST restart the cycle from the beginning - there is no resume point and no per-step retry.

Crucially, a cycle MUST NOT be assumed atomic: each write is its own transaction and is committed as it happens, so an abort partway leaves the writes already made in place while everything later stays at its previous value. A failure between the node-families write and the gateway write, for example, leaves fresh nym-nodes and families alongside a stale gateway snapshot, stale delegations and a stale summary, and the API serves exactly that mixture until a later cycle completes. Any replacement wanting whole-snapshot atomicity MUST introduce it explicitly.

The cycle MUST perform these steps in this order:

1. build a nym-api client (see the following requirement);
2. fetch all described nodes v2, keyed by node id (abort on failure);
3. classify described nodes declaring `entry` or `exit_ipr` as gateways;
4. fetch all bonded nym-nodes (contract bond info) and all basic nodes with metadata (abort on failure);
5. write the nym-nodes snapshot;
6. fetch and write the node-families snapshot;
7. **stop here when running in one-shot mode** (`run_once`);
8. fetch the active mixing-assigned node set;
9. compute the summary counts;
10. build and write the gateway snapshot;
11. refresh per-node delegations from `nyxd`;
12. read the historical gateway/mixnode counts;
13. compute the network stats from the store as written by steps 5, 6 and 10 and from one load of the geolocation snapshot;
14. write the summary keys (including `network.stats`) and the summary-history row;
15. write the history snapshot when one is due (see the history-snapshot requirement).

The cycle MUST NOT geolocate nodes itself: location comes from the geolocation snapshot, which a separate worker refreshes from the geolocation contract and publishes whole. Steps 13-15 MUST run only when every earlier step succeeded, so stats and history are never derived from a cycle that aborted partway.

#### Scenario: Early failure writes nothing
- **GIVEN** the described-nodes fetch fails
- **WHEN** the cycle runs
- **THEN** no nym-nodes, gateways, delegations, summary or history rows are written, the loop sleeps 60s, and the previous snapshot stays readable on the HTTP API

#### Scenario: Mid-cycle failure leaves a mixed snapshot
- **GIVEN** a cycle that has already written nym-nodes and node families
- **WHEN** the mixing-assigned-nodes fetch then fails
- **THEN** those two writes remain committed while the gateways table, delegations cache, summary and history keep their previous values, and the API serves that mixture until a later cycle succeeds

#### Scenario: One-shot mode writes only nodes and families
- **WHEN** the monitor is run in one-shot mode (the `ScrapeNode` subcommand with `RUN_ONCE_INIT_NODES` set)
- **THEN** it writes the nym-nodes and node-families snapshots and returns before gateways, delegations, summaries, stats and history are touched

### Requirement: Network stats SHALL be derived with the dVPN directory's own per-gateway derivations

Step 13 MUST compute each field over the population named for it below, with every location and ASN kind taken from a single load of the geolocation snapshot so all fields describe one contract height. The **directory list** is the dVPN gateway list built by the same pipeline that serves `/dvpn/v1/directory/gateways` (filter, enrich and sort), with the default minimum-version filter (`1.6.2`, unparsable versions dropped) applied exactly as the default directory routes apply it; gateway-derived fields MUST use it, together with the same functions for the weighted `performance_v2` score, the `performance_v2.score` tier and the `performance_v2.load` tier. A second implementation of any of these derivations MUST NOT exist. The node-wide fields `families` and `build_versions` MUST NOT apply the gateway filter, so outdated and non-gateway nodes stay visible there. The stats object MUST contain:

| Field | Population | Definition |
| --- | --- | --- |
| `gateways` | directory list | number of gateways in the list |
| `locations` | directory list | number of distinct `two_letter_iso_country_code` values in the list |
| `performance_mean` | directory list | mean mixnet performance as a 0..1 float (the value served as `performance_v2.uptime_percentage_last_24_hours`) |
| `performance_v2_score_mean` | directory list | mean of the numeric weighted score before tier bucketing, over gateways with a parsed probe result |
| `load_mean` | directory list | mean of `1 - ping_ips_performance_v4` over gateways whose probe has a WireGuard result |
| `performance_tiers` | directory list | count of gateways per `performance_v2.score` value |
| `load_tiers` | directory list | count of gateways per `performance_v2.load` value |
| `quic_bridges` | directory list | number of gateways with at least one `bridges.transports[].transport_type` starting with `quic` |
| `residential` | directory list | `{ gateways, locations, load_mean }` computed as above over gateways whose `location.asn.kind` is `residential` |
| `families` | every node in the `node_families` / `node_family_members` snapshot, any role or version | `{ active, nodes, gateways, mixnodes }`: families with at least one member, member count, and members split by declared role (entry or exit-IPR counts as gateway, otherwise mixnode-declaring counts as mixnode) |
| `build_versions` | every described nym-node, any role or version | map of `build_information.build_version` to the number of nodes reporting it |

A mean over an empty set MUST be `null`, never `0`.

#### Scenario: Stats match the directory
- **GIVEN** a completed cycle
- **WHEN** a client recomputes `gateways`, `locations` and `performance_tiers` from `GET /dvpn/v1/directory/gateways` without a `min_node_version` parameter, against the same cached list
- **THEN** the values equal the `network` object served by `GET /v2/summary`

#### Scenario: Outdated gateway is excluded
- **GIVEN** a gateway reporting build version `1.5.0` or an unparsable version
- **WHEN** the stats are computed
- **THEN** it is counted in none of the directory-list fields, just as it is absent from `GET /dvpn/v1/directory/gateways`, while its version is still counted in `build_versions` and it still counts in `families` if it is a family member

#### Scenario: No residential gateways
- **GIVEN** no gateway whose ASN kind is `residential`
- **WHEN** the stats are computed
- **THEN** `residential.gateways` is `0` and `residential.load_mean` is `null`

### Requirement: The monitor SHALL write tiered history snapshots after a completed cycle

When step 15 runs, the monitor MUST check whether a `summary_history_hourly` row exists for the current UTC hour (the timestamp truncated to the hour). If none exists it MUST, in a single transaction:

1. insert the hourly global row whose `value_json` is the serialized `NetworkSummary` including `network`;
2. insert one `country_stats_hourly` row per country present in the dVPN gateway list, with that country's `gateways`, `performance_mean`, `performance_v2_score_mean`, `load_mean` and residential gateway count, computed as in the network-stats requirement;
3. upsert one `country_stats_daily` row per such country for the current UTC date with the same values;
4. upsert one `gateway_daily_stats` row per gateway in the list for the current UTC date with its mixnet performance, numeric `performance_v2` score and `1 - ping_ips_performance_v4` load;
5. delete rows older than the retention windows: hourly tables older than `history_hourly_retention_days` (default 90), daily per-country and per-gateway tables older than `history_daily_retention_days` (default 365).

When a row for the current hour already exists, step 15 MUST write nothing. While the geolocation snapshot is still the cold-start value (height `0`, nothing read yet), steps 13-15 MUST NOT run: the directory list is empty then, and recording it would store an all-zero hour indistinguishable from a real outage. A failure anywhere in the transaction MUST roll back all five writes and MUST NOT fail the cycle; the next cycle retries.

#### Scenario: One snapshot per hour
- **GIVEN** `monitor_refresh_interval` of 300s and a run of successful cycles
- **WHEN** twelve cycles complete within one UTC hour
- **THEN** exactly one hourly global row and one set of hourly per-country rows exist for that hour

#### Scenario: Failed hour leaves a gap
- **GIVEN** every cycle within a UTC hour fails before step 15
- **WHEN** the next hour's first cycle completes
- **THEN** the failed hour has no hourly rows and the new hour's snapshot is written normally

#### Scenario: Daily rows freeze at the last snapshot of the day
- **GIVEN** hourly snapshots written at every hour of a UTC day
- **WHEN** the day ends
- **THEN** that day's `country_stats_daily` and `gateway_daily_stats` rows hold the values of the day's last snapshot and are not changed afterwards

#### Scenario: No stats or history before the first geolocation read
- **GIVEN** a freshly started service whose geolocation snapshot has not been refreshed yet
- **WHEN** a cycle completes
- **THEN** no `network.stats` key and no history rows are written for it, and the first cycle after the snapshot is published writes them normally

#### Scenario: Snapshot failure does not fail the cycle
- **GIVEN** the history transaction fails on a constraint violation
- **WHEN** step 15 runs
- **THEN** no history row from that attempt is visible, the cycle still reports success, and the next cycle attempts the snapshot again
