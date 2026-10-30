## MODIFIED Requirements

### Requirement: The monitor SHALL execute one strictly ordered cycle per iteration and retry the whole cycle on failure

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
13. write the summary keys and the summary-history row.

The cycle MUST NOT touch geolocation at all. No step reads or writes it, so no step may depend on it: geolocation is refreshed by its own worker (see "Geolocation SHALL be refreshed by a dedicated worker"), and the two are independent both ways. A cycle that aborts at step 2 MUST NOT prevent a geolocation refresh, and a chain fault MUST NOT delay any write above.

#### Scenario: Early failure writes nothing
- **GIVEN** the described-nodes fetch fails
- **WHEN** the cycle runs
- **THEN** no nym-nodes, gateways, delegations or summary rows are written, the loop sleeps 60s, and the previous snapshot stays readable on the HTTP API

#### Scenario: Mid-cycle failure leaves a mixed snapshot
- **GIVEN** a cycle that has already written nym-nodes and node families
- **WHEN** the mixing-assigned-nodes fetch then fails
- **THEN** those two writes remain committed while the gateways table, delegations cache and summary keep their previous values, and the API serves that mixture until a later cycle succeeds

#### Scenario: A failing cycle still leaves geolocation fresh
- **GIVEN** a cycle that aborts on the described-nodes fetch, repeatedly
- **WHEN** the geolocation worker's interval elapses
- **THEN** it refreshes the snapshot regardless, so the dVPN directory's locations stay current while its gateway rows go stale

#### Scenario: One-shot mode writes only nodes and families
- **WHEN** the monitor is run in one-shot mode (the `ScrapeNode` subcommand with `RUN_ONCE_INIT_NODES` set)
- **THEN** it writes the nym-nodes and node-families snapshots and returns before gateways, delegations and summaries are touched, and no geolocation worker is started

### Requirement: Gateway records SHALL be derived from described nodes with bond-conditional enrichment

For each described node classified as a gateway the monitor MUST write one gateway row keyed by base58 ed25519 identity, containing: `bonded` set from presence in the bonded-nym-nodes map; `self_described` as the serialized description (always present, the column is `NOT NULL`); `explorer_pretty_bond` as `{ identity_key, owner, pledge_amount }` for bonded nodes and `NULL` otherwise; `last_updated_utc` as the current unix timestamp; and `performance` as the matching skimmed node's performance rounded to an integer percent, defaulting to `0` when no skimmed node has that identity. Gateway classification MUST be independent of bonding, so unbonded described gateways are written with `bonded=false`.

`explorer_pretty_bond` MUST NOT carry a location. Location is served from the geolocation snapshot at read time, so it is never frozen into a persisted row and never outlives the entry it came from. Readers MUST tolerate a `location` key in rows written before this change.

Because a gateway with `performance == 0` or without `explorer_pretty_bond` is dropped from the dVPN directory, an unbonded or unrewarded gateway MUST remain visible on `/v2/gateways` while disappearing from the dVPN routes.

#### Scenario: Unbonded gateway retains its row
- **GIVEN** a described node declaring the entry role that is not bonded
- **WHEN** the gateway snapshot is written
- **THEN** its row has `bonded=false` and `explorer_pretty_bond=NULL`, and it is absent from the dVPN directory but present on `/v2/gateways`

#### Scenario: Gateway missing from the skimmed set scores zero
- **GIVEN** a described gateway whose identity is absent from the basic-nodes response
- **WHEN** its record is built
- **THEN** `performance` is written as `0`

#### Scenario: A pre-existing location key is ignored rather than rejected
- **GIVEN** a gateway row written before this change, whose `explorer_pretty_bond` still contains a `location` object
- **WHEN** that row is read
- **THEN** it parses successfully and the stored location is ignored in favour of the geolocation snapshot

## ADDED Requirements

### Requirement: Geolocation SHALL be refreshed by a dedicated worker

Geolocation MUST be refreshed by its own timed worker rather than as a step of the monitor cycle. The worker MUST refresh once at start-up and then on its interval, and it MUST be independent of the monitor in both directions: a failing monitor cycle leaves it refreshing, and a failing or slow refresh delays nothing the monitor writes.

