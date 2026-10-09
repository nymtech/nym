# HTTP surface

This covers the axum HTTP server: the route table, the response contract, the read caches, and the dVPN directory pipeline. The internal `/internal/testruns*` routes are specified in [Test-runs](testruns.md). The spine here is `openspec/specs/node-status-api-http/spec.md` (revised 2026-08-03).

## Route table

The server binds `0.0.0.0:{http_port}` (default 8000, `NYM_NODE_STATUS_API_HTTP_PORT`) and serves exactly this table. All `/v2`, `/explorer/v3` and `/dvpn/v1` routes are unauthenticated; only `/internal/testruns*` requires agent authentication.

| Method | Path | Success | Success body |
| --- | --- | --- | --- |
| GET | `/` | 308 | empty, redirects to `/swagger` |
| GET | `/swagger*` | 200 | Swagger UI assets |
| GET | `/api-docs/openapi.json` | 200 | OpenAPI document |
| GET | `/v2/gateways` | 200 | `PagedResult<Gateway>` |
| GET | `/v2/gateways/skinny` | 200 | `PagedResult<GatewaySkinny>` |
| GET | `/v2/gateways/{identity_key}` | 200 | `Gateway` |
| GET | `/v2/mixnodes/stats` | 200 | `[DailyStats]` |
| GET | `/v2/services` | 200 | `PagedResult<Service>` |
| GET | `/v2/summary` | 200 | `NetworkSummary` |
| GET | `/v2/summary/history` | 200 | `[SummaryHistory]` |
| GET | `/v2/metrics/sessions` | 200 | `PagedResult<SessionStats>` |
| GET | `/v2/status/build_information` | 200 | build info |
| GET | `/v2/status/health` | 200 | `{ "uptime": <seconds> }` |
| GET | `/explorer/v3/nym-nodes` | 200 | `PagedResult<ExtendedNymNode>` |
| GET | `/explorer/v3/nym-nodes/{node_id}/delegations` | 200 | `[NodeDelegation]` |
| GET | `/dvpn/v1/directory/gateways` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/ips` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/countries` | 200 | `[String]` |
| GET | `/dvpn/v1/directory/gateways/country/{cc}` | 200 | `[DVpnGateway]` |
| GET | `/dvpn/v1/directory/gateways/entry(/countries\|/country/{cc})?` | 200 | see above |
| GET | `/dvpn/v1/directory/gateways/exit(/countries\|/country/{cc})?` | 200 | see above |
| GET, POST | `/internal/testruns*` | see [Test-runs](testruns.md) | |

CORS allows any origin, any request headers, methods `POST, GET, PATCH, OPTIONS`, and does not allow credentials. An unknown path gets axum's fallback 404 with an empty body. A method not allowed on a known path gets 405 with an empty body.

## Response contract

Every data endpoint answers `Content-Type: application/json` with a bare `serde_json` body: no envelope, no `data` key. Field names are the Rust field names as written. An optional field that is absent is still emitted, as JSON `null`, never omitted. Numeric conventions: integer types as JSON numbers, `Decimal` and `Uint128` as strings, `IpAddr` as a string, `time::Date` as `YYYY-MM-DD`, unix timestamps as RFC3339 UTC with second precision, and ed25519/x25519 keys as base58 strings (except inside agent request payloads, see [Test-runs](testruns.md)).

Errors are `(status, plain-text message)`, never a JSON envelope:

| Condition | Status | Body |
| --- | --- | --- |
| invalid input | 400 | the offending value or a specific message, echoed verbatim |
| unregistered agent key, bad signature, or stale request | 401 | `Make sure your public key is registered with NS API` |
| no delegation data for a node | 404 | `No delegation data for node_id={node_id}` |
| internal failure | 500 | `Internal server error` |
| no test run to assign | 503 | `No testruns available` |

The 503 body must stay exactly `No testruns available`: the agent client matches on that substring to tell "nothing to do" apart from a real failure. Axum's own extractor rejections are not remapped (400 for bad path or query parameters or malformed JSON, 415 for a missing or wrong content type, 422 for well-typed JSON that does not match the target, 413 over the body limit), with one exception: `POST /internal/testruns/{id}` (v1) captures the JSON rejection itself and re-emits every variant as 400.

## Paged endpoints

