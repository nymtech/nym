# Node status API geolocation from the contract

## Why

The node status API geolocates every node itself, by calling a metered third-party API (ipinfo) once per node and holding the answers in an in-memory `moka` cache. That makes it the de facto public answer on where the network is, derived from a source nobody can check, lost on every restart, and billed per lookup. The geolocation contract now holds this data on chain and `common/nym-geolocation-client` can read and cryptographically verify the whole set, so the service can stop being an originator of unverifiable claims and become a consumer of verified ones.

This is the third and last of the three changes the geolocation work was sequenced into. The contract and the geolocator service landed first, the verifying retrieval client landed second, and `docs/geolocation/node-status-api-migration.md` was written specifically to brief this one.

## What Changes

- A dedicated worker reads the whole geolocation record set through `GeolocationClient::verified_geolocation`, backed by a `ProvenTrustAnchor`, and resolves each subject to a single entry with `DefaultResolutionPolicy`. It is deliberately not a step of the monitor cycle: after this change no cycle step consumes geolocation, so that placement would couple a chain read to a sequence of database writes and establish no ordering anything relies on. It refreshes every six hours, retrying five minutes after a failure.
- Reads are pinned to a **cadence height**: the greatest multiple of the directory contract's snapshot interval at or below the chain tip minus a small lag. This is the same height grid every attested contract shares, so a future directory read and a future move to nym-api-served attested snapshots both join at a height this service already pins to.
- The per-node `moka` geodata cache is replaced by a single atomically swapped snapshot. A per-key cache cannot be replaced atomically, so it would let a reader observe entries from two different heights and discard the coherence the digest proof establishes.
- Both consumers read that one snapshot, in memory, at request time. The dVPN directory stops reading location out of the persisted `explorer_pretty_bond` JSONB, and the monitor stops writing it there.
- A failed or empty read never replaces a good snapshot. This is load-bearing rather than defensive: an empty country removes a gateway from the dVPN directory, so swapping in an empty snapshot would empty the entire directory at once.
- Nodes with no usable entry keep today's behaviour exactly, resolving to an empty location and being dropped by the existing country-code filter. What changes is that the outcome is now honest: an empty country means the contract holds nothing, not that a metered API call failed.
- The three distinct reasons an entry can be unusable (absent, unreadable payload version, malformed payload) are logged distinctly, because a payload version rolled out ahead of this build is an alarm rather than a missing-data case.
- **BREAKING**: `IpInfoClient`, the geodata cache, the `geodata_ttl` knob and the required `--ipinfo-api-token` / `IPINFO_API_TOKEN` argument are removed. A deployment passing the flag on the command line fails to start until it is dropped.

## Capabilities

### New Capabilities

None. This change re-sources existing behaviour; it introduces no capability of its own.

### Modified Capabilities

- `node-status-api-monitoring`: the ordered cycle loses its ipinfo quota check and its geodata sweep outright, and gains no geolocation step in their place; the geodata caching requirement is replaced wholesale by a dedicated refresh worker and snapshot semantics; gateway records stop carrying a location in `explorer_pretty_bond`.
- `node-status-api-http`: `/explorer/v3/nym-nodes` sources `geoip` from the snapshot rather than the geodata cache and can no longer populate `ip_address` from it; the dVPN pipeline sources location from the snapshot rather than from the persisted JSON, which changes what a missing `explorer_pretty_bond` implies.

## Impact

**Code.** A new `src/geolocation/` module holding the snapshot, its handle, the height arithmetic and the refresh worker. `src/monitor/{mod,geodata}.rs` loses the `IpInfoClient`, `Location`, `NodeGeoCache` and `location_cached` machinery and ends up with no geolocation knowledge at all. `src/http/{server,state}.rs` (the cache is plumbed through both), `src/http/api/nym_nodes.rs`, `src/cli/mod.rs` and `src/main.rs` (the removed argument, the added interval, the spawned worker).

**Dependencies.** Adds `nym-geolocation-client`, `nym-contract-anchor` and `arc-swap` (already a workspace dependency) to the service. Drops the ipinfo HTTP client. The service already depends on `nym-geolocation-contract-common` with the `payload` feature and already holds a `QueryHttpRpcNyxdClient`, so neither the payload adapters nor chain access needs introducing.

**Deployment.** `GEOLOCATION_CONTRACT_ADDRESS` must be set in the network config, and the chain RPC must retain at least `snapshot_interval + lag` blocks of state, because a proven read at a pruned height cannot be served. Any deployment passing `--ipinfo-api-token` must drop it.

**Data.** One field cannot be reproduced: `geoip.ip_address` has no on-chain source, deliberately, and is sourced from the node's announced addresses or left empty.

**Not in scope.** Reading directory data from the directory contract. That is a later change; this one only adopts the height grid it will use.
