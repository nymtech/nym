# node-status-api-http Specification (delta)

## RENAMED Requirements

- FROM: `### Requirement: Read endpoints SHALL be served from capacity-1 TTL caches that are not populated with empty results`
- TO: `### Requirement: Read endpoints SHALL be served from TTL caches that are not populated with empty results`

## MODIFIED Requirements

### Requirement: The API SHALL serve exactly this route table

The server MUST bind `0.0.0.0:{http_port}` (default 8000) and serve the following routes and no others. All `/v2`, `/explorer/v3` and `/dvpn/v1` routes MUST be unauthenticated; only `/internal/testruns*` requires agent authentication. Nested routers register their root handler such that the bare path (no trailing slash) is served, e.g. `GET /v2/gateways` and `GET /internal/testruns`.

| Method | Path | Success | Success body |
| --- | --- | --- | --- |
| GET | `/` | 308 | empty, `Location: /swagger` |
| GET | `/swagger*` | 200 | Swagger UI assets (HTML/JS/CSS) |
| GET | `/api-docs/openapi.json` | 200 | OpenAPI document |
| GET | `/v2/gateways` | 200 | `PagedResult<Gateway>` |
| GET | `/v2/gateways/skinny` | 200 | `PagedResult<GatewaySkinny>` |
| GET | `/v2/gateways/{identity_key}` | 200 | `Gateway` |
| GET | `/v2/gateways/{identity_key}/history` | 200 | `[GatewayDailyStats]` |
| GET | `/v2/mixnodes/stats` | 200 | `[DailyStats]` |
| GET | `/v2/services` | 200 | `PagedResult<Service>` |
| GET | `/v2/summary` | 200 | `NetworkSummary` |
| GET | `/v2/summary/history` | 200 | `[SummaryHistory]` |
| GET | `/v2/summary/history/hourly` | 200 | `[HourlySummaryHistory]` |
| GET | `/v2/summary/history/hourly/country/{two_letter_country_code}` | 200 | `[CountryStats]` |
| GET | `/v2/summary/history/daily/country/{two_letter_country_code}` | 200 | `[CountryStats]` |
| GET | `/v2/metrics/sessions` | 200 | `PagedResult<SessionStats>` |
| GET | `/v2/status/build_information` | 200 | `BinaryBuildInformationOwned` |
| GET | `/v2/status/health` | 200 | `{ "uptime": <i64 seconds> }` |
| GET | `/explorer/v3/nym-nodes` | 200 | `PagedResult<ExtendedNymNode>` |
| GET | `/explorer/v3/nym-nodes/{node_id}/delegations` | 200 | `[NodeDelegation]` |
| GET | `/dvpn/v1/directory/gateways` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/ips` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/countries` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/country/{two_letter_country_code}` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/entry` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/entry/countries` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/entry/country/{two_letter_country_code}` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/exit` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/exit/countries` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/exit/country/{two_letter_country_code}` | 200 | `[DVpnGateway]` |
| GET | `/internal/testruns` | 200 | `TestrunAssignmentWithTickets` |
| GET | `/internal/testruns/ports-check` | 200 | `TestrunAssignmentWithTickets` |
| POST | `/internal/testruns/{testrun_id}` | 201 | empty |
| POST | `/internal/testruns/{testrun_id}/v2` | 201 | empty |
| POST | `/internal/testruns/{testrun_id}/ports-check/v2` | 201 | empty |

CORS MUST allow any origin, any request headers, methods `POST, GET, PATCH, OPTIONS`, and MUST NOT allow credentials. Every request MUST pass through a logging middleware that only logs (it MUST NOT add or alter response headers) and which requires per-connection peer address info (`ConnectInfo<SocketAddr>`).

#### Scenario: Root redirects permanently to Swagger

- **WHEN** `GET /` is called
- **THEN** the response is 308 Permanent Redirect with `Location: /swagger` and an empty body

#### Scenario: Unknown path

- **WHEN** a path outside the table is requested
- **THEN** axum's fallback returns 404 with an empty body (no JSON error envelope)

#### Scenario: Method not allowed on a known path

- **WHEN** `POST /v2/gateways` is called
- **THEN** the response is 405 Method Not Allowed with an empty body

#### Scenario: Cross-origin read from a browser

- **GIVEN** a browser on any origin
- **WHEN** it issues a preflight and then `GET /v2/summary`
- **THEN** the response carries `access-control-allow-origin: *` and no credentials are allowed

### Requirement: `/v2/summary` SHALL return a complete NetworkSummary or 500

`GET /v2/summary` MUST read the `summary` key-value table directly (it is NOT cached) and MUST return:

```json
{
  "total_nodes": 0,
  "mixnodes": {
    "bonded":     { "count": 0, "self_described": 0, "last_updated_utc": "" },
    "historical": { "count": 0, "last_updated_utc": "" }
  },
  "gateways": {
    "bonded":     { "count": 0, "entry": 0, "exit": 0, "last_updated_utc": "" },
    "historical": { "count": 0, "last_updated_utc": "" }
  },
  "network": {
    "gateways": 0,
    "locations": 0,
    "performance_mean": 0.0,
    "performance_v2_score_mean": 0.0,
    "load_mean": 0.0,
    "performance_tiers": { "high": 0, "medium": 0, "low": 0, "offline": 0 },
    "load_tiers": { "low": 0, "medium": 0, "high": 0, "offline": 0 },
    "quic_bridges": 0,
    "residential": { "gateways": 0, "locations": 0, "load_mean": 0.0 },
    "families": { "active": 0, "nodes": 0, "gateways": 0, "mixnodes": 0 },
    "build_versions": { "1.36.0": 0 },
    "last_updated_utc": ""
  }
}
```

All eight keys `nymnode.total.count`, `assigned.mixing.count`, `nymnode.described.count`, `gateways.bonded.count`, `assigned.entry.count`, `assigned.exit.count`, `mixnodes.historical.count`, `gateways.historical.count` MUST be present; if any is missing the endpoint MUST return 500 rather than a partial summary. Counts MUST be parsed from the stored text as `i32`, falling back to `0` on a parse failure. Each `last_updated_utc` MUST be the RFC3339 rendering of the source row's own `last_updated_utc`: the mixnode `bonded` block takes it from `assigned.mixing.count`, the gateway `bonded` block from `gateways.bonded.count`, each `historical` block from its own key, and `network` from `network.stats`. `network` MUST be `null` when the `network.stats` key is absent or does not deserialise; a mean field inside it MUST be `null` when it was computed over an empty set. Field definitions are fixed by [[node-status-api-monitoring]].

#### Scenario: Missing summary key

- **GIVEN** the `summary` table lacks `assigned.exit.count`
- **WHEN** `GET /v2/summary` is called
- **THEN** the response is 500 with the body `Internal server error`

#### Scenario: Unparsable count

- **GIVEN** a summary row whose `value_json` is not an integer
- **WHEN** the summary is served
- **THEN** the corresponding count is `0` and the request still succeeds

#### Scenario: Before the first stats write

- **GIVEN** all eight count keys present and no `network.stats` key
- **WHEN** `GET /v2/summary` is called
- **THEN** the response is 200 with `"network": null`

### Requirement: `/v2/summary/history` SHALL return the 30 most recent daily snapshots newest-first

`GET /v2/summary/history` MUST return a bare array of `{ "date": "YYYY-MM-DD", "value_json": <any JSON>, "timestamp_utc": "<RFC3339>" }`, ordered by `date` descending (lexicographic order over the stored `YYYY-MM-DD` text) and limited to 30 rows. `value_json` MUST be the stored daily snapshot parsed back into JSON - in practice a whole `NetworkSummary` object as written by the monitor, one row per UTC day refreshed until midnight - or `null` when the stored text does not parse. The optional `offset` query parameter MUST skip that many of the most recent days before taking the 30 rows, so the full daily series is reachable; it defaults to `0`, which preserves the existing response, and a negative value MUST be rejected with 400 and the body `Offset must be non-negative`. A query failure MUST yield `[]` with status 200. Successful responses MUST carry `Cache-Control: public, max-age=300`.

#### Scenario: Newest first

- **GIVEN** 45 stored history rows
- **WHEN** the endpoint is called
- **THEN** 30 rows are returned with the most recent date first

#### Scenario: Offset reaches older days

- **GIVEN** 45 stored history rows
- **WHEN** the endpoint is called with `?offset=30`
- **THEN** the 15 oldest rows are returned, newest of them first

### Requirement: Read endpoints SHALL be served from TTL caches that are not populated with empty results

The gateway list, dVPN gateway list, dVPN gateway IPs, nym-nodes list, mixnode stats, summary history and session stats MUST each be served from their own `moka` cache with max capacity 1 and time-to-live `nym_http_cache_ttl` (default 30s); the summary-history cache MUST instead be keyed by `offset`. The four history routes added by this change (hourly summary history, hourly and daily country history, gateway history) MUST each be served from their own `moka` cache keyed by the full set of path and query parameters, with time-to-live `nym_http_cache_ttl` and a max capacity at least the size of that route's key space (number of allowed `days` values times the number of countries or gateways). On a miss the value MUST be rebuilt from the database, stored (except as noted) and returned. An empty gateway list, empty dVPN list and empty nym-nodes list MUST NOT be cached, so an empty database causes a rebuild on every request. `GET /v2/summary` MUST bypass caching entirely. Failures during a rebuild MUST behave asymmetrically: a failed read of the `gateways` table MUST panic the handler task (`Cannot read gateways table`), and inside the dVPN rebuild a failed read of `nym_nodes`, the node-families tables or the bond-info column MUST likewise panic; the explorer nym-nodes aggregation MUST instead return 500; and mixnode stats, summary history, the four history routes and session stats MUST degrade to empty lists (which are then cached for the TTL).

#### Scenario: Cache hit within TTL

- **GIVEN** a cached gateway list within its TTL
- **WHEN** `GET /v2/gateways` is called
- **THEN** the cached list is paginated and returned without a database read

#### Scenario: Empty list is not cached

- **GIVEN** an empty gateways table
- **WHEN** `GET /v2/gateways` is called twice
- **THEN** both calls query the database and both return `total: 0`

#### Scenario: dVPN list rebuild cost

- **GIVEN** an expired dVPN cache
- **WHEN** any `/dvpn/v1/directory/gateways*` route is called
- **THEN** the whole list is rebuilt (gateways, nym-nodes, families, bond info, SOCKS5 percentiles) before the route-specific filter is applied

#### Scenario: History windows cached independently

- **GIVEN** a cached response for `/v2/summary/history/hourly?days=30` within its TTL
- **WHEN** `/v2/summary/history/hourly?days=7` is called
- **THEN** it is rebuilt from the database and cached under its own key, and the `days=30` entry is unaffected

## ADDED Requirements

### Requirement: History routes SHALL accept only whitelisted windows and send cache headers

The `days` query parameter MUST accept only `1`, `7`, `30` or `90` on the hourly routes (`/v2/summary/history/hourly`, `/v2/summary/history/hourly/country/{two_letter_country_code}`) and only `30`, `90` or `365` on the daily routes (`/v2/summary/history/daily/country/{two_letter_country_code}`, `/v2/gateways/{identity_key}/history`). It MUST default to `30` on hourly routes and `365` on daily routes. Any other value MUST be rejected with 400 and a plain-text body listing the allowed values, e.g. `days must be one of 1, 7, 30, 90`. A window MUST select rows whose timestamp is within that many days of the request time, and responses MUST be ordered oldest-first. Successful responses on all four routes MUST carry `Cache-Control: public, max-age=300`.

#### Scenario: Window outside the whitelist

- **WHEN** `GET /v2/summary/history/hourly?days=13` is called
- **THEN** the response is 400 with the body `days must be one of 1, 7, 30, 90`

#### Scenario: Default window

- **WHEN** `GET /v2/summary/history/hourly` is called without `days`
- **THEN** the rows of the last 30 days are returned, oldest first, with `Cache-Control: public, max-age=300`

### Requirement: `/v2/summary/history/hourly` SHALL return hourly NetworkSummary snapshots

`GET /v2/summary/history/hourly` MUST return a bare array of `{ "hour": "YYYY-MM-DDTHH", "value_json": <any JSON>, "timestamp_utc": "<RFC3339>" }` from `summary_history_hourly`, one element per stored hour in the window. `value_json` MUST be the stored `NetworkSummary`, including `network`, parsed back into JSON, or `null` when the stored text does not parse. Hours with no stored row MUST be absent rather than filled.

#### Scenario: Gap is not filled

- **GIVEN** no stored row for one hour inside the window
- **WHEN** the endpoint is called
- **THEN** that hour is absent from the array and the neighbouring hours are present

### Requirement: Country history routes SHALL return per-country aggregates from the materialised tables

`GET /v2/summary/history/hourly/country/{two_letter_country_code}` MUST read `country_stats_hourly` and `GET /v2/summary/history/daily/country/{two_letter_country_code}` MUST read `country_stats_daily`. Both MUST return a bare array of `CountryStats`: `{ "timestamp_utc": "<RFC3339>" }` on the hourly route or `{ "date": "YYYY-MM-DD" }` on the daily route, plus `gateways`, `performance_mean`, `performance_v2_score_mean`, `load_mean` and `residential_gateways`, with `null` for a mean stored as `NULL`. The path segment MUST be parsed into a recognised country exactly as the dVPN country routes do; an unrecognised value MUST be rejected with 400. A recognised country with no stored rows MUST yield `[]` with status 200.

#### Scenario: Recognised country without gateways

- **GIVEN** a valid country code that has never had a gateway
- **WHEN** its hourly history is requested
- **THEN** the response is 200 with `[]`

#### Scenario: Unrecognised country

- **WHEN** `GET /v2/summary/history/daily/country/XX` is called
- **THEN** the response is 400

### Requirement: `/v2/gateways/{identity_key}/history` SHALL return a gateway's daily stats

`GET /v2/gateways/{identity_key}/history` MUST resolve the identity key to a node id through `nym_nodes` and return a bare array of `{ "date": "YYYY-MM-DD", "performance": <0..1>, "performance_v2_score": <0..1 or null>, "load": <0..1 or null> }` from `gateway_daily_stats`. An identity key that matches no node MUST be rejected exactly as `GET /v2/gateways/{identity_key}` rejects an unknown key: 400 with the key echoed as the plain-text body. A known gateway with no stored rows MUST yield `[]` with status 200.

#### Scenario: Unknown gateway echoes the key

- **WHEN** `GET /v2/gateways/NOT_A_REAL_KEY/history` is called
- **THEN** the response is 400 with `Content-Type: text/plain; charset=utf-8` and the body `NOT_A_REAL_KEY`

#### Scenario: Gateway that left the network

- **GIVEN** a gateway that stopped appearing in the dVPN list 100 days ago
- **WHEN** its history is requested with `?days=365`
- **THEN** its daily rows from before it left are returned until they age past the daily retention window
