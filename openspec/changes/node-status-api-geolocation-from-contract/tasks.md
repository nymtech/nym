# Tasks

## 1. The snapshot type and its holder

- [ ] 1.1 Add `nym-geolocation-client`, `nym-contract-anchor` and `arc-swap` to the service's `Cargo.toml`.
- [ ] 1.2 Define `GeoSnapshot { height: Height, locations: HashMap<NodeId, ResolvedLocation> }` and the `ResolvedLocation` payload the two consumers need, deriving it from the contract payload type rather than from `geodata::Location`.
- [ ] 1.3 Replace the `NodeGeoCache` type alias with an `ArcSwap<GeoSnapshot>` handle, keeping the name's role as the thing plumbed into `http/server.rs:25` and `http/state.rs:43` so the wiring change stays mechanical.
- [ ] 1.4 Unit-test that swapping a snapshot is observed whole: a reader resolving several nodes across a concurrent swap sees one height throughout.

## 2. Height selection

- [ ] 2.1 Add a helper that reads the directory contract's snapshot interval via `get_snapshot_interval()` and computes the greatest multiple of it at or below `tip - lag`. Re-read the interval on every call; do not cache it.
- [ ] 2.2 Pick the settle lag as a named constant with a comment explaining that `H+1` must exist for the proof, and that `interval + lag` is the RPC state-retention requirement.
- [ ] 2.3 Unit-test the grid arithmetic: on-grid and off-grid tips, a tip below one interval, and an interval change between two calls producing heights on the new grid.

## 3. The verified read

- [ ] 3.1 Construct `GeolocationClient::new(ProvenTrustAnchor::new(nyxd, geo_contract, digest_item_key()), nyxd)` in the monitor, reusing the existing `QueryHttpRpcNyxdClient` rather than opening a second connection.
- [ ] 3.2 Implement the refresh: select `H`, call `verified_geolocation(H)`, apply `resolve_all(&DefaultResolutionPolicy)`, build a `GeoSnapshot`, and swap it in.
- [ ] 3.3 Implement the retention rule: on any failure, log at error, leave the held snapshot in place, and never swap in an empty or partial snapshot. This is what stops a failed read emptying the dVPN directory, so it needs its own test.
- [ ] 3.4 Classify unusable entries via `get_subject`, logging absent at debug, `DecodedLocation::UnsupportedVersion` at warn and `DecodedLocation::Malformed` at error.
- [ ] 3.5 Unit-test 3.3 and 3.4 against a constructed `VerifiedGeolocation`: a failed refresh retains the prior snapshot, and each of the three unusable-entry reasons resolves to no location and is classified distinctly.

## 4. Wire the monitor cycle

- [ ] 4.1 Replace the per-node sweep at `mod.rs:197` with one call to the refresh, and make its failure non-fatal so the cycle continues to the mixing-assigned-nodes fetch.
- [ ] 4.2 Remove the ipinfo bandwidth check from the head of the cycle (`mod.rs:113`, `check_ipinfo_bandwidth` at `mod.rs:400`), so the cycle's first step becomes building the nym-api client.
- [ ] 4.3 Drop `location` from `ExplorerPrettyBond` and remove the `location_cached` call at `mod.rs:363` that populated it.
- [ ] 4.4 Delete `location_cached` (`mod.rs:296`), `IpInfoClient`, `LocationResponse` and the ipinfo-specific parsing from `monitor/geodata.rs`, keeping only what the new snapshot path still needs.

## 5. Wire the consumers

- [ ] 5.1 Point `/explorer/v3/nym-nodes` at the snapshot: load once in `http/api/nym_nodes.rs:44` and hoist it out of the per-node loop at `state.rs:766`, so one response is built against one height.
- [ ] 5.2 Source `geoip.ip_address` from the node's first declared host IP, the same value as the top-level `ip_address`, and the empty string when it declares none. It has no on-chain source.
- [ ] 5.3 Resolve the dVPN gateway's location from the snapshot by `node_id` at `state.rs:409` and pass it into `DVpnGateway::new`, rather than deriving it from `explorer_pretty_bond`.
- [ ] 5.4 Remove `Gateway::geo_location` and the location half of `Gateway::location` (`http/models/mod.rs:57-90`), which exist only to read the JSONB copy.
- [ ] 5.5 Render absent coordinates as `0.0` at the dVPN boundary and as the stringified zero in `geoip`, preserving what consumers see today.
- [ ] 5.6 Confirm the country filter at `state.rs:431` is left exactly as it is: a node with no resolved entry yields an empty country code and is dropped, which is the pre-existing outcome and deliberately unchanged.

## 6. Remove the configuration

- [ ] 6.1 Remove `ipinfo_api_token` from `cli/mod.rs:114-115` and its pass-through at `main.rs:85`.
- [ ] 6.2 Remove the `geodata_ttl` knob and any remaining ipinfo references, including the `ipinfo` dependency in `Cargo.toml`.
- [ ] 6.3 Delete the ipinfo-gated tests at `monitor/geodata.rs:202-238`, including the one that panics when `IPINFO_API_TOKEN` is unset.

## 7. Verify

- [ ] 7.1 `cargo check -p nym-node-status-api` and `cargo test -p nym-node-status-api` pass.
- [ ] 7.2 Grep the service tree for `ipinfo`, `IpInfo`, `geocache` and `geodata_ttl` and confirm nothing survives outside the change's own history.
- [ ] 7.3 `cargo fmt --all`.
- [ ] 7.4 `openspec validate node-status-api-geolocation-from-contract --strict` passes.

## 8. Rollout notes for the PR description

- [ ] 8.1 State the three deployment preconditions: `GEOLOCATION_CONTRACT_ADDRESS` set in the network config, the RPC retaining at least `interval + lag` blocks, and `--ipinfo-api-token` dropped from any command line that passes it.
- [ ] 8.2 State the coverage gate: compare resolved entry count against the described-gateway count before treating the dVPN directory as correct, and treat a large gap as a blocker rather than a curiosity.