`/v2/gateways`, `/v2/gateways/skinny`, `/v2/services`, `/v2/metrics/sessions` and `/explorer/v3/nym-nodes` return `{ "page", "size", "total", "items" }`, where `total` is the length of the filtered, pre-pagination list. `size` defaults to 10 and is silently clamped to 200; `page` defaults to 0. When `page * size` exceeds `total`, the served page snaps back to `total / size` (integer division) and reports that recomputed value. A non-numeric `size` or `page` is rejected with 400.

Whole-list consumers pay for this design twice over: a client that omits `size` walks a large gateway list ten items at a time, and the 200 cap means even a deliberate caller needs several round trips. The dVPN directory routes are the one exception: they are unpaginated and return the whole array.

## Caching

The gateway list, dVPN gateway list, dVPN gateway IPs, nym-nodes list, mixnode stats, summary history and session stats each sit behind their own `moka` cache, capacity 1, TTL `nym_http_cache_ttl` (default 30s, `NYM_NODE_STATUS_API_NYM_HTTP_CACHE_TTL`). A miss rebuilds from the database and stores the result, except that an empty gateway list, empty dVPN list or empty nym-nodes list is never cached, so an empty database rebuilds on every request. `GET /v2/summary` bypasses caching entirely.

Failures during a rebuild are handled asymmetrically. A failed read of the `gateways` table, or of `nym_nodes`, the node-families tables or bond info inside the dVPN rebuild, panics the handler task: these are treated as unrecoverable. The explorer nym-nodes aggregation instead returns 500 on the same kind of failure. Mixnode stats, summary history and session stats degrade to an empty list, which is then cached for the TTL like any other value.

## Gateway and services endpoints

`GET /v2/gateways` and `GET /v2/gateways/{identity_key}` return the stored gateway row close to verbatim, with two exceptions that are worth naming because they look like real measurements and are not. `routing_score` is always `0.0` and `config_score` is always `0`: both are hardcoded in the DTO conversion, and the underlying columns are dead (see [Persistence](persistence.md)). Missing description columns are served as the literal string `NA`. Stored JSON blobs (`self_described`, `explorer_pretty_bond`, `last_probe_result`) are parsed and re-serialized rather than passed through byte for byte, so object keys come out alphabetically sorted and malformed stored JSON collapses silently to `null`. `GET /v2/gateways/skinny` returns only bonded gateways with a narrower field set. `GET /v2/gateways/{identity_key}` matches by exact string equality and returns 400, not 404, on a miss.

`GET /v2/services` derives one candidate entry per gateway from its `self_described` JSON via JSONPath (`ip_address`, `hostname`, `service_provider_client_id`), reports a constant `routing_score` of `1.0`, and by default keeps only entries with a network requester (`entry=true` widens the initial keep, `wss=true` and `hostname=true` narrow it further).

## Summary, mixnode stats and sessions