The separation is structural rather than stylistic. After this change no monitor step consumes the snapshot - both readers are HTTP handlers reading it at request time - so placing the refresh inside the ordered cycle would couple a chain read to a sequence of database writes while establishing no ordering that anything relies on.

The refresh interval MUST default to 30 minutes, and MUST be environment-overridable while staying hidden from the command-line help, since it exists for an operator tuning a live deployment rather than as part of the service's documented surface. The data behind it changes on the order of days, and the cadence grid bounds how fresh any single read can be in any case, so the default is chosen to keep the snapshot obviously current rather than to chase the chain.

A failed refresh MUST NOT be retried ahead of the next interval. The held snapshot keeps being served meanwhile, and at this cadence a sooner retry would arrive barely before the next scheduled one.

One-shot mode (`run_once`) MUST NOT start the worker at all.

#### Scenario: Start-up refreshes immediately
- **WHEN** the service starts
- **THEN** the worker attempts a refresh straight away rather than waiting out its interval, because until it succeeds the held snapshot is empty

#### Scenario: A failed refresh waits for the next interval
- **GIVEN** a refresh that fails
- **WHEN** the worker schedules its next attempt
- **THEN** it waits the ordinary interval rather than retrying sooner, logs the failure at error level, and keeps serving the previously held snapshot meanwhile

#### Scenario: The worker outlives monitor failures
- **GIVEN** monitor cycles that keep aborting
- **WHEN** the worker's interval elapses
- **THEN** it refreshes normally, because it shares no step, client or failure path with the cycle

### Requirement: Geolocation SHALL be read from the contract as one verified, height-pinned snapshot

The service MUST obtain node geolocation by reading the geolocation contract, not by querying any third-party geolocation service. Each refresh MUST read the whole record set at a single height through a trust anchor that proves the contract's on-chain digest, MUST verify the retrieved records against that digest by local recompute, and MUST resolve each subject to at most one entry using the retrieval client's default resolution policy. A read that cannot establish a trusted digest, or whose records do not recompute to it, MUST yield no records at all rather than partial ones.

The resolved result MUST be published as a single snapshot value carrying the height it was read at, replaced atomically in whole. It MUST NOT be stored in a per-key cache with independent entry lifetimes, because such a store cannot be replaced atomically and would let a reader observe entries established at two different heights, discarding the coherence the digest proof exists to establish.

A failed refresh MUST leave the previously held snapshot in place, and MUST NOT publish a partial one.

An empty result MUST NOT replace a non-empty snapshot, and MUST be reported as a failure when it would. This is load-bearing rather than defensive: an empty country code removes a gateway from the dVPN directory, so replacing a populated snapshot with an empty one would empty the whole directory in one step. An empty result MAY replace an empty snapshot, because a network whose geolocator has not written anything yet holds nothing to read, and refusing that reading would report a fault where there is none while changing nothing a reader sees. It MUST still be reported at warning level, since it means the dVPN directory is empty.

#### Scenario: The whole set is verified before any of it is published
- **GIVEN** a record set whose locally recomputed accumulator does not equal the proven digest
- **WHEN** the refresh runs
- **THEN** no records are published, the previously held snapshot is retained, and the failure is logged at error level

#### Scenario: Readers never observe a half-replaced snapshot
- **GIVEN** a refresh that is publishing a new snapshot
- **WHEN** a reader resolves several nodes concurrently
- **THEN** every node it resolves comes from one snapshot at one height, either entirely the new one or entirely the previous one

#### Scenario: A failed refresh never empties the directory
- **GIVEN** a held snapshot containing entries and a refresh that fails
- **WHEN** the dVPN directory is rebuilt
- **THEN** it is built from the retained snapshot, because publishing an empty snapshot would remove every gateway from the directory at once

#### Scenario: An empty result is refused only when it would lose entries
- **GIVEN** a verified read that resolves no usable location at all
- **WHEN** the held snapshot already contains entries
- **THEN** the refresh fails, the held entries are retained, and the refusal names how many entries it declined to discard
- **AND WHEN** the held snapshot is empty instead, as on a network whose geolocator has not written anything yet
- **THEN** the empty result is published at that height and reported at warning level, rather than being retried as a fault

