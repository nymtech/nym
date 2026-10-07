## Why

There is no network-wide, time-resolved view of the gateway fleet. `/v2/summary` serves eight node counts, and `/v2/summary/history` serves 30 daily copies of those counts. Neither carries performance, load, location, residential-IP, family, bridge or version aggregates, and nothing below daily resolution is kept. Everything else the service knows about the fleet is current state, overwritten each monitor cycle.

The gap is real enough that a stopgap has been built outside the service: `load.nymte.ch` scrapes the dVPN directory, the summary and two nym-api endpoints every 5 minutes, recomputes aggregates, keeps its own hourly SQLite history, and re-serves the results as an interim static API. That duplicates ingestion this service already does, and its numbers can drift from what the dVPN directory serves. The node status API is the natural source of truth for this data; the stopgap should become a thin consumer of it.

## What Changes

- `NetworkSummary` gains an optional `network` object with fleet aggregates: dVPN gateway count, locations, mean mixnet performance, mean `performance_v2` score, mean load and the score/load tier distributions, QUIC bridge count, residential-IP gateway count, locations and load, node-family totals split by role, and the build-version distribution. Computed once per monitor cycle with the dVPN directory's own derivations, stored under a new `network.stats` summary key. Additive: existing fields and the eight required keys are unchanged.
- Tiered, materialised history, written after a completed cycle and pruned by retention:
  - global hourly snapshots (whole `NetworkSummary`), kept 90 days;
  - per-country hourly aggregates, kept 90 days;
  - per-country daily aggregates, kept 365 days;
  - per-gateway daily stats, kept 365 days;
  - the existing global daily `summary_history` keeps its current behaviour (never pruned) and now carries `network` too.
- New read routes over bounded, whitelisted windows so every response is cacheable from a small key space:
  - `GET /v2/summary/history/hourly?days={1,7,30,90}`
  - `GET /v2/summary/history/hourly/country/{cc}?days={1,7,30,90}`
  - `GET /v2/summary/history/daily/country/{cc}?days={30,90,365}`
  - `GET /v2/gateways/{identity_key}/history?days={30,90,365}`
- `GET /v2/summary/history` gains an optional `offset` (days back, like `/v2/mixnodes/stats`) so the full daily series is reachable; default behaviour unchanged.
- History routes are served from keyed in-process caches and send `Cache-Control: public, max-age=300`, so proxies, CDNs and browsers can absorb repeat reads.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `node-status-api-monitoring`: a new cycle step computing the network stats, a history-snapshot step after a completed cycle, and the new `network.stats` summary key.
- `node-status-api-persistence`: four new history tables, their ownership and types, and retention extended from two deletions to six.
- `node-status-api-http`: four new routes, `network` on `/v2/summary`, `offset` and cache headers on `/v2/summary/history`, keyed caches for history routes.

## Impact

- **Code**: `nym-node-status-api` only. The per-gateway derivations in `http/models/gw_probe/mod.rs` and the dVPN list build in `http/state.rs` move to a module the monitor can call, so the stats and the directory cannot disagree. New migration, monitor step, queries, handlers, OpenAPI annotations.
- **Storage**: bounded at roughly 50 MB at steady state, measured on PostgreSQL 16 with production column conventions (see `design.md`). Size scales with countries and gateways, not with time.
- **Compatibility**: additive for every existing route. `network` is `null` until the first cycle after deployment; the eight required summary keys and their 500-on-missing behaviour are unchanged.
- **Consumers**: `load.nymte.ch` and its meeting-stats exporter switch to these routes and its interim `/api/v0` is retired. Current per-country tables and the residential-gateway list are derived client-side from `/dvpn/v1/directory/gateways`, which already carries every field needed.
- **Out of scope**: hourly per-gateway history; changing `/v2/summary/history`'s 30-row default; restoring `routing_score`/`config_score` (raised as an open question).
