## MODIFIED Requirements

### Requirement: Performance snapshots follow the directory's cadence and retained window

The producer SHALL take performance snapshots at the cadence heights of the directory contract's snapshot interval, and SHALL retain and promote them with the same retention count, settle lag and polling interval as the directory snapshots, read from `[directory.debug]`, so that every retained height serves both contracts, as `directory-attestation-provider` requires of every attested contract. Performance snapshots SHALL be produced together with every other attested contract's, as `directory-attestation-provider` requires: during startup the newest `retention_count` cadence heights strictly below the chain tip are tried, oldest first, logging and skipping any that cannot be produced for every contract, and at each poll the newest cadence height strictly below the chain tip is produced unless it is already retained. The latest snapshot SHALL be the newest retained one at least `settle_lag` blocks behind the tip last polled.

#### Scenario: Both contracts are retained at the same heights
- **WHEN** a nym-api has produced directory and performance snapshots over several cadence heights
- **THEN** it retains the same heights for both, a height either of them failed to produce being retained for neither, so a consumer holding a directory snapshot at a retained height can request performance at that height

#### Scenario: A height that cannot be served is skipped at startup
- **WHEN** the producer cannot retrieve one of the newest `retention_count` cadence heights at startup
- **THEN** it logs the failure, retains the others, and startup continues

#### Scenario: Latest waits for the settle lag
- **WHEN** a performance snapshot has just been produced for cadence height H and the tip has not reached `H + settle_lag`
- **THEN** `/v1/performance/snapshot/H` returns it while `/v1/performance/snapshot/latest` still returns the previous one

### Requirement: The producer signs only what it verified

Before signing a performance snapshot for height H, the producer SHALL retrieve and verify the performance contract's whole committed set through the shared whole-set read under the performance contract's anchor, with the mixnet bond set at H read once for every attested contract and supplied to it as the node-identity map, under the chain anchor nym-api shares across its attested contracts (a proven read through the RPC node when `[directory.debug].trusted_rpc_node` is set, a light client otherwise), with the performance contract's digest proven under its digest key. It SHALL sign a `DigestSnapshot` naming the performance contract and carrying the app hash that anchor trusts for H, the accumulator the verification established, and the hash of the bond set at H, and SHALL retain the entries exactly as served together with the bond set. When verification fails, it MUST NOT sign or retain anything for H, for this or any other attested contract, and MUST log the failure; the next poll tries the newest cadence height again. A contract that has received no submission SHALL be attested as the empty set its absent digest commits.

#### Scenario: A set that fails verification is not attested
- **WHEN** the entries read at H do not recompute to the digest the anchor trusts for H
- **THEN** no snapshot is signed or retained for H, of this or any other attested contract, and the failure is logged

#### Scenario: An empty contract is attested as empty
- **WHEN** the performance contract has received no submission at H
- **THEN** the producer signs a snapshot for H with the empty accumulator and retains no entries and the bond set at H

#### Scenario: The snapshot names the performance contract
- **WHEN** a nym-api signs a directory snapshot and a performance snapshot at the same height
- **THEN** both are `DigestSnapshot`s, distinguished by the contract address each binds into its signing payload

### Requirement: The performance snapshot cache persists across restarts

The producer SHALL persist its retained snapshots, with what is held beside them and the last submission it recorded, in the one file every attested contract's snapshots share (`directory-attestation-provider`), and SHALL load it at startup. The recorded submission SHALL be persisted as a key of its block height, epoch, sender and last node, which identifies a submission. A file that fails to load SHALL be treated as an empty cache, which the startup production refills, its first snapshot paging both epochs. Retained heights that are no longer among the newest `retention_count` cadence heights at startup SHALL be dropped.

#### Scenario: A restart keeps the retained snapshots
- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves the retained snapshots from the loaded file, together with what the startup production adds

#### Scenario: A restart pages nothing that has not changed
- **WHEN** nym-api restarts within its retained window and nothing has been submitted to the contract since its last snapshot
- **THEN** its next snapshot reuses the persisted medians without paging any epoch

#### Scenario: An unreadable file starts empty
- **WHEN** the persisted snapshot file cannot be decoded
- **THEN** nym-api starts from an empty snapshot cache and refills it at startup
