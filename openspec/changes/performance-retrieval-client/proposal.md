## Why

The performance contract now commits every node's latest score to an LtHash digest stored raw for ICS23 proofs, and serves the committed entries through `LatestPerformancePaged` (`performance-contract-verifiable-routing-view`). Nothing on the client side uses it yet. Clients still choose nodes by performance served from an untrusted topology server, which can deflate competitors below their minimum-performance cutoffs or inflate its own nodes to capture traffic, and nym-api has no verified view to publish scores or select the active set from. This change is the other half of that design: a library that retrieves the committed set at one height, proves it complete and authentic against a trust anchor, and derives the routing view from it.

It would be the third client built on the same pattern, after the directory and geolocation clients, and those two already carry identical copies of every step that does not depend on what a record means: height-pinned paging, reading the bond set at a height, folding and comparing the accumulator, and fetching attested data from one source at a time. Geolocation lacks that last step entirely, though it needs it. So this change first extracts those steps into the shared anchor crate, moves both existing clients onto them without changing their behaviour, and builds the performance client on top.

## What Changes

- A `DigestLeaf` trait in `nym-contracts-common`, through which a contract's record type exposes the leaf its contract folds. `DirectoryEntryRecord`, `GeolocationRecord` and `LatestPerformance` implement it with their existing encodings, which move into the trait unchanged.
- Shared whole-set verification in `nym-contract-anchor`, generic over `DigestLeaf`: the fold and comparison, the check of attested data against a trusted snapshot (height label, node-identity hash, accumulator), the bond set at a height as a `NodeId -> ed25519` map, a shared error type, and `AttestedTrustAnchor::verified_snapshot_data`, which fetches a height's data from one source at a time and returns the first set that verifies against the quorum.
- A shared height-pinned paging helper in the validator client, used by the directory, geolocation and mixnet bond reads and by a new `PinnedPerformanceQueryClient::get_all_latest_performance_at_height`. The unpinned `get_all_latest_performance` gains the warning that it must not feed a digest comparison.
- The directory and geolocation clients move onto the shared steps with their public types, error variants and test assertions unchanged. Geolocation gains an attested read through its anchor's sources, which it does not have today.
- A new crate `nym-performance-client`: the committed set verified at a pinned height H under any trust anchor, or offline against a quorum-attested snapshot; the bond set at H alongside it; and a routing view listing every node bonded at H with its newest committed score and source epoch, or nothing if it was never measured. Staleness is unbounded.

## Capabilities

### New Capabilities

- `performance-retrieval-client`: verified whole-set retrieval of the performance contract's committed entries at a pinned height, through the chain RPC under any trust anchor or offline against a quorum-attested snapshot; the bond set at that height; and the routing view derived from both.

### Modified Capabilities

- `contract-trust-anchor`: adds the shared whole-set verification generic over the record's leaf, the attested anchor's verified read of a height's data from its sources, and the requirement that moving the directory and geolocation clients onto them preserves their behaviour.
- `geolocation-retrieval-client`: adds verified retrieval through an attested anchor's sources, alongside the existing offline verification.

## Impact

**Code**:
- `common/cosmwasm-smart-contracts/contracts-common` (the trait) and the directory, geolocation and performance common crates (its implementations);
- the three contracts and the root-workspace callers of `digest_leaf`, which import the trait;
- `common/nym-contract-anchor` (the shared verification, the attested read, the error type);
- `common/client-libs/validator-client` (the paging helper and the pinned performance read);
- `common/nym-directory-client` and `common/nym-geolocation-client` (moved onto the shared steps);
- the new crate `common/nym-performance-client` (a root workspace member and workspace dependency).

No contract behaviour, message, stored byte or leaf encoding changes.

**Consumers**: nym-api, nym-node-status-api and the checkpoint updater use only the client structs, error types, anchors and key helpers, all unchanged. Nothing consumes the performance client in this change: nym-api (`nym-api-performance-contract-consumer`) uses it for published performance and active-set selection and signs attested snapshots of it, and the nym-api HTTP attestation source and its routes come with that producer.
