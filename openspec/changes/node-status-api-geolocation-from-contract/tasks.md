# Tasks

## 1. The snapshot type and its holder

- [x] 1.1 Add `nym-geolocation-client`, `nym-contract-anchor` and `arc-swap` to the service's `Cargo.toml`.
- [x] 1.2 Define `GeoSnapshot { height: Height, locations: HashMap<NodeId, payload::Location> }`, carrying the contract payload type itself rather than a shape derived from `geodata::Location`. No local location type: both consumers already convert from the payload, so a third shape would only lose the explicit absence of coordinates.
- [x] 1.3 Add `GeoSnapshotHandle`, a cheaply cloned struct over `Arc<ArcSwap<GeoSnapshot>>` exposing only `load`/`store`, and plumb it from `main.rs` into the monitor beside the existing `NodeGeoCache`. Deliberately additive rather than a type swap on the alias: the swap breaks four use sites (`mod.rs:198`, `mod.rs:300`, `mod.rs:364`, `state.rs:766`) that later groups already own, so `http/server.rs:25`, `http/state.rs:43` and the removal of `NodeGeoCache` land with the consumer cutover in group 7 and every step in between stays compilable. 2.1 then takes the field back off `Monitor`, once the refresh became a worker of its own.

Note: a test that a swap is observed whole was planned here and dropped. `GeoSnapshotHandle`'s methods are one-line delegations, so such a test asserts `arc_swap`'s advertised behaviour rather than any logic of ours. The whole-snapshot guarantee comes from the type choice; the behaviour that is ours, retaining a good snapshot when a refresh fails, is tested in 4.5.

## 2. Give the snapshot its own home

- [x] 2.1 Move `monitor/geolocation.rs` to `src/geolocation/mod.rs` and drop the `geo_snapshot` field from `Monitor` and from both of its entry points, along with the `main.rs` construction 1.3 added. The worker owns the refresh, so the monitor never touches the handle, and nothing else holds one until 5.3 constructs it for the worker and group 7 hands it to the HTTP server. This leaves `monitor/` with no geolocation knowledge at all.

## 3. Height selection

- [x] 3.1 Add a helper that reads the directory contract's snapshot interval via `get_snapshot_interval()` and computes the greatest multiple of it at or below `tip - lag`. Re-read the interval on every call; do not cache it. It lives in a new `src/directory/` module rather than under `geolocation/`: the interval is the directory contract's, and the grid it describes is what a later directory read pins to as well. Split in two, a pure `cadence_height(tip, interval)` and an async `select_cadence_height(client)` that reads both inputs, so the arithmetic is testable without a chain.
- [x] 3.2 Pick the settle lag as a named constant with a comment explaining that `H+1` must exist for the proof, and that `interval + lag` is the RPC state-retention requirement. `SETTLE_LAG = 2`: one block for the proof, one to absorb an RPC answering from a block behind, and no more than that because each one raises the retention requirement.
- [x] 3.3 Unit-test the grid arithmetic: that every selected height is on the grid, settled behind the lag and the newest such height; that a boundary becomes readable exactly once the lag has passed; and that a chain shorter than one interval, a tip that would underflow the lag, and an interval of zero each yield no height. The planned fourth case, an interval change between two calls landing on the new grid, was dropped: it is a property of `select_cadence_height` re-reading rather than caching, which the pure function cannot demonstrate and which a mock client would only assert about a two-line function with no branches.

## 4. The verified read