`GET /v2/summary` reads the `summary` key-value table directly (uncached) and requires all eight keys the monitor writes (see [Monitor cycle](monitor-cycle.md#summary-keys)); a missing key is 500, not a partial object. `GET /v2/summary/history` returns up to 30 daily snapshots, newest first; a query failure degrades to `[]` with 200. `GET /v2/mixnodes/stats` returns up to 30 ascending daily rows with an optional `offset` (in days, not rows) and rejects a negative offset with 400; a query failure also degrades to `[]` with 200. `GET /v2/metrics/sessions` pages over cached session stats with optional `day` and `node_id` filters applied before pagination; the underlying query has no `ORDER BY`, so item order, and therefore pagination, is not stable across cache refills.

## The explorer nym-nodes endpoint

`GET /explorer/v3/nym-nodes` returns only nodes with a stored self-description, enriched with nym-api performance, stake, geodata and family membership. `geoip` and `family_data` are `null` when the node is absent from the relevant cache. `GET /explorer/v3/nym-nodes/{node_id}/delegations` returns 404 with `No delegation data for node_id={id}` only when the node has no cache entry at all (unbonded, or its chain query failed); a bonded node with zero delegators returns 200 with `[]`.

## The dVPN directory pipeline

The dVPN gateway list is built from the cached gateway list by, in order:

1. drop `bonded=false` gateways;
2. drop gateways with `performance == 0`;
3. drop gateways with no matching row in the nym-nodes table;
4. attach family, staking and SOCKS5 percentile data;
5. drop gateways whose `explorer_pretty_bond` or `self_described` JSON is missing or unparsable, which is how a missing location or a missing `build_information` removes a node;
6. drop gateways whose country code is not exactly two characters;
7. sort by `(country_code, identity_key)`.

Each surviving item carries identity, name, description, IP packet router and authenticator addresses, location, the last probe outcome, IP addresses, role, entry details, bridges, a fixed-two-decimal `performance` string, a computed `performance_v2` score, Lewes protocol details, family and staking data, build information, and the ports-check summary.

The minimum node version defaults to `1.6.2` and is applied at read time by parsing `build_information.build_version` as semver; a version that fails to parse drops the gateway. Only the root `/dvpn/v1/directory/gateways` route accepts a `min_node_version` override; every other route in the family ignores it and uses the default, because the filter runs at read time against an unfiltered cached list rather than against a per-parameter cache.

`/entry` keeps gateways whose last probe reports `as_entry.can_route == true`. `/exit` keeps gateways whose last probe reports both `as_exit.can_route_ip_external_v4` and `as_exit.can_route_ip_external_v6`. `/countries` and its entry/exit variants return two-letter codes with adjacent duplicates removed, which is exhaustive only because the source list is already sorted by country. `/ips` returns the sorted, deduplicated IP set of the default-minimum-version list, cached independently of any version parameter.

### The probe outcome is an external contract

`last_probe.outcome` carries exactly `as_entry`, `as_exit`, `wg`, `socks5`, `lp`. This shape is parsed by the VPN API on the other side, so its field names and score encodings cannot change without coordinating with that consumer, whatever the scoring logic itself looks like. Two quirks matter for a reader: `as_entry` and `as_exit` are untagged, so "not tested" and "a failed test" both serialize to `null` and are indistinguishable on the wire; and `socks5` is always present, deriving `can_proxy_https` from a percentile score when the stored probe carries no dedicated SOCKS5 section.

Displayed scores are computed with fixed thresholds: `performance_v2.score` weights mixnet performance 40%, download speed 30% and IPv4 ping performance 30%, and buckets the weighted result into `offline`/`low`/`medium`/`high`. `performance_v2.mixnet_score` and `performance_v2.load` are derived independently with their own thresholds, and `load` is forced to `offline` whenever `score` is `offline`. The SOCKS5 percentile score is computed across the whole gateway set from HTTPS latency: zero or missing latency is `offline`, and the remainder is bucketed by rank into `high`/`medium`/`low`.

## OpenAPI is descriptive, not normative

`GET /api-docs/openapi.json` is generated from `#[utoipa::path]` annotations and served at `/swagger`. It is not the compatibility contract: `/dvpn/v1/directory/gateways/ips` and every `/internal/testruns*` route carry no annotation and are absent from the document, and the delegations endpoint is annotated with a singular type while the handler returns an array.

## Cross-links

- Performance contract (`contracts/performance`): the nym-api performance score this layer reads and reshapes into `uptime`, `performance` and `performance_v2`. This service does not compute or submit scores.
- Network monitors contract (`contracts/network-monitors`): a separate authorisation registry, unrelated to the probes this service runs.
- Geolocation contract (`contracts/geolocation`): the `geoip` field on `/explorer/v3/nym-nodes` and the country code that gates the dVPN directory both come from this service's own `ipinfo` lookup (see [Ticketbook issuance and geodata](ticketbook-and-geodata.md#geodata)), not from the geolocation contract.

## Technical notes

- **Implementation**: `http/server.rs` (`start_http_api`, also spawns the ports-check scheduler), `http/api/mod.rs` (`RouterBuilder`, CORS, request logging), `http/state.rs` (`AppState`, `HttpCache`, `aggregate_node_info_from_db`, `load_family_lookup`), `http/mod.rs` (`PagedResult`, `Pagination`), `http/models/mod.rs`, `http/models/gw_probe/` (probe reshaping, scoring, `socks5_calc.rs`), `http/error.rs`, `db/models.rs`, `db/queries/summary.rs`.
- **Framework**: axum 0.8, tower-http CORS, utoipa/utoipa-swagger-ui/utoipauto for OpenAPI, `moka` for caching, `serde_json_path` for JSONPath, `celes` for country parsing, `semver` for version comparison.
