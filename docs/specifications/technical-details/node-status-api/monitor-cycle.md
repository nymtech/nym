# Monitor cycle

This covers the network monitor's timed ingestion cycle and the three scrapers that run alongside it on their own schedules: the description scraper, the packet-stats scraper, and the metrics scraper. Together they are what keeps the database that [HTTP surface](http-surface.md) reads current. The spine here is `openspec/specs/node-status-api-monitoring/spec.md`; every requirement below is verified against `nym-node-status-api/nym-node-status-api/src/monitor/mod.rs` and its siblings.

## The monitor cycle is strictly ordered

`Monitor::run` (`src/monitor/mod.rs`) runs one cycle, sleeps 60 seconds on failure, and sleeps `monitor_refresh_interval` (default 300s, `NODE_STATUS_API_MONITOR_REFRESH_INTERVAL`) on success. A cycle performs these steps in order, and any step's failure aborts every step after it:

1. Check the `ipinfo` bandwidth allowance. A failure here is logged at debug level and does not abort the cycle: this check is advisory only.
2. Build a nym-api client from the network environment (see below).
3. Fetch all described nodes v2, keyed by node id. A failure here aborts the cycle before anything is written.
4. Classify described nodes declaring `entry` or `exit_ipr` as gateways.
5. Fetch all bonded nym-nodes (contract bond info) and all basic nodes with metadata.
6. Write the nym-nodes snapshot.
7. Fetch and write the node-families snapshot.
8. Stop here in one-shot mode (see below).
9. Refresh geodata for every described node.
10. Fetch the active mixing-assigned node set.
11. Compute the summary counts.
12. Build and write the gateway snapshot.
13. Refresh per-node delegations from `nyxd`.
14. Read the historical gateway and mixnode counts.
15. Write the summary keys and the summary-history row.

### A cycle is not atomic

Each of the writes above is its own database transaction, committed as it happens. There is no cycle-level transaction and no resume point: a failure between steps, say after the node-families write but before the gateway write, leaves fresh nym-nodes and families sitting next to a gateway table, a delegations map and a summary that are all still one or more cycles stale. The HTTP API serves exactly that mixture until a later cycle completes in full. The service provides no whole-snapshot atomicity.

### `--nym-api` is accepted but not used

Both the monitor and the metrics scraper build their nym-api client from the first endpoint of `NymNetworkDetails::new_from_env()`, using that endpoint's `api_url`, not from the `--nym-api` / `NYM_API` CLI argument. The process panics if the network environment carries no endpoint or no `api_url`. `--nym-api` is required configuration that the code never reads: pointing it at a different nym-api has no effect on which directory gets scraped.

### One-shot mode

Running the `ScrapeNode { node_id }` subcommand with `RUN_ONCE_INIT_NODES` set runs `monitor::run_once`, which performs steps 1 to 7 above and returns before geodata, gateways, delegations or summaries are touched. It then scrapes the single named node's description and exits. This is a bootstrap path, not the service's normal mode.

## Gateway records

For each described node classified as a gateway, the monitor writes one row keyed by base58 ed25519 identity: `bonded` from presence in the bonded-nym-nodes map, `self_described` as the serialized description (the column is `NOT NULL`), `explorer_pretty_bond` as `{ identity_key, owner, pledge_amount, location }` for bonded nodes and `NULL` otherwise, `last_updated_utc` as the current unix timestamp, and `performance` as the matching skimmed node's performance rounded to an integer percent, or `0` if the identity is absent from the skimmed set.

