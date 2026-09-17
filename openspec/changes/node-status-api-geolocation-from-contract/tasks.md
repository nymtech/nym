# Tasks

## 1. The snapshot type and its holder

- [x] 1.1 Add `nym-geolocation-client`, `nym-contract-anchor` and `arc-swap` to the service's `Cargo.toml`.
- [x] 1.2 Define `GeoSnapshot { height: Height, locations: HashMap<NodeId, payload::Location> }`, carrying the contract payload type itself rather than a shape derived from `geodata::Location`. No local location type: both consumers already convert from the payload, so a third shape would only lose the explicit absence of coordinates.
- [x] 1.3 Add `GeoSnapshotHandle`, a cheaply cloned struct over `Arc<ArcSwap<GeoSnapshot>>` exposing only `load`/`store`, and plumb it from `main.rs` into the monitor beside the existing `NodeGeoCache`. Deliberately additive rather than a type swap on the alias: the swap breaks four use sites (`mod.rs:198`, `mod.rs:300`, `mod.rs:364`, `state.rs:766`) that later groups already own, so `http/server.rs:25`, `http/state.rs:43` and the removal of `NodeGeoCache` land with the consumer cutover in group 7 and every step in between stays compilable. 2.1 then takes the field back off `Monitor`, once the refresh became a worker of its own.

Note: a test that a swap is observed whole was planned here and dropped. `GeoSnapshotHandle`'s methods are one-line delegations, so such a test asserts `arc_swap`'s advertised behaviour rather than any logic of ours. The whole-snapshot guarantee comes from the type choice; the behaviour that is ours, retaining a good snapshot when a refresh fails, is tested in 4.5.

## 2. Give the snapshot its own home

- [x] 2.1 Move `monitor/geolocation.rs` to `src/geolocation/mod.rs` and drop the `geo_snapshot` field from `Monitor` and from both of its entry points, along with the `main.rs` construction 1.3 added. The worker owns the refresh, so the monitor never touches the handle, and nothing else holds one until 5.3 constructs it for the worker and group 7 hands it to the HTTP server. This leaves `monitor/` with no geolocation knowledge at all.

## 3. Height selection

- [ ] 3.1 Add a helper that reads the directory contract's snapshot interval via `get_snapshot_interval()` and computes the greatest multiple of it at or below `tip - lag`. Re-read the interval on every call; do not cache it.
- [ ] 3.2 Pick the settle lag as a named constant with a comment explaining that `H+1` must exist for the proof, and that `interval + lag` is the RPC state-retention requirement.
- [ ] 3.3 Unit-test the grid arithmetic: on-grid and off-grid tips, a tip below one interval, and an interval change between two calls producing heights on the new grid.

## 4. The verified read

- [ ] 4.1 Construct `GeolocationClient::new(ProvenTrustAnchor::new(nyxd, geo_contract, digest_item_key()), nyxd)` over a `clone_query_client()` of the client `main.rs` already builds, so the worker gets its own client without a second connection.
- [ ] 4.2 Implement the refresh: select `H`, call `verified_geolocation(H)`, apply `resolve_all(&DefaultResolutionPolicy)`, build a `GeoSnapshot`, and store it, logging the height and the resolved count.
- [ ] 4.3 Implement the retention rule: on any failure, log at error, leave the held snapshot in place, and never store an empty or partial snapshot. This is what stops a failed read emptying the dVPN directory, so it needs its own test.
- [ ] 4.4 Classify unusable entries via `get_subject`, logging absent at debug, `DecodedLocation::UnsupportedVersion` at warn and `DecodedLocation::Malformed` at error.
- [ ] 4.5 Unit-test 4.3 and 4.4 against a constructed `VerifiedGeolocation`: a failed refresh retains the prior snapshot, and each of the three unusable-entry reasons resolves to no location and is classified distinctly.

## 5. The worker

- [ ] 5.1 Write the loop: refresh once at start-up, then sleep the success interval; on failure sleep a shorter fixed retry delay (5 minutes, named constant) so a cold start is not stuck without a directory for hours.
- [ ] 5.2 Add the success-interval knob, default 6 hours, environment-overridable and hidden from `--help`: it exists for an operator tuning a live deployment, not as part of the documented surface.
- [ ] 5.3 Spawn the worker in `main.rs` under the shutdown manager alongside the scrapers, and not at all in one-shot mode.

