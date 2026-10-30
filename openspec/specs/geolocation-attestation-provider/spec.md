# geolocation-attestation-provider Specification

## Purpose

Defines nym-api's producer of attested geolocation snapshots: on every nym-api, at the directory's cadence and retained window, it verifies the geolocation contract's whole committed set, the location entries and the whitelisted agents alike, through `nym-geolocation-client` under the chain anchor it shares across its attested contracts, signs a `DigestSnapshot` for it and serves it with the records exactly as committed under `/v1/geolocation`, so that a client without a chain connection can verify every node's geolocation entries, and which of them were authorised, against a quorum of producers. Nothing is held beside the records, since every geolocation payload is inside the digest, and it persists what it retains across restarts.

## Requirements
### Requirement: Every nym-api produces attested geolocation snapshots

nym-api SHALL produce signed snapshots of the geolocation contract's committed set on every instance, whatever its configuration. No setting SHALL turn the producer off. The geolocation contract's address SHALL be assumed present in the network details, and network details that do not carry it MUST fail startup, as a missing directory or performance contract address does.

#### Scenario: A default configuration produces snapshots
- **WHEN** nym-api starts with a default configuration and network details carrying the geolocation contract's address
- **THEN** it produces geolocation snapshots and serves them under `/v1/geolocation`

#### Scenario: A missing address fails startup
- **WHEN** the network details carry no geolocation contract address
- **THEN** startup fails, whatever the configuration

### Requirement: Geolocation snapshots follow the directory's cadence and retained window

The producer SHALL take geolocation snapshots at the cadence heights of the directory contract's snapshot interval, and SHALL retain and promote them with the same retention count, settle lag and polling interval as the directory and performance snapshots, read from `[directory.debug]`, so that every retained height serves every attested contract, as `directory-attestation-provider` requires. During startup it SHALL try to produce the newest `retention_count` cadence heights strictly below the chain tip, oldest first, logging and skipping any it cannot, and at each poll it SHALL produce the newest cadence height strictly below the chain tip unless it already holds it. The latest snapshot SHALL be the newest retained one at least `settle_lag` blocks behind the tip last polled.

#### Scenario: Every attested contract is retained at the same heights
- **WHEN** a nym-api has produced directory, performance and geolocation snapshots over several cadence heights
- **THEN** it retains the same heights for all three, apart from a height one of them failed to produce, so a consumer holding a directory snapshot at a retained height can request geolocation at that height

#### Scenario: A height that cannot be served is skipped at startup
- **WHEN** the producer cannot retrieve one of the newest `retention_count` cadence heights at startup
- **THEN** it logs the failure, retains the others, and startup continues

#### Scenario: Latest waits for the settle lag
- **WHEN** a geolocation snapshot has just been produced for cadence height H and the tip has not reached `H + settle_lag`
- **THEN** `/v1/geolocation/snapshot/H` returns it while `/v1/geolocation/snapshot/latest` still returns the previous one

### Requirement: The producer signs only what it verified

Before signing a geolocation snapshot for height H, the producer SHALL retrieve and verify the geolocation contract's whole committed set, the location entries and the whitelisted-agent entries alike, and the mixnet bond set at H through `nym-geolocation-client`'s raw verified read, under the chain anchor nym-api shares across its attested contracts, with the geolocation contract's digest proven under its digest key. It SHALL sign a `DigestSnapshot` naming the geolocation contract and carrying the app hash that anchor trusts for H, the accumulator the verification established, and the hash of the bond set at H, and SHALL retain the records exactly as served together with the bond set. Nothing SHALL be read or held beside the records: every geolocation payload is inside the digest. When verification fails, it MUST NOT sign or retain anything for H, and MUST log the failure; the next poll tries the newest cadence height again. A contract that holds no entry SHALL be attested as the empty set its absent digest commits.

#### Scenario: A set that fails verification is not attested
- **WHEN** the records read at H do not recompute to the digest the anchor trusts for H
- **THEN** no snapshot is signed or retained for H, and the failure is logged

#### Scenario: An empty contract is attested as empty
- **WHEN** the geolocation contract holds no entry at H
- **THEN** the producer signs a snapshot for H with the empty accumulator and retains no records and the bond set at H

#### Scenario: The snapshot names the geolocation contract
- **WHEN** a nym-api signs a directory snapshot, a performance snapshot and a geolocation snapshot at the same height
- **THEN** all three are `DigestSnapshot`s, distinguished by the contract address each binds into its signing payload

### Requirement: Geolocation snapshots are served under their own route tree

nym-api SHALL serve its geolocation snapshots under `/v1/geolocation`, mirroring `/v1/performance`: `/snapshot/latest`, the latest snapshot, answering 503 while none has settled; `/snapshot/{height}`, the snapshot at a retained height; and `/entries/{height}/records`, the retained records at a height as `GeolocationRecord`s exactly as committed, the whitelisted-agent records among the location records, paged with a default of 200 and a maximum of 500 per page. A height outside the retained window MUST answer 404. The tree SHALL NOT serve a node-identity map of its own: the map at a height is the mixnet bond set at that height, identical to the directory's and hashed identically into every attested contract's snapshot, and is served by the directory tree.

#### Scenario: A no-RPC client verifies a producer's geolocation set
- **WHEN** a client holding a quorum-attested geolocation snapshot for a retained height H fetches every page of `/v1/geolocation/entries/H/records` and `/v1/directory/entries/H/node-identities`
- **THEN** the records recompute to the snapshot's accumulator and the identities to its `node_identities_hash`

#### Scenario: The whitelist is served with the records
- **WHEN** a client fetches every page of `/v1/geolocation/entries/H/records`
- **THEN** the whitelisted-agent records committed at H are among them, so the client determines which measured entries were authorised at H from the verified set alone

#### Scenario: A height outside the window is not found
- **WHEN** a client requests a geolocation snapshot or records at a height the producer does not retain
- **THEN** the response is 404

#### Scenario: Latest before any snapshot has settled
- **WHEN** no retained geolocation snapshot is yet `settle_lag` blocks behind the tip
- **THEN** `/v1/geolocation/snapshot/latest` answers 503

#### Scenario: Records are paged
- **WHEN** a client requests `/v1/geolocation/entries/H/records` with no page size, or with a page size above 500
- **THEN** it receives at most 200, respectively 500, records, with the total count of the retained set

### Requirement: The geolocation snapshot cache persists across restarts

The producer SHALL persist its retained snapshots to a file of its own, separate from the directory's and the performance one, and SHALL load it at startup. A file that fails to load SHALL be treated as an empty cache, which the startup production refills. Retained heights that are no longer among the newest `retention_count` cadence heights at startup SHALL be dropped.

#### Scenario: A restart keeps the retained snapshots
- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves the retained snapshots from the loaded file, together with what the startup production adds

#### Scenario: An unreadable file starts empty
- **WHEN** the persisted geolocation snapshot file cannot be decoded
- **THEN** nym-api starts from an empty geolocation snapshot cache and refills it at startup