- [x] 4.1 Construct `GeolocationClient::new(ProvenTrustAnchor::new(nyxd, geo_contract, digest_item_key()), nyxd)` over a `clone_query_client()` of the client `main.rs` already builds. The anchor and the reader each take a client by value, so each gets its own clone; all of them share one `reqwest` connection pool, so this is three wrappers over one transport rather than three dials.
- [x] 4.2 Implement the refresh: call `verified_geolocation(H)`, apply `resolve_all(&DefaultResolutionPolicy)`, build a `GeoSnapshot`, and store it, logging the height and the resolved count against the subject count. `H` is a **parameter** rather than something the refresh selects: the cadence grid is shared across every attested contract, so when the directory read lands one selection should drive both reads at one height. Selecting inside each reader would make that two selections to reconcile.
- [x] 4.3 Implement the retention rule: publish last, so every failure leaves the held snapshot untouched, and refuse an empty result **only when it would replace a non-empty snapshot**. An empty result over an empty snapshot is published with a warning instead, because a network whose geolocator has not written anything yet is not a fault and refusing it would report one forever while changing nothing a reader sees. This is what stops a failed read emptying the dVPN directory, so it needs its own test.
- [x] 4.4 Classify unusable entries per subject, logging `DecodedLocation::UnsupportedVersion` at warn and `DecodedLocation::Malformed` at error, with the most severe reason across that subject's own slots rather than the first found. A subject with no entries at all is not visible here, since it is absent from the contract's set entirely; that case is logged at debug where the node ids are known, at the consumer's lookup in 7.3.
- [x] 4.5 Unit-test 4.3 and 4.4. For 4.3, against the extracted `publish`: an empty result is refused over held locations and the held snapshot keeps its entries and its height, an empty result over an empty snapshot succeeds, and a non-empty result replaces rather than merges. For 4.4, against `unusable_reason`: a malformed payload outranks an unreadable version in an earlier slot, an unreadable version is reported when nothing is worse, and all-readable entries yield no reason. Everything constructed from public API, with no test-only feature on the client crate. Two planned assertions were dropped: that each unusable reason resolves to no location, which is `DefaultResolutionPolicy`'s behaviour and is tested in `nym-geolocation-client`'s own `policy.rs`, and that a chain failure retains the snapshot, which is structural (`publish` is the last statement) and would need a fake client behind three query traits to assert that an early `?` does not reach a later line.

## 5. The worker

- [x] 5.1 Write the loop as `NodeDataRefreshWorker` in `src/node_data.rs`, not under `geolocation/`: one tick selects one cadence height and refreshes every on-chain snapshot at it, which is what makes those reads joinable rather than merely recent, and a directory read joins the same tick later. Named for the data rather than the contract, since `src/directory/` already means the directory contract here. An `IntervalStream` whose first tick fires immediately gives the start-up refresh for free, and the loop selects on the shutdown token, so the two branches are shutdown or tick and nothing else. No failure-retry delay: a failed refresh waits for the next tick, because the held snapshot keeps being served and a sooner retry at this cadence would arrive barely ahead of it.
- [x] 5.2 Add the refresh-interval knob, default 30 minutes, environment-overridable and hidden from `--help`: it exists for an operator tuning a live deployment, not as part of the documented surface.
- [x] 5.3 Spawn the worker in `main.rs` under the shutdown manager alongside the scrapers, and not at all in one-shot mode, which returns before the spawn. Built from a borrow of the nyxd client before the monitor takes ownership of it, so both read the chain through one connection pool.

## 6. Retire ipinfo from the monitor

- [x] 6.1 Remove the per-node sweep at `mod.rs:197`, with nothing in its place: the cycle no longer touches geolocation.
- [x] 6.2 Remove the ipinfo bandwidth check from the head of the cycle (`mod.rs:113`, `check_ipinfo_bandwidth` at `mod.rs:400`), so the cycle's first step becomes building the nym-api client. Took `IpInfoClient::check_remaining_bandwidth` and the `ipinfo` response module with it, since the check was their only caller.
- [x] 6.3 Drop `location` from the `ExplorerPrettyBond` the monitor **writes**, and remove the `location_cached` call that populated it. The served shape keeps its `location`: see 7.7, which composes it per response.
- [x] 6.4 Delete `location_cached`, `IpInfoClient`, `LocationResponse`, `Location`, `Coordinates` and `Asn`, which empties `monitor/geodata.rs` entirely, so the file is deleted and the write-side `ExplorerPrettyBond` moves into `monitor/mod.rs` beside the gateway record it is built for. This also removes the ipinfo-gated tests that lived in that file, which 8.3 had listed separately.

## 7. Wire the consumers