## 6. Retire ipinfo from the monitor

- [ ] 6.1 Remove the per-node sweep at `mod.rs:197`, with nothing in its place: the cycle no longer touches geolocation.
- [ ] 6.2 Remove the ipinfo bandwidth check from the head of the cycle (`mod.rs:113`, `check_ipinfo_bandwidth` at `mod.rs:400`), so the cycle's first step becomes building the nym-api client.
- [ ] 6.3 Drop `location` from `ExplorerPrettyBond` and remove the `location_cached` call at `mod.rs:363` that populated it.
- [ ] 6.4 Delete `location_cached` (`mod.rs:296`), `IpInfoClient`, `LocationResponse`, `Location`, `Coordinates` and `Asn` from `monitor/geodata.rs`. Only `ExplorerPrettyBond` survives, at which point `geodata.rs` is a misnomer and the struct should move to where the gateway record is built.

## 7. Wire the consumers

- [ ] 7.1 Point `/explorer/v3/nym-nodes` at the snapshot: plumb the handle through `http/server.rs:25` and `http/state.rs:43` in place of `NodeGeoCache`, load once in `http/api/nym_nodes.rs:44` and hoist it out of the per-node loop at `state.rs:766`, so one response is built against one height. `NodeGeoCache` and the `geocache` name disappear here.
- [ ] 7.2 Source `geoip.ip_address` from the node's first declared host IP, the same value as the top-level `ip_address`, and the empty string when it declares none. It has no on-chain source.
- [ ] 7.3 Resolve the dVPN gateway's location from the snapshot by `node_id` at `state.rs:409` and pass it into `DVpnGateway::new`, rather than deriving it from `explorer_pretty_bond`.
- [ ] 7.4 Remove `Gateway::geo_location` and the location half of `Gateway::location` (`http/models/mod.rs:57-90`), which exist only to read the JSONB copy.
- [ ] 7.5 Render absent coordinates as `0.0` at the dVPN boundary and as the stringified zero in `geoip`, preserving what consumers see today.
- [ ] 7.6 Confirm the country filter at `state.rs:431` is left exactly as it is: a node with no resolved entry yields an empty country code and is dropped, which is the pre-existing outcome and deliberately unchanged.

## 8. Remove the configuration

- [ ] 8.1 Remove `ipinfo_api_token` from `cli/mod.rs:114-115` and its pass-through at `main.rs:85`.
- [ ] 8.2 Remove the `geodata_ttl` knob and any remaining ipinfo references. Note the service's own `Cargo.toml` never declared the `ipinfo` crate; `ipinfo = "3.5.0"` is a workspace entry (`Cargo.toml:328`), so check whether any other member still uses it before removing that line.
- [ ] 8.3 Delete the ipinfo-gated tests at `monitor/geodata.rs:202-238`, including the one that panics when `IPINFO_API_TOKEN` is unset.

## 9. Verify

- [ ] 9.1 `cargo check -p nym-node-status-api` and `cargo test -p nym-node-status-api` pass.
- [ ] 9.2 Grep the service tree for `ipinfo`, `IpInfo`, `geocache` and `geodata_ttl` and confirm nothing survives outside the change's own history.
- [ ] 9.3 `cargo fmt --all`.
- [ ] 9.4 `openspec validate node-status-api-geolocation-from-contract --strict` passes.

## 10. Rollout notes for the PR description

- [ ] 10.1 State the three deployment preconditions: `GEOLOCATION_CONTRACT_ADDRESS` set in the network config, the RPC retaining at least `interval + lag` blocks, and `--ipinfo-api-token` dropped from any command line that passes it.
- [ ] 10.2 State the coverage gate: compare resolved entry count against the described-gateway count before treating the dVPN directory as correct, and treat a large gap as a blocker rather than a curiosity.
- [ ] 10.3 Note the new refresh interval and its default, so an operator knows geolocation now moves on a six-hour clock independent of `monitor_refresh_interval`.
