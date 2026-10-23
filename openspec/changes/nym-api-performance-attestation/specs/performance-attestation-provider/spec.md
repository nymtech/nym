## ADDED Requirements

### Requirement: Every nym-api produces attested performance snapshots

nym-api SHALL produce signed snapshots of the performance contract's committed set on every instance, whatever its configuration: independently of `[performance_provider].use_performance_contract_data` and of whether it rewards. No setting SHALL turn the producer off. The performance contract's address SHALL be assumed present in the network details, and network details that do not carry it MUST fail startup, as a missing directory contract address does.

#### Scenario: A default configuration produces snapshots
- **WHEN** nym-api starts with a default configuration and network details carrying the performance contract's address
- **THEN** it produces performance snapshots and serves them under `/v1/performance`

#### Scenario: A missing address fails startup
- **WHEN** the network details carry no performance contract address
- **THEN** startup fails, whatever the configuration

### Requirement: Performance snapshots follow the directory's cadence and retained window

The producer SHALL take performance snapshots at the cadence heights of the directory contract's snapshot interval, and SHALL retain and promote them with the same retention count, settle lag and polling interval as the directory snapshots, read from `[directory.debug]`, so that every retained height serves both contracts, as `directory-attestation-provider` requires of every attested contract. During startup it SHALL try to produce the newest `retention_count` cadence heights, oldest first, logging and skipping any it cannot, and at each poll it SHALL produce the newest cadence height strictly below the chain tip unless it already holds it. The latest snapshot SHALL be the newest retained one at least `settle_lag` blocks behind the tip last polled.

#### Scenario: Both contracts are retained at the same heights
- **WHEN** a nym-api has produced directory and performance snapshots over several cadence heights
- **THEN** it retains the same heights for both, so a consumer holding a directory snapshot at a retained height can request performance at that height

#### Scenario: A height that cannot be served is skipped at startup
- **WHEN** the producer cannot retrieve one of the newest `retention_count` cadence heights at startup
- **THEN** it logs the failure, retains the others, and startup continues

#### Scenario: Latest waits for the settle lag
- **WHEN** a performance snapshot has just been produced for cadence height H and the tip has not reached `H + settle_lag`
- **THEN** `/v1/performance/snapshot/H` returns it while `/v1/performance/snapshot/latest` still returns the previous one

### Requirement: The producer signs only what it verified

Before signing a performance snapshot for height H, the producer SHALL retrieve and verify the performance contract's whole committed set and the mixnet bond set at H through `nym-performance-client`'s verified read, under the chain anchor nym-api shares across its attested contracts (a proven read through the RPC node when `[directory.debug].trusted_rpc_node` is set, a light client otherwise), with the performance contract's digest proven under its digest key. It SHALL sign a `DigestSnapshot` naming the performance contract and carrying the app hash that anchor trusts for H, the accumulator the verification established, and the hash of the bond set at H, and SHALL retain the entries exactly as served together with the bond set. When verification fails, it MUST NOT sign or retain anything for H, and MUST log the failure; the next poll tries the newest cadence height again. A contract that has received no submission SHALL be attested as the empty set its absent digest commits.

#### Scenario: A set that fails verification is not attested
- **WHEN** the entries read at H do not recompute to the digest the anchor trusts for H
- **THEN** no snapshot is signed or retained for H, and the failure is logged

#### Scenario: An empty contract is attested as empty
- **WHEN** the performance contract has received no submission at H
- **THEN** the producer signs a snapshot for H with the empty accumulator and retains no entries and the bond set at H

#### Scenario: The snapshot names the performance contract
- **WHEN** a nym-api signs a directory snapshot and a performance snapshot at the same height
- **THEN** both are `DigestSnapshot`s, distinguished by the contract address each binds into its signing payload

### Requirement: Performance snapshots are served under their own route tree

nym-api SHALL serve its performance snapshots under `/v1/performance`, mirroring `/v1/directory` without its subset routes: `/snapshot/latest`, the latest snapshot, answering 503 while none has settled; `/snapshot/{height}`, the snapshot at a retained height; and `/entries/{height}/records`, the retained entries at a height as `LatestPerformance` records exactly as committed, paged with a default of 200 and a maximum of 500 per page. A height outside the retained window MUST answer 404. The tree SHALL NOT serve a node-identity map of its own: the map at a height is the mixnet bond set at that height, identical to the directory's and hashed identically into both snapshots, and is served by the directory tree.