### Requirement: Geolocation reads SHALL be pinned to the shared attestation cadence height

The height each refresh reads at MUST be the greatest multiple of the directory contract's snapshot interval that is at or below the current chain tip minus a settle lag. The interval MUST be read from the directory contract on each refresh rather than cached at start-up or hardcoded, because it is mutable on chain and a stale value would desynchronise this service from every other consumer of that height grid.

The lag is required rather than defensive: a digest proof at height `H` verifies against the `app_hash` carried in the header at `H+1`, so the chain tip itself is never a readable height.

Pinning to the cadence grid rather than to an arbitrary recent height is what lets a later directory-contract read, and a later move to consuming producer-attested snapshots, join at a height this service already reads at. Producer-attested snapshots exist only at cadence heights, so reading off-grid would have to be undone to adopt them.

#### Scenario: Height is on the cadence grid
- **GIVEN** a snapshot interval of `D` and a chain tip of `T`
- **WHEN** a refresh selects its height
- **THEN** it selects the greatest multiple of `D` at or below `T - lag`, so two independent readers observing the same tip select the same height

#### Scenario: An interval change is picked up without a restart
- **GIVEN** the directory contract's snapshot interval is updated on chain
- **WHEN** the next refresh runs
- **THEN** it reads the new interval and selects its height on the new grid

#### Scenario: Pruned state fails the read rather than silently falling back
- **GIVEN** a chain RPC retaining fewer than `interval + lag` blocks of state
- **WHEN** a refresh attempts a proven read at the selected cadence height
- **THEN** the read fails and is reported, rather than being retried at an unpinned or unproven height

### Requirement: An unusable geolocation entry SHALL be distinguished by reason

A subject that resolves to no usable location MUST be recorded as having no location, exactly as a node whose geolocation could not be determined was before this change. The three distinct reasons MUST be distinguishable in the service's logs rather than collapsed into one: the subject has no entries at all; the resolved entry carries a payload version this build has no decoder for; or the resolved entry carries a version this build understands whose content did not parse.

Each is reported where it is observable. A node with no entry at all is not visible to the refresh, which only sees what the contract holds, so it MUST be reported at the point of use, where the node is known and where the consequence is: a gateway with no entry is dropped from the dVPN directory, so it MUST be reported at warning level naming that outcome. An entry this build cannot read MUST be reported at warning level as a payload version rolled out ahead of this build, and a malformed payload at error level, because the contract stores content opaquely and checks only its size, so nothing on the write path would have rejected it.

#### Scenario: A node with no entry is reported where it is dropped
- **GIVEN** a bonded gateway with no geolocation entry of any kind
- **WHEN** the dVPN directory is built
- **THEN** its absence is logged at warning level, naming the node and the height read, and saying that the gateway is dropped

#### Scenario: Unreadable payload version is surfaced as a warning
- **WHEN** a node's resolved entry carries a payload version this build cannot decode
- **THEN** it resolves to no location and this is logged at warning level, identifying it as a version rolled out ahead of this build rather than as missing data

#### Scenario: Malformed payload is surfaced as an error
- **WHEN** a node's resolved entry carries a known version whose content does not parse
- **THEN** it resolves to no location and this is logged at error level

## REMOVED Requirements

### Requirement: Geodata SHALL be cached only on success and re-attempted every cycle otherwise

**Reason**: Every mechanism this requirement specifies is gone. There is no third-party lookup to fail per IP, no per-node cache entry to expire under `geodata_ttl`, and no per-node retry, because one verified contract read now returns the whole set at one height. The double lookup it documented (once in the sweep, once while building the gateway record) disappears with the per-node path, as does the sweep's place in the monitor cycle.

**Migration**: Replaced by "Geolocation SHALL be refreshed by a dedicated worker", "Geolocation SHALL be read from the contract as one verified, height-pinned snapshot" and "Geolocation reads SHALL be pinned to the shared attestation cadence height". The one externally visible consequence it specified is preserved verbatim: a node with no usable location still yields an empty two-letter country code, which still removes the gateway from the dVPN directory. The `geodata_ttl` configuration knob and the `IPINFO_API_TOKEN` / `--ipinfo-api-token` argument are removed; a deployment passing the argument on the command line MUST drop it before upgrading.