- [x] 7.1 Point `/explorer/v3/nym-nodes` at the snapshot: plumb the handle through `http/server.rs` and `http/state.rs` in place of `NodeGeoCache`, and hoist one `load` out of the per-node loop, so one response is built against one height. `NodeGeoCache` and the `geocache` name disappear here.
- [x] 7.2 Source `geoip.ip_address` from the node's first declared host IP, the same value as the top-level `ip_address`, and the empty string when it declares none. It has no on-chain source. Done in a `NodeGeoData::new` constructor beside the other payload conversions rather than inline in the aggregation loop.
- [x] 7.3 Resolve the dVPN gateway's location from the snapshot by `node_id` and pass it into `DVpnGateway::new`, rather than deriving it from `explorer_pretty_bond`. A node the snapshot has nothing for is logged at warn naming the height and the fact that it is dropped, which is where the "no entry at all" case from 4.4 is reported.
- [x] 7.4 Remove `Gateway::geo_location` and `Gateway::location`, which exist only to read the JSONB copy. The `explorer_pretty_bond` parse they performed is not preserved: it could only fire alongside `bonded=false`, which step (1) of the pipeline already drops, so keeping a parse whose result is discarded would have been cargo-culting. The pipeline's step (5) now checks `self_described` only.
- [x] 7.5 Render absent coordinates as `0.0` at the dVPN boundary and as the stringified zero in `geoip`, preserving what consumers see today.
- [x] 7.6 Confirm the country filter is left exactly as it is: a node with no resolved entry yields an empty country code and is dropped, which is the pre-existing outcome and deliberately unchanged. `http::models::Location` gains a `Default` impl, which is that empty value.
- [x] 7.7 Keep `explorer_pretty_bond.location` on `/v2/gateways`, `/v2/gateways/{identity_key}` and `/v2/gateways/skinny`, which the nym-wallet and explorer-v2 both read, by composing it per response instead of reading it from the row. Needs three pieces: a typed `ExplorerPrettyBond` in place of the opaque `serde_json::Value`, with `location` marked `skip_deserializing` so a pre-migration row's stale location is ignored rather than failing the parse; a `NodeIndex` of identity to node id, published by the monitor each cycle, because the gateways table carries no node id; and `AppState::attach_locations`, called by the three handlers so a location is never stale by the response cache's TTL on top of the refresh interval.

## 8. Remove the configuration

- [x] 8.1 Remove `ipinfo_api_token` from `cli/mod.rs` and its pass-through in `main.rs`, and drop `IPINFO_API_TOKEN` from `.env.example`.
- [x] 8.2 Remove the `geodata_ttl` knob and any remaining ipinfo references, including `NODE_STATUS_API_GEODATA_TTL` in `.env.example`. The workspace `ipinfo = "3.5.0"` entry (`Cargo.toml:328`) **stays**: `nym-geolocator` uses it, which is the service that writes the contract this change now reads.
- [x] 8.3 Delete the ipinfo-gated tests at `monitor/geodata.rs:202-238`, including the one that panics when `IPINFO_API_TOKEN` is unset. Done by 6.4, which deleted the file they lived in.

## 9. Verify

- [x] 9.1 `cargo check -p nym-node-status-api` and `cargo test -p nym-node-status-api` pass: 0 errors, 132 tests. Also compared the three changed surfaces against the live mainnet instance, which caught two regressions in `explorer_pretty_bond.location` that compiled and tested clean: a vanished `ip_address` and a `null` where prod always serves an object. Both fixed and pinned by `the_served_bond_keeps_the_key_set_mainnet_serves`.
- [x] 9.2 Grep the service tree for `ipinfo`, `IpInfo`, `geocache` and `geodata_ttl` and confirm nothing survives outside the change's own history. Only `NodeGeoData` matches, on the substring: that is the type behind the public `geoip` field and is deliberately untouched. The workspace `ipinfo` entry stays for `nym-geolocator` (see 8.2).
- [x] 9.3 `cargo fmt --all`.
- [x] 9.4 `openspec validate node-status-api-geolocation-from-contract --strict` passes.

## 10. Rollout notes for the PR description

- [x] 10.1 State the two deployment preconditions: the RPC retaining at least `interval + lag` blocks (102 at the current defaults), and `--ipinfo-api-token` dropped from any command line that passes it. The geolocation and directory contract addresses are not preconditions: both ship in the network defaults for mainnet, sandbox and canary.
- [x] 10.2 State the coverage gate: compare the resolved entry count in the refresh log line against the live dVPN gateway count before treating the directory as correct, and treat a large gap as a blocker rather than a curiosity. Mainnet served 614 dVPN gateways with zero empty country codes on 2026-09-18, so that is the number the contract's coverage has to approach.
- [x] 10.3 Note the new refresh interval and its default, so an operator knows geolocation now moves on a 30 minute clock independent of `monitor_refresh_interval`.