Gateway classification does not depend on bonding. An unbonded described gateway still gets a row, with `bonded=false`. Because the dVPN directory drops any gateway with `performance == 0` or no `explorer_pretty_bond` (see [HTTP surface](http-surface.md#the-dvpn-directory-pipeline)), an unbonded or unrewarded gateway stays visible on `/v2/gateways` while disappearing from the dVPN routes.

## Nym-node records

For every node in the basic-nodes response, the monitor builds a record with the node id, base58 ed25519 identity, base58 x25519 sphinx key, IP addresses, mix port, role, supported roles, entry flag, performance (as the decimal string form of the percent), total stake (`0` when unbonded), the described-node JSON or `NULL`, the contract bond JSON or `NULL`, and the operator's custom HTTP API port or `NULL`. A record that fails to build is logged as an error and skipped; every other node in the same snapshot is still written.

## Geodata is cached only on success

Node geolocation is looked up in a `moka` cache keyed by node id with TTL `geodata_ttl` (default 86400s, `NODE_STATUS_API_GEODATA_TTL`). On a miss the monitor tries each of the node's declared IP addresses in order against `ipinfo`, caches and returns the first success. Exhausting every IP returns an empty `Location`, which is not cached: a node that cannot be geolocated is retried on every subsequent cycle, burning `ipinfo` quota each time, and its stored `explorer_pretty_bond` carries an empty location. An empty location means an empty two-letter country code, which is enough on its own to drop the gateway from the dVPN directory (see [HTTP surface](http-surface.md#the-dvpn-directory-pipeline)). A described gateway is looked up twice in the same cycle: once in the geodata sweep, once again while its gateway record is built. See [Ticketbook issuance and geodata](ticketbook-and-geodata.md#geodata) for the geodata summary.

## Summary keys

Each cycle upserts eight summary keys under one shared timestamp:

| Key | Source |
| --- | --- |
| `nymnode.total.count` | nodes in the basic-nodes (bonded) response |
| `assigned.mixing.count` | active mixing-assigned nodes |
| `nymnode.described.count` | described nodes |
| `gateways.bonded.count` | described nodes declaring entry or exit-IPR, regardless of bonding |
| `assigned.entry.count` | basic nodes with role `EntryGateway` |
| `assigned.exit.count` | basic nodes with role `ExitGateway` |
| `mixnodes.historical.count` | `count(id)` over the whole `mixnodes` table |
| `gateways.historical.count` | `count(id)` over the whole `gateways` table |

`gateways.bonded.count` keeps its historical name even though it counts described-role gateways, not bonded ones. The two `historical` counts are all-time row counts over tables nothing prunes, so they only ever grow.

The same cycle upserts one `summary_history` row per UTC date, refreshed on every cycle until midnight and then frozen, whose `value_json` is the serialized network summary. Inside that stored object, `last_updated_utc` fields are unix-seconds strings (for example `"1751894748"`), while the live `GET /v2/summary` renders the same fields as RFC3339. Both representations are correct; they are just not the same string.

## Delegations

The monitor queries `nyxd` for delegations once per bonded node, sequentially, and inserts an entry only for nodes whose query succeeded (a node with no delegators gets an empty vector). It then replaces the whole shared delegations map in one write-lock acquisition. Unbonded nodes and nodes whose query failed get no entry at all, which is what makes `/explorer/v3/nym-nodes/{node_id}/delegations` answer 404 for them until a later cycle succeeds.

## The description scraper

Runs every 4 hours, at most 5 concurrent tasks, over the shared scraping query (nodes that are both described and bonded). Per node it tries each candidate contact URL (see below) with `/description` appended, using a 3-second-timeout HTTP client that accepts invalid TLS certificates. The scrapers address nodes by IP, and many nodes serve self-signed certificates. The service stores what a node returns for `/description`, `/stats` and the bridge endpoint, and serves it on the public routes. It takes the first response that deserializes into `{ moniker, website, security_contact, details }`, sanitizes every field with `ammonia` configured to strip all tags, attributes and URL schemes, replaces empty or whitespace-only fields with the literal `N/A`, and replaces an `N/A` moniker with a deterministic three-word name derived from the node id.

The result is upserted into `nym_node_descriptions`, and for entry/exit nodes additionally into `gateway_description` so the `/v2/gateways` join sees it (see [Persistence](persistence.md)). The scraper does not wait for its spawned tasks: a cycle counts as finished once its queue drains, while writes may still be in flight. An unscraped row therefore surfaces as three distinct "missing" markers depending on which read path serves it: the literal `NA` on `/v2/gateways` (the read-side `COALESCE` default) and an empty string on `/explorer/v3/nym-nodes`.

## The packet-stats scraper

Runs every hour over the same node set, at most `packet_stats_max_concurrent_tasks` (default 10) concurrent tasks, and waits for every task before storing. Per node it tries each candidate contact URL with `/stats`, accepting either the legacy mixnode counter names (`packets_*_since_startup`) or the nym-node names (`received_since_startup`, `sent_since_startup`, `dropped_since_startup`), truncating any counter reported as a float. When a URL yields stats, it additionally fetches `/api/v1/bridges/client-params` from that same URL and keeps the parsed bridge information when it deserializes.

Each node reports its own counters over its HTTP API. The scraper applies restart detection (a counter lower than the stored baseline is treated as the delta itself, not a negative number) and no other plausibility check. A non-JSON response body aborts that node's scrape entirely rather than falling through to the remaining candidate URLs. All collected records are written in one transaction: bridge information onto the gateway row, then the raw counter row, then the daily delta, per record. Any single record's failure aborts the whole batch, so one node missing from `nym_nodes` loses every other node's stats for that cycle too.

## Candidate contact URLs

For a node with no custom HTTP API port in its bond, the candidate list is, per declared IP: `http://{ip}:8080`, `http://{ip}:8000`, `https://{ip}`, `http://{ip}`. When the bond declares a custom HTTP API port, the candidate list is replaced entirely by `http://{ip}:{custom_port}` per declared IP, with no fallback to the default ports. A wrong custom port makes both scrapers fail for that node by design: it exists to disambiguate multiple nodes sharing one IP.

## The metrics scraper

Runs every 6 hours (60s retry on failure). Its node set is the union of all bonded nym-nodes (addressed by their bond host and custom HTTP port) and all described nodes that are not legacy mixnodes (addressed by their first declared IP), keyed by node id with the bonded entry taking precedence. It visits nodes strictly sequentially and treats a session-metrics response whose `update_time` is the unix epoch as "no data".

For each retained response it splits sessions by type into VPN, mixnet and unknown duration arrays, serializes each non-empty array (and the unique-user hashes) as JSON text, and stores `NULL` for empty ones. It records `day` as the date of the node's own reported `update_time`, not the scrape time, and inserts with `ON CONFLICT DO NOTHING` against the `(node_id, day)` constraint, so the first successful scrape for a given node-day wins and later scrapes that day are discarded. After inserting it deletes session rows whose `day` is at or before one year before now.

## Technical notes

- **Implementation**: `monitor/mod.rs` (`run_in_background`, `run_once`, `Monitor::run`, `prepare_nym_node_data`, `prepare_gateway_data`, `location_cached`, `historical_count`, `prepare_node_family_data`), `monitor/geodata.rs` (`IpInfoClient`, `Location`, `ExplorerPrettyBond`), `monitor/node_delegations.rs` (`DelegationsCache`, `refresh`), `node_scraper/description.rs`, `node_scraper/scraper.rs`, `node_scraper/helpers.rs` (`scrape_node`, `scrape_and_store_description`, `sanitize_description`, `update_daily_stats_uncommitted`), `metrics_scraper/mod.rs`, `db/models.rs` (`ScraperNodeInfo::contact_addresses`, `NymNodeInsertRecord::new`).
- **External services**: nym-api (described nodes v2, bonded nym-nodes, basic nodes with metadata, mixing-assigned nodes, node families), `nyxd` (per-node delegations), `ipinfo.io` (geolocation and quota), and each node's own HTTP API.
- **Intervals**: monitor 300s (60s retry); description scraper 4h; packet-stats scraper 1h; metrics scraper 6h (60s retry); ports-check scheduler 10 min (see [Test-runs](testruns.md)).
