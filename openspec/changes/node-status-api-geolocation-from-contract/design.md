# Design

## Context

The node status API geolocates nodes itself. `IpInfoClient` (`monitor/geodata.rs:4`) calls a metered third-party API per node, `location_cached` (`monitor/mod.rs:296`) memoises the answers in a `moka` `Cache<NodeId, Location>` with a 24 hour TTL, and the sweep at `mod.rs:197` fills it once per cycle. That cache is not private to the monitor: it is passed to `http/server.rs:25` and `http/state.rs:43`, so the HTTP handlers read it directly.

Two consumers read the result. `/explorer/v3/nym-nodes` reads the cache live (`http/api/nym_nodes.rs:44` into `state.rs:766`). The dVPN directory reads a copy that was serialized into the `explorer_pretty_bond` JSONB at `mod.rs:362-391` and read back at `http/models/mod.rs:59-66`.

Those two are not independent sources. Both ultimately call `location_cached`, so they agree at the moment of writing. What differs is durability: the cache is in memory and the JSONB is on disk. They come apart after a restart, when the cache is empty and the rows survive, and for a node that leaves the described-gateway set, whose row then freezes indefinitely.

The payload-shape work for this migration is already done and is not part of this change. `From<payload::Location> for Location`, `From<payload::Asn> for Asn` and `From<payload::AsnKind> for AsnKind` exist at `http/models/mod.rs:194-250` with tests, and the crate already depends on `nym-geolocation-contract-common` with the `payload` feature. `main.rs` already builds a `QueryHttpRpcNyxdClient`, and `NyxdClient::clone_query_client` hands a second worker its own without opening a second connection, so chain access needs no introduction either. This change is about sourcing, not shapes.

## Goals / Non-Goals

**Goals:**

- Source geolocation from the contract, verified, instead of from a metered third-party API.
- Make what the service serves internally coherent: one height, one snapshot, one source for both consumers.
- Remove ipinfo from the service entirely, including its configuration.
- Read at a height that a later directory-contract read can join without reconciliation.

**Non-Goals:**

- Reading directory data from the directory contract. That is a later change. This one reads the directory contract only to learn the shared cadence interval.
- Consuming producer-attested snapshots over HTTP instead of doing a proven chain read. The retrieval client supports it, the nym-api producer for geolocation does not exist yet, and its transport is a stub.
- Changing what the dVPN directory contains. A node with no entry is dropped exactly as a node with a failed lookup was.
- Changing any response shape. Every field keeps its current type and meaning.

## Decisions

**Verified read rather than a plain smart query.** `GeolocationClient::verified_geolocation(H)` over a `ProvenTrustAnchor` proves the contract's digest at `H` and recomputes the accumulator locally over what was returned, so a lying or buggy RPC fails the read rather than silently changing where the network appears to be. The alternative, calling `get_all_geolocation_records` directly, is simpler and needs no anchor or height selection, but it discards the entire point of the contract work: this service is the de facto public answer on network geography, and it would be asserting that answer on an RPC's word. Rejected for that reason.

**One atomically swapped snapshot rather than a per-key cache.** The new source returns a set that is coherent at one height, and the digest proof is what establishes that. A `moka` cache with per-entry TTLs cannot be replaced atomically: clearing and refilling it lets a reader observe node A from this refresh and node B from the last, which throws the coherence away and keeps none of the guarantee we paid for. `ArcSwap<GeoSnapshot>` replaces the whole value behind one pointer store, so a reader sees either the new snapshot or the previous one. `arc-swap` is already a workspace dependency (`Cargo.toml:243`). A `tokio::sync::watch` would work equally well; `ArcSwap` is preferred only because reads are synchronous, which lets `state.rs:766` hoist one load out of its per-node loop instead of awaiting a get per node.

Note that the other `moka` caches in `HttpCache` (`state.rs:190-210`) are unaffected. Those are capacity-1 TTL caches of computed HTTP responses, a different job.

**Both consumers read the snapshot; location leaves the JSONB.** `explorer_pretty_bond` keeps `identity_key`, `owner` and `pledge_amount` and loses `location`. This removes the only remaining way the two consumers could disagree, and it fixes the frozen-row case as a side effect, since nothing persisted outlives the entry it came from. Readers must tolerate a `location` key still present in rows written before this change, which serde does by default for an ignored field. The alternative, keeping the persisted copy as a fallback when the snapshot has no entry, was rejected: it reintroduces two read-time sources, which is the thing this change exists to stop.

**Cadence heights rather than an arbitrary recent height.** `H` is the greatest multiple of the directory contract's snapshot interval at or below `tip - lag`. The interval lives only in the directory contract (`DEFAULT_SNAPSHOT_INTERVAL = 100`, mutable via `try_update_snapshot_interval`); the geolocation contract deliberately has none, because the cadence is specified as shared across every attested contract so that one height serves all of them.

The immediate argument is not the future directory read, it is that producer-attested snapshots exist **only** at cadence heights. If this service ever drops its own chain RPC in favour of verifying producer-served snapshots offline, it must already be reading on that grid. Reading off-grid now would have to be undone then, and it buys nothing in exchange.

A lag is not optional: a digest proof at `H` verifies against the `app_hash` in the header at `H+1`, so the tip is never readable. The interval must be re-read each refresh rather than cached, because an on-chain change would otherwise silently desynchronise this service from every other consumer of the grid.

