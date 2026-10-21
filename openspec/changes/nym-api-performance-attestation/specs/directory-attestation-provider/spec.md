## MODIFIED Requirements

### Requirement: Deterministic production, retained window, and settle-lagged latest

A producer SHALL retain the most recent `N` cadence snapshots (a local configuration value) and SHALL answer an explicit request for any retained cadence height immediately once that height is produced. The producer SHALL advertise as "latest" the greatest cadence height `H` such that the current chain tip is at least `H + settle_lag` blocks, where `settle_lag` is a small local configuration value in blocks. A freshly produced cadence height SHALL be explicitly queryable before it is advertised as latest.

In nym-api, `N` SHALL be `[directory.debug].retention_count` and `settle_lag` SHALL be `[directory.debug].settle_lag`, both shared by every contract the producer attests. The settle lag MUST NOT be taken from any other setting.

#### Scenario: Explicit height is served before it becomes latest

- **WHEN** a producer has just produced the snapshot for cadence height `H` and the tip has not yet reached `H + settle_lag`
- **THEN** an explicit query for height `H` returns that snapshot, while "latest" still returns the previous cadence height

#### Scenario: Latest advances after the settle lag

- **WHEN** the chain tip reaches `H + settle_lag`
- **THEN** "latest" advances to `H`

#### Scenario: Old snapshots fall out of the window

- **WHEN** more than `N` cadence snapshots have been produced
- **THEN** the producer retains only the most recent `N`, and an explicit query for an evicted height is reported as unavailable

#### Scenario: The configured settle lag decides latest

- **WHEN** nym-api runs with `retention_count` 3 and `settle_lag` 10, and the tip is 5 blocks past the newest retained cadence height `H`
- **THEN** "latest" is still the cadence height before `H`, because the settle lag, not the retention count, decides it

### Requirement: Whole-directory serving at retained heights

A producer SHALL be able to serve the whole verified contract record set (entries and the node-identity mapping) at a retained cadence height, so a client with no chain RPC connection can retrieve and verify it against that height's quorum-attested `accumulator` and `node_identities_hash`. This serving path SHALL use the values already committed by `DigestSnapshot` and SHALL NOT require the generic subset mechanism. The transfer payload SHALL be generic over the record type, so one type serves directory records and geolocation records rather than one struct per domain.

The node-identity mapping is the mixnet bond set at the height, the same for every contract attested there. The directory tree's node-identity route SHALL therefore serve it for every contract the producer attests at that height; the performance tree serves none of its own.

#### Scenario: No-RPC client pulls and verifies the full directory

- **WHEN** a client fetches the full directory at a retained height `H` from a producer and holds the quorum-attested snapshot for `H`
- **THEN** it verifies the entries against the `accumulator` and the node identities against the `node_identities_hash` by local recompute alone, with no chain query

#### Scenario: The same payload shape carries geolocation records

- **WHEN** a producer serves geolocation records at a retained height
- **THEN** it uses the same generic transfer payload, parameterised by the geolocation record type, with no second bespoke struct and no duplicated identity-map encoding

#### Scenario: One identity map serves every contract at a height

- **WHEN** a producer has signed a directory snapshot and a performance snapshot at the same cadence height `H`
- **THEN** both carry the same `node_identities_hash`, and the map served by the directory tree's node-identity route at `H` verifies against either
