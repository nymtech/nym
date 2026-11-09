## MODIFIED Requirements

### Requirement: Geolocation snapshots follow the directory's cadence and retained window

The producer SHALL take geolocation snapshots at the cadence heights of the directory contract's snapshot interval, and SHALL retain and promote them with the same retention count, settle lag and polling interval as the directory and performance snapshots, read from `[directory.debug]`, so that every retained height serves every attested contract, as `directory-attestation-provider` requires. Geolocation snapshots SHALL be produced together with every other attested contract's, as `directory-attestation-provider` requires: during startup the newest `retention_count` cadence heights strictly below the chain tip are tried, oldest first, logging and skipping any that cannot be produced for every contract, and at each poll the newest cadence height strictly below the chain tip is produced unless it is already retained. The latest snapshot SHALL be the newest retained one at least `settle_lag` blocks behind the tip last polled.

#### Scenario: Every attested contract is retained at the same heights
- **WHEN** a nym-api has produced directory, performance and geolocation snapshots over several cadence heights
- **THEN** it retains the same heights for all three, a height any of them failed to produce being retained for none, so a consumer holding a directory snapshot at a retained height can request geolocation at that height

#### Scenario: A height that cannot be served is skipped at startup
- **WHEN** the producer cannot retrieve one of the newest `retention_count` cadence heights at startup
- **THEN** it logs the failure, retains the others, and startup continues

#### Scenario: Latest waits for the settle lag
- **WHEN** a geolocation snapshot has just been produced for cadence height H and the tip has not reached `H + settle_lag`
- **THEN** `/v1/geolocation/snapshot/H` returns it while `/v1/geolocation/snapshot/latest` still returns the previous one

### Requirement: The producer signs only what it verified

Before signing a geolocation snapshot for height H, the producer SHALL retrieve and verify the geolocation contract's whole committed set, the location entries and the whitelisted-agent entries alike, through the shared whole-set read under the geolocation contract's anchor, with the mixnet bond set at H read once for every attested contract and supplied to it as the node-identity map, under the chain anchor nym-api shares across its attested contracts, with the geolocation contract's digest proven under its digest key. It SHALL sign a `DigestSnapshot` naming the geolocation contract and carrying the app hash that anchor trusts for H, the accumulator the verification established, and the hash of the bond set at H, and SHALL retain the records exactly as served together with the bond set. Nothing SHALL be read or held beside the records: every geolocation payload is inside the digest. When verification fails, it MUST NOT sign or retain anything for H, for this or any other attested contract, and MUST log the failure; the next poll tries the newest cadence height again. A contract that holds no entry SHALL be attested as the empty set its absent digest commits.

#### Scenario: A set that fails verification is not attested
- **WHEN** the records read at H do not recompute to the digest the anchor trusts for H
- **THEN** no snapshot is signed or retained for H, of this or any other attested contract, and the failure is logged

#### Scenario: An empty contract is attested as empty
- **WHEN** the geolocation contract holds no entry at H
- **THEN** the producer signs a snapshot for H with the empty accumulator and retains no records and the bond set at H

#### Scenario: The snapshot names the geolocation contract
- **WHEN** a nym-api signs a directory snapshot, a performance snapshot and a geolocation snapshot at the same height
- **THEN** all three are `DigestSnapshot`s, distinguished by the contract address each binds into its signing payload

### Requirement: The geolocation snapshot cache persists across restarts

The producer SHALL persist its retained snapshots in the one file every attested contract's snapshots share (`directory-attestation-provider`), and SHALL load it at startup. A file that fails to load SHALL be treated as an empty cache, which the startup production refills. Retained heights that are no longer among the newest `retention_count` cadence heights at startup SHALL be dropped.

#### Scenario: A restart keeps the retained snapshots
- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves the retained snapshots from the loaded file, together with what the startup production adds

#### Scenario: An unreadable file starts empty
- **WHEN** the persisted snapshot file cannot be decoded
- **THEN** nym-api starts from an empty snapshot cache and refills it at startup