**A worker of its own, not a step in the monitor cycle.** The obvious placement is where the geodata sweep is today, step 9 of the ordered cycle, and that was the original plan here. It is wrong, and what makes it wrong is this change itself: once `location` leaves the `explorer_pretty_bond` JSONB, no step of the cycle reads or writes geolocation at all. Both consumers are HTTP handlers reading the snapshot at request time. So the placement establishes no ordering that anything depends on, and all it actually does is couple a chain read to a sequence of database writes: a cycle aborting at the described-nodes fetch stops geolocation refreshing for reasons that have nothing to do with the chain, and a slow proven read delays the gateway, delegation and summary writes. Separating them also makes non-fatality structural rather than a rule to remember, and keeps the worker out of one-shot mode by simply never starting it.

The cost is a second `nyxd` client, since the monitor owns the one `main.rs` builds. `NyxdClient::clone_query_client` makes that a shared query client rather than a second connection, so it is a genuine cost of roughly nothing.

**Refresh on a multi-hour interval, not on the monitor's.** Six hours by default, hidden from `--help` and overridable by environment, with a short fixed retry (5 minutes) after a failure. A node's country changes on the order of days, so the monitor's 300s cadence would buy no freshness anyone can perceive while paying for a whole-set download, an accumulator recompute and an ICS23 proof each time. The retry is separate from the interval because the cold-start case has no snapshot to fall back on: waiting out six hours there would mean six hours with no dVPN directory, where waiting five minutes is a blip.

**A failed refresh retains the previous snapshot.** Load-bearing rather than defensive: an empty country removes a gateway at `state.rs:431`, so publishing an empty snapshot would empty the entire dVPN directory in one step.

**Unusable entries are distinguished by reason in logs, not in the response.** `resolve` returns `None` for three different things, and `get_subject` (`verified.rs:251`) tells them apart. All three produce the same served outcome, but `DecodedLocation::UnsupportedVersion` means a payload version has been rolled out ahead of this build and `DecodedLocation::Malformed` is anomalous, since the contract checks payload size but not content. Collapsing all three into "no location" is exactly the failure the migration note warned against, so they are logged at debug, warn and error respectively.

**The ipinfo argument is removed outright rather than deprecated.** It would serve no purpose once nothing reads it.

## Risks / Trade-offs

**A single failure now costs the whole set, where it used to cost one node.** → The previous snapshot is retained, so a failed read costs freshness rather than coverage. The exception is cold start, which has no previous snapshot; that window is now one chain read rather than a full per-node sweep, it is retried every 5 minutes rather than on the success interval, and it is logged loudly rather than silently serving an empty directory.

**Cadence heights tighten the pruning requirement.** Reading at up to `interval + lag` behind tip is roughly 105 blocks at the default, where an arbitrary recent height would be a handful. A nyx signer RPC has previously been observed retaining only about 100 blocks, which would sit right at that edge. → State it as a deployment requirement rather than discovering it in production: the RPC must retain at least `interval + lag` blocks. The read fails loudly on pruned state rather than falling back to an unproven height.

**Thin contract coverage would shrink the dVPN directory.** If the geolocator has not populated entries for most nodes, dropping nodes with no entry removes them from the directory. → This is a rollout gate rather than a code problem: compare resolved coverage against the described-gateway count before cutting over, and treat a large gap as a blocker. Keeping ipinfo as a transitional fallback was considered and rejected, because it keeps the metered dependency and mixes a verified source with an unverified one under one field.

**Staleness is now bounded by the refresh interval, six hours, rather than by a 24 hour cache TTL.** → Still an improvement on what it replaces, and irrelevant for data that changes on the order of days. The cadence grid adds up to `interval + lag` blocks on top, roughly eight minutes at the default, which is noise beside the six hours.

**A directory-contract query enters a geolocation-only path.** → One query per refresh, four times a day, and it is what the shared cadence is specified to require.

## Migration Plan

1. Set `GEOLOCATION_CONTRACT_ADDRESS` in the deployment's network config. `NymContractsProvider` resolves it from there (`network-defaults/src/network.rs:51`); the read fails with an unavailable-contract error without it.
2. Confirm the chain RPC retains at least `interval + lag` blocks of state.
3. Drop `--ipinfo-api-token` from any deployment passing it on the command line. Deployments setting `IPINFO_API_TOKEN` as an environment variable need no action, since the variable is simply no longer read. **This is the breaking step**: the argument is removed, so passing it prevents start-up.
4. Deploy, then compare resolved coverage against the described-gateway count before treating the dVPN directory as correct.

Rollback is a redeploy of the previous version with the ipinfo token restored. There is no schema migration, but rollback is **not** instantaneous, and the reason is worth knowing in advance rather than discovering it during an incident.

The previous version declares `ExplorerPrettyBond.location` as a non-optional `Location` with no serde default (`monitor/geodata.rs:66-72`). Rows this change writes have no `location` key, so under the rolled-back binary they fail to deserialize, `Gateway::geo_location` returns an error, and dVPN step (5) drops every such gateway. The dVPN directory is therefore empty from the moment of rollback until one monitor cycle rewrites every gateway row with a location, which is up to `monitor_refresh_interval` (default 300s). It then self-heals with no intervention. `/v2/gateways` and `/explorer/v3/nym-nodes` are unaffected, since neither parses that field for location.

This is accepted rather than mitigated: the service is replaced in one step, so there is no staged window in which both versions read the same rows.

## Open Questions

None blocking. The rollout gate in the third risk is a deployment-time check rather than a design question.