#### Scenario: A no-RPC client verifies a producer's performance set
- **WHEN** a client holding a quorum-attested performance snapshot for a retained height H fetches every page of `/v1/performance/entries/H/records` and `/v1/directory/entries/H/node-identities`
- **THEN** the entries recompute to the snapshot's accumulator and the identities to its `node_identities_hash`

#### Scenario: A height outside the window is not found
- **WHEN** a client requests a performance snapshot or records at a height the producer does not retain
- **THEN** the response is 404

#### Scenario: Latest before any snapshot has settled
- **WHEN** no retained performance snapshot is yet `settle_lag` blocks behind the tip
- **THEN** `/v1/performance/snapshot/latest` answers 503

#### Scenario: Records are paged
- **WHEN** a client requests `/v1/performance/entries/H/records` with no page size, or with a page size above 500
- **THEN** it receives at most 200, respectively 500, records, with the total count of the retained set

### Requirement: Unproven medians and weights are read beside each snapshot

At each height H it snapshots, the producer SHALL also read, every query pinned at H, the weights in force at the epoch current at H (`CurrentWeights`), the block time of H, and the per-node medians of the newest epoch among the snapshot's entries and of the epoch before it, through `EpochPerformancePaged`. Entries from older epochs SHALL get no medians, and no per-node query SHALL be made for them. These SHALL be held with the snapshot for H and read only to build annotations. They MUST NOT enter the signed snapshot and MUST NOT be served under `/v1/performance`, since the digest does not commit them.

The producer SHALL first read the contract's last submission at H. When it equals the one recorded at the previous snapshot, nothing has been submitted since, and the medians held SHALL be reused without paging. Otherwise it SHALL re-page each of those two epochs that is not older than the recorded submission's epoch, and record the new submission. The medians it holds for an older one SHALL be kept, not re-paged: an epoch is frozen once its transition begins.

When any of these reads fails, nothing SHALL be retained for H, as when verification fails.

#### Scenario: An unchanged last submission pages nothing
- **WHEN** the last submission at H equals the one recorded at the previous snapshot
- **THEN** the producer reads the last submission, the weights and the block time at H, and reuses the medians it holds without paging any epoch

#### Scenario: A new submission re-pages only what can have changed
- **WHEN** the recorded submission was for epoch 99 and the last submission at H is for epoch 100
- **THEN** the producer re-pages epochs 99 and 100 at H, holds the medians of those two, and pages no older epoch

#### Scenario: Medians come from the bundle as it stood at H
- **WHEN** node 7's committed entry at H names epoch 100, and node 7's bundle at epoch 100 later changes after H
- **THEN** the medians held for H are those of node 7's bundle at epoch 100 as it stood at H

#### Scenario: An entry from an older epoch has no medians
- **WHEN** the newest epoch among the entries at H is 100, and node 8's committed entry names epoch 80
- **THEN** the producer holds no medians for node 8 and makes no query for it

#### Scenario: A failed auxiliary read retains nothing
- **WHEN** the verified read at H succeeds but the last submission, weights, block time or a medians page fails
- **THEN** nothing is retained for H, and the next poll tries the newest cadence height again

#### Scenario: The medians are never served on the attested routes
- **WHEN** a client fetches anything under `/v1/performance`
- **THEN** the response carries no medians or weights

### Requirement: The performance snapshot cache persists across restarts

The producer SHALL persist its retained snapshots, with what is held beside them and the last submission it recorded, to a file of its own, separate from the directory's, and SHALL load it at startup. The recorded submission SHALL be persisted as a key of its block height, epoch, sender and last node, which identifies a submission. A file that fails to load SHALL be treated as an empty cache, which the startup production refills, its first snapshot paging both epochs. Retained heights that are no longer among the newest `retention_count` cadence heights at startup SHALL be dropped.

#### Scenario: A restart keeps the retained snapshots
- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves the retained snapshots from the loaded file, together with what the startup production adds

#### Scenario: A restart pages nothing that has not changed
- **WHEN** nym-api restarts and nothing has been submitted to the contract since its last snapshot
- **THEN** its next snapshot reuses the persisted medians without paging any epoch

#### Scenario: An unreadable file starts empty
- **WHEN** the persisted performance snapshot file cannot be decoded
- **THEN** nym-api starts from an empty performance snapshot cache and refills it at startup
