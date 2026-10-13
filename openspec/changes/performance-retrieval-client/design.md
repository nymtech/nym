## Context

This change is the second of four stacked PRs that get network-monitor-v3 measurements into the performance contract and serve them verifiably: the contract side (`performance-contract-verifiable-routing-view`, archived), this retrieval library, the orchestrator's submission path (`network-monitor-contract-submission`) and the nym-api consumer (`nym-api-performance-contract-consumer`). They are reviewed separately and merged together.

What the contract now provides, and this library consumes:

- The committed set is every node with a `last-known-epoch` pointer, one entry per node: `LatestPerformance { node_id, epoch_id, score }`, its newest bundle scored under the weights of that bundle's own epoch, with no per-kind medians. `score` is `None` when the bundle cannot be scored.
- The leaf is `LatestPerformance::digest_leaf()` in `nym-performance-contract-common`: 26 fixed bytes (tag `1`, big-endian `u32` node and epoch, a presence byte, the big-endian `u128` atomics of the score). It is exact, so a served score differing from the committed one by any amount yields another leaf.
- The full `LtHash16` accumulator is stored raw at `digest_state` (`storage_keys::DIGEST_STATE`); an absent key is the empty accumulator. `Digest {}` returns its 32-byte collapse, unproven.
- `LatestPerformancePaged { start_after, limit }` ranges over the pointer map in ascending node order, 100 entries by default and 200 at most. `start_next_after` is the last node returned, or `None` for an empty page, so a full enumeration ends with one empty page.
- The mixnet contract's unbond callback deletes a node's pointer in the transaction that removes its bond, and the contract rejects submissions for unbonding nodes, so the committed set holds only measured nodes that have not unbonded. Bundles are append-only.
- The validator client has `get_latest_performance_paged`, `get_performance_digest` and the unpinned collector `get_all_latest_performance`.

The shared machinery that exists today, built for the directory and geolocation clients:

- `nym-contract-anchor`: `TrustAnchor` (`trusted_app_hash(H)`, `trusted_digest(H)` returning a `TrustedDigest { height, accumulator }`), `ProvenTrustAnchor::new(client, contract, digest_key)` (ICS23 proof of the digest item against the `app_hash` from `header[H + 1]`, a proven absence read as the empty accumulator), `LightClientAnchor`, and `AttestedTrustAnchor<S>` (a K-of-N quorum of nym-api signers on small signed snapshots, exposing `trusted_snapshot(H)` and its `sources()`). Every anchor takes the contract address and the bare digest item key as parameters.
- `nym-contract-attestation`: `DigestSnapshot { chain_id, contract, height, app_hash, accumulator, node_identities_hash }`, `node_identities_hash` over a `NodeId -> ed25519` map, `SnapshotData<R> { height, records, node_identities }`, `AttestationSource` with an associated `Record` type, and `MockAttestationSource<R>`.
- `nym-contracts-common::digest`: the contract-side `DigestItem`, `DigestResponse` and `CorruptDigestState` the three contracts share since the previous change.

What is not shared yet, and is duplicated between `nym-directory-client` and `nym-geolocation-client`:

- the height-pinned paging loop in the validator client (`PinnedDirectoryQueryClient`, `PinnedGeolocationQueryClient`, and the mixnet's `get_all_nymnode_bonds_at_height`, about 35 lines each): every page at H, a check that the cursor advances, the `MAX_PINNED_READ_RECORDS` bound;
- reading the bonds at H into the identity map (`all_node_identities_at`, `node_identities_at`, identical);
- `recompute_accumulator` and the comparison with the trusted accumulator;
- the attested read through the anchor's sources, which exists only in the directory client (`AttestedDirectoryExt`): take the quorum-agreed snapshot for H, ask each source in turn for its data at H, reject a height label other than H, check the identity map's hash and the accumulator, return the first set that passes. Geolocation has only an offline function and no read through sources.

The threat this change answers: clients choose nodes from performance served by an untrusted topology server (nym-api today, nym-nodes later), which can deflate competitors below the clients' minimum-performance cutoffs (`client-core/src/client/topology_control/nym_api_provider.rs`, `client-core/src/init/helpers.rs`) or inflate its own nodes to capture traffic. At a pinned height H a client gets three guarantees: every score is authentic, the set is complete, and the snapshot is consistent with the directory at H. It does not get accuracy (monitors are trusted), freshness on its own (the client checks the time of H) or availability.

## Goals / Non-Goals

**Goals:**

- At a caller-chosen height H, verify that the served performance entries are exactly the committed set, under any trust anchor through the chain RPC, or with no chain connection against a quorum-attested snapshot.
- Give every consumer one routing view: each node bonded at H with its newest score and source epoch, with no staleness cut.
- Give the nym-api producer (PR4) everything it signs and serves at H: the accumulator, the entries, the node-identity map and the trusted `app_hash`.
- Extract the steps the clients share, which depend only on a record's leaf, into one place. Then move the directory and geolocation clients onto them with no change to their behaviour, and give geolocation the attested read it lacks.

**Non-Goals:**

- **Accuracy, freshness and availability.** Monitors are trusted, the client checks the time of H itself, and serving the last snapshot while the chain is unreachable is the calling binary's concern.
- **A generic client struct.** What each client does with a verified set has nothing in common across domains (Decision 2).
- **A combined attested endpoint serving the directory, geolocation and performance data together.** That is later work; this change only makes the attested read generic over the record type.
- **A staleness bound or a staleness policy seam** (Decision 10).
- **Joining the directory.** A consumer reads the directory at the same H through its own client; the performance client does not read it.
- **Rewarding.** Rewarding resolves through `RewardingInputs`, whose fallback this view does not reproduce; nym-api reads it in PR4.
- **The nym-api HTTP attestation source, its routes and its response types.** They come with the producer in PR4 (Decision 12).
- **Proven single-entry reads** (Decision 7).
- **Consumers.** nym-api (PR4) and a verifying client topology provider (later) are separate changes.

## Decisions

### 1. A `DigestLeaf` trait owns each record type's leaf

`nym-contracts-common::digest` gains, outside the `lthash` feature:

```rust
pub trait DigestLeaf {
    fn digest_leaf(&self) -> Vec<u8>;
}
```

`DirectoryEntryRecord`, `GeolocationRecord` and `LatestPerformance` implement it in their common crates. Each type's existing inherent `digest_leaf` moves into its trait impl unchanged, so there is still exactly one encoding per type; the contracts and the root-workspace callers import the trait. The bytes do not change, and each common crate's existing leaf tests keep pinning them.

The leaf is always a `Vec<u8>`, with no associated type. Only the performance leaf has a fixed length, and keeping it a `[u8; 26]` would spare one 26-byte allocation per entry at the cost of an `AsRef` bound in every generic fold. Its length stays pinned by its layout test.

The trait lives in `nym-contracts-common` because every contract common crate already depends on it and can implement it there. A trait defined in a client-side crate could only be implemented inside that crate for foreign types, which would make it depend on every contract's common crate.

### 2. Shared whole-set verification lives in `nym-contract-anchor`

A new `whole_set` module in `nym-contract-anchor` holds the steps that depend only on a record's leaf:

- `recompute_accumulator<R: DigestLeaf>(&[R]) -> LtHash16`, folding every leaf into an empty accumulator;
- `verify_records<R: DigestLeaf>(&[R], trusted: &LtHash16)`, the fold and the comparison;
- `verify_snapshot_data<R: DigestLeaf>(&SnapshotData<R>, height, trusted: &TrustedSnapshot)`: the data's height label against the requested height, the identity map against `node_identities_hash`, then the records against the accumulator;
- `node_identities_at(client, height)`, reading `get_all_nymnode_bonds_at_height(height)` into the `NodeId -> ed25519` map and skipping, with an error log, a key that does not parse, as both clients do today.

`TrustedSnapshot` gains a public `From<DigestSnapshot>`, replacing its private constructor, so a caller holding a `DigestSnapshot` it obtained itself uses the same check.

The crate already holds the domain-neutral anchors and depends on the validator client, `nym-crypto` and `nym-contract-attestation`; it gains a dependency on `nym-contracts-common` for the trait. The identity map cannot be a method on `PinnedMixnetQueryClient`, because the validator client does not depend on `nym-crypto`.

Not extracted:

- **A generic client struct, `Client<A, C, R>`.** After verification the directory attributes node signatures and groups entries, geolocation groups entries, checks attestations and builds its whitelist, and performance keys entries by node and builds the routing view. With the shared steps, each client's RPC read is a few lines of composition: check the addresses, take the trusted digest, page at H, read the identities, verify, build.
- **The per-domain result types.**
- **The sibling clients' `key.rs`.** It is public API that nym-api and nym-node-status-api import. The performance client simply has none (Decision 11).

### 3. The attested read moves onto the anchor

`AttestedTrustAnchor<S>` gains `verified_snapshot_data(height) -> Result<SnapshotData<S::Record>, RecordSetError>`, available when `S::Record: DigestLeaf`. It is the loop `AttestedDirectoryExt` runs today:

1. take the quorum-agreed `trusted_snapshot(height)`;
2. ask each source in turn for `snapshot_data(height)`;
3. check the data with `verify_snapshot_data` and return the first set that passes;
4. otherwise return the last failure, or `NoQuorumSnapshotForHeight` when there are no sources.

All the trust comes from K signers agreeing on the small signed snapshot. The bulk data can come from any single source because its recompute must reproduce that snapshot's hashes.

The anchor still knows nothing about what a record means, only that it has a leaf. A source is skipped only when it fails one of the structural checks or fails to answer. Domain checks applied afterwards, such as the directory's node signatures, are reported in the result and never cause a source to be skipped, exactly as today, since the data is still what the contract committed. Each client's attested read is then one call plus its own builder:

- `AttestedDirectoryExt` keeps its signature and becomes that wrapper;
- geolocation gains `AttestedGeolocationExt::verified_geolocation(height)`, returning the same `VerifiedGeolocation` as its RPC path;
- performance gets `AttestedPerformanceExt::verified_performance(height)`.

### 4. One error type for the shared steps, mapped onto each client's own variants

`nym-contract-anchor` gains `RecordSetError` beside `AnchorError`:

- `DigestMismatch`;
- `NodeIdentitiesMismatch`;
- `SnapshotHeightMismatch { requested, received }`;
- `Anchor(AnchorError)`, for when the quorum cannot be reached for the height;
- `Transport(AttestationSourceError)`, for when no source could answer.

The pure checks raise only the first three; the attested read raises any of them. `RecordSetError` stays separate from `AnchorError`, because `AnchorError` covers anchoring and proving, and a served record set that fails to verify is neither.

Each client implements `From<RecordSetError>` onto its existing variants, so its public error enum and every test assertion stay as they are. The directory keeps reporting an identity-map mismatch as `DigestMismatch`, as it does today. Geolocation gains `SnapshotHeightMismatch` and `AttestationTransport` for its new attested read. Nothing outside the client crates matches on these variants: nym-api and nym-node-status-api only name the error types.

### 5. One paging helper for every height-pinned read

A private generic async function in the validator client's `contract_traits/mod.rs` runs the loop the three pinned reads repeat. It takes:

- the contract address and a contract label for its error messages;
- the height;
- a query builder from the cursor, `Fn(Option<K>) -> Q`;
- a page splitter into records and the next cursor, `Fn(P) -> (Vec<R>, Option<K>)`.

It issues every query through `query_contract_smart_at_height` at the height, fails when a page's cursor equals the one it was requested with, and fails once past `MAX_PINNED_READ_RECORDS`. The directory, geolocation and mixnet bond reads move onto it unchanged. The fourth user is the new `PinnedPerformanceQueryClient::get_all_latest_performance_at_height(H)`, blanket-implemented over `CosmWasmClient + NymContractsProvider + Send + Sync` like its siblings, with `LatestPerformancePaged { start_after, limit: None }`.

The digest is proven at H, so reading pages at the chain tip would turn a submission landing mid-enumeration into a mismatch indistinguishable from tampering. The unpinned collector `get_all_latest_performance` stays for display and diagnostics, and its doc gains the geolocation collector's warning that it must not feed a digest comparison.

### 6. The directory and geolocation clients move without behaviour change

Both clients drop their own `recompute_accumulator`, comparison, identity-map read and (for the directory) source loop in favour of the shared steps, and keep:

- their client structs and verified result types;
- their offline functions (`verify_directory_offline`, `verify_geolocation_offline`);
- their public error enums;
- their `key.rs`.

Their existing tests pass with their assertions untouched; only imports change, where a test called a function that moved. The directory's RPC path keeps checking the identity map only when given a trusted hash, as today. The one addition is geolocation's attested read (Decision 3), with its own tests and a spec delta.

### 7. Performance: whole-set verification only, with the contract's own leaf

The performance client folds `LatestPerformance`'s `DigestLeaf` implementation, the encoding the contract itself folds. Entries are folded as served. The leaf commits the score, so the client never reads or proves weights and never sees medians. The contract side chose to commit the score precisely so that verifiers need not prove weights, which live in a sparse map resolved by a descending range.

Proven single-entry reads are not offered. A node's pointer and its bundle are each provable by an ICS23 point proof, but its score also needs the weights in force at its source epoch. A point proof can show that a weights entry exists, not that no other lies between it and that epoch. The whole committed set is about 1000 entries at mainnet scale, a handful of pages, so a client wanting one node's score reads them all; only the whole-set recompute establishes completeness anyway.

### 8. The bond set at H is the node-identity map

The verified result carries the `NodeId -> ed25519` identity map of the mixnet bonds at H, and the routing view filters by its keys. The attested snapshot commits the identity map, not a set of ids, so the offline path must receive and hash the map. Carrying the same map in the RPC path gives PR4's producer the `node_identities_hash` it signs.

Through the RPC the map comes from the shared `node_identities_at`, which both siblings now use, so a producer attesting the directory and performance at the same H signs the same `node_identities_hash` for both. Offline, the map is the one served with the entries, accepted only if it hashes to the snapshot's `node_identities_hash`.

### 9. The verified result and the routing view

`VerifiedPerformance { height, accumulator, entries: BTreeMap<NodeId, LatestPerformance>, node_identities }`. The entries are the raw records themselves, so a producer re-serves `entries.values()` verbatim and needs no separate raw list.

`VerifiedPerformance::routing_view()` returns `RoutingView { height, nodes: BTreeMap<NodeId, Option<LatestPerformance>> }`: every node in the identity map, with its committed entry, or `None` when it has never been measured.

- **"Score at X, else last known".** A node's committed entry is its bundle at the current epoch X when it was measured in X, and its newest earlier bundle otherwise. So the view gives exactly that without the library knowing X.
- **No parallel type.** An entry carries its own `node_id`, `epoch_id` (the source epoch) and `score`, so the view reuses `LatestPerformance`.
- **A committed entry whose node is not bonded at H** is left out of the view and stays in `entries`. It cannot arise while the mixnet contract holds the performance contract's address, since the callback deletes the pointer in the transaction that removes the bond, and the contract-side deployment ships both together. So this is a guard, not an expected state.
- **An entry with `score: None`** stays in the view. PR3 never submits a bundle that cannot score, but the client does not assume it.

### 10. Staleness is unbounded and there is no policy seam

The view never drops, discounts or flags an entry for its age. During a measurement outage of any length the network keeps operating on last known values, and unbonded nodes leave through the mixnet callback, so only a bonded node that stopped being measured can carry an old entry. Every entry carries its source epoch, so a consumer that wants a bound applies it to the view.

Rejected: a staleness policy trait, mirroring the geolocation client's `ResolutionPolicy`. That seam exists because a geolocation subject holds several entries to select among. Here each node has exactly one, so the only possible decision is a cut-off, which this design rules out. A cut-off would also need the current epoch at H, which no digest covers.

### 11. The performance client's entry points and errors

- **`PerformanceClient<A, C>`**, where `A: TrustAnchor + Sync` and `C: NymContractsProvider + PinnedPerformanceQueryClient + PinnedMixnetQueryClient + Sync`.
  - `verified_performance(H)` first checks that the performance and mixnet contract addresses are configured, so a missing one surfaces as this crate's typed variant rather than a generic query failure. It then takes `trusted_digest(H)`, enumerates at H, reads `node_identities_at(H)`, checks with `verify_records` and builds the result.
  - `trusted_app_hash(H)` passes the anchor's through, as `DirectoryClient` does, so the producer signs the `app_hash` from the same anchor that established the digest.
  - The caller builds the anchor with the performance contract's address and the bare item key `storage_keys::DIGEST_STATE.as_bytes()`. The type's doc says so, because the anchor prefixes the contract itself and a pre-prefixed key never matches a proof. There is no `key.rs`: the raw-key layout is the shared `digest_storage_key`, already tested in `nym-contract-anchor`.
- **`AttestedPerformanceExt::verified_performance(H)`** wraps `verified_snapshot_data` (Decision 3).
- **`verify_performance_offline(&DigestSnapshot, SnapshotData<LatestPerformance>)`** is for a caller holding a quorum-agreed snapshot and data obtained some other way: `verify_snapshot_data` at the snapshot's height, then the builder.

`PerformanceClientError` follows the siblings:

- `Anchor(AnchorError)`, wrapped so the anchoring step that failed survives;
- `ChainQuery(NyxdError)`;
- `DigestMismatch`;
- `NodeIdentitiesMismatch`;
- `SnapshotHeightMismatch { requested, received }`;
- `AttestationTransport(AttestationSourceError)`;
- `UnavailablePerformanceContract`;
- `UnavailableMixnetContract`.

It has `From<RecordSetError>`, and no variant carries entries, so no error path returns data.

### 12. The nym-api HTTP source comes with the producer

The geolocation client shipped a stub `NymApiGeolocationSource` for a producer that was never built. Here the producer is scheduled (PR4) and merged together with this change, and its routes and response types are defined alongside it. So PR4 adds the nym-api source (`type Record = LatestPerformance`) and its `NymApiClientExt` methods to this crate, together with the routes they call. This change's attested read is generic over `AttestationSource` and tested with mock sources.

### 13. Testing

Offline only; no unit test depends on the network.

- **Shared core**, in `nym-contract-anchor`, through a small test-local record type implementing `DigestLeaf` and `MockAttestationSource` over it:
  - the fold is order-independent and an empty set is the empty accumulator;
  - an added, withheld or altered record fails;
  - a mislabelled height, a substituted identity map and an extended identity map each fail;
  - `verified_snapshot_data` covers the happy path, skips a bad source for a good one, returns the last failure when every source fails, and fails when the quorum is not reached.
- **Directory and geolocation:** their existing suites are the regression check for Decision 6, with assertions untouched. Geolocation's attested read gets the happy path, a skipped bad source and every source tampered.
- **Performance:**
  - verification: a score altered from `0.85` to `0.8549` or another epoch fails, and an unscoreable entry verifies and is kept;
  - the offline function covers both halves;
  - the attested read covers its happy path and one failure; the loop itself is tested once, in the anchor crate;
  - the routing view covers each of its scenarios.

Test keys come from the `nym_test_utils` helpers.

Not covered by unit tests:
- the pinned reads, the paging helper's callers and `PerformanceClient`'s wiring, which are blanket-implemented over `CosmWasmClient` and cannot be mocked without implementing that trait (the gap the geolocation client already accepted);
- a live ICS23 fixture for the performance digest key, because the contract with the digest runs nowhere yet. The proof path is the shared anchor code, covered by the directory and geolocation fixtures.

## Risks / Trade-offs

- [The refactor changes sibling behaviour by accident] -> Their test suites stay as they are and must pass, the public error enums are mapped rather than changed, and nothing outside the client crates names the moved functions.
- [The paging helper's closures make the pinned reads harder to follow than the loops they replace] -> Each call site is a query builder and a page splitter; the cursor and bound rules live in one place and so cannot drift between contracts.
- [Moving `digest_leaf` into a trait touches the contracts] -> Only imports change; the encodings and every leaf test are unchanged, and the contracts' behaviour is identical.
- [The bond set is an unproven smart query on the RPC path] -> As in the directory and geolocation clients. A dishonest RPC can add or hide bonded nodes in the view, never alter a committed score. Under the proven anchor the RPC is trusted for the `app_hash` anyway, and the attested path checks the map against the quorum's `node_identities_hash`.
- [Old entries are served unflagged] -> Intended (Decision 10). Each entry carries its source epoch, and PR3 alerts on any incomplete epoch, so an outage is noticed rather than silently frozen.
- [The pinned reads and the client wiring have no unit tests] -> Both are thin; the verification and the attested read, which hold the security property, are fully tested.
- [A node returns pages without end] -> A repeated cursor fails at once, and `MAX_PINNED_READ_RECORDS` bounds fresh fabricated pages.
- [The client depends on the frozen leaf format] -> It uses the common crate's `DigestLeaf` implementation, so contract and client share one encoding; a new format takes a new tag and both move together.
- [The performance attested read is not usable end to end until PR4] -> The four PRs merge together; until then it is exercised through mock sources.

## Migration Plan

None. No contract state, message or leaf changes; the sibling clients keep their public API; the performance client has no consumer until PR4. Rollback is reverting the change.

## Open Questions

None for this change.
