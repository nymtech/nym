## 1. The `DigestLeaf` trait

- [x] 1.1 In `common/cosmwasm-smart-contracts/contracts-common/src/digest.rs` add `DigestLeaf { fn digest_leaf(&self) -> Vec<u8>; }` outside the `lthash` feature, with a one-line doc that the leaf is the contract's frozen encoding
- [x] 1.2 Implement it for `DirectoryEntryRecord`, `GeolocationRecord` and `LatestPerformance` in their common crates, moving each inherent `digest_leaf` body and doc into the impl unchanged, and re-export the trait from each common crate beside `DigestResponse`
- [x] 1.3 Import the trait at every caller: the directory, geolocation and performance contracts (`storage.rs`, `queries.rs`, the performance `testing/mod.rs`), `nym-directory-client` (`verify.rs`) and `nym-geolocation-client` (`attestation.rs`, `test_records.rs`, `verified.rs`, `verify.rs`)
- [x] 1.4 In `contracts`: `cargo test -p nym-directory-contract -p nym-geolocation-contract -p nym-performance-contract`. In the root: `cargo test -p nym-directory-contract-common -p nym-geolocation-contract-common -p nym-performance-contract-common` (the leaf tests pin the unchanged bytes) and `cargo check -p nym-directory-client -p nym-geolocation-client --tests`

## 2. Height-pinned paging (`common/client-libs/validator-client/src/nyxd/contract_traits`)

- [x] 2.1 In `mod.rs` add the private `AttestationPage` trait (`QueryMsg`, `Record`, `Cursor`, `CONTRACT`, `query_msg`, `into_parts`, as in design Decision 5) and the private `collect_pages_at_height::<P: AttestationPage, C>(client, contract_address, height)`: every query through `query_contract_smart_at_height` at the height, `extension_query_failure(P::CONTRACT, ...)` when a page's cursor equals the one requested and once past `MAX_PINNED_READ_RECORDS`, with today's messages
- [x] 2.2 Implement `AttestationPage` for `AllEntriesPagedResponse`, `AllRecordsPagedResponse` and `PagedNymNodeBondsResponse` beside their pinned reads, and move `get_all_directory_entries_at_height`, `get_all_geolocation_records_at_height` and `get_all_nymnode_bonds_at_height` onto the helper, each keeping its address lookup and missing-address error
- [x] 2.3 In `performance_query_client.rs` implement `AttestationPage` for `LatestPerformancePagedResponse` over `LatestPerformancePaged { start_after, limit: None }`, and add `PinnedPerformanceQueryClient::get_all_latest_performance_at_height(height) -> Result<Vec<LatestPerformance>, NyxdError>` over the helper, blanket-implemented over `CosmWasmClient + NymContractsProvider + Send + Sync`, a missing address reported as `unavailable_contract_address("performance contract")`. Re-export it from `mod.rs` beside `PagedPerformanceQueryClient`
- [x] 2.4 Give `get_all_latest_performance` a short doc pointer: it pins no height, so it must never be verified against a digest; use the pinned method
- [x] 2.5 `cargo check -p nym-validator-client --tests`, then `cargo test -p nym-directory-client -p nym-geolocation-client`

## 3. Shared whole-set verification (`common/nym-contract-anchor`)

- [x] 3.1 Depend on `nym-contracts-common`. In `error.rs` add `VerificationError` (design Decision 4) with `DigestMismatch`, `NodeIdentitiesMismatch`, `SnapshotHeightMismatch { requested, received }`, `Anchor(AnchorError)` and `Transport(AttestationSourceError)`, the last two through `#[from]`
- [x] 3.2 Add a `whole_set` module with `recompute_accumulator<R: DigestLeaf>`, `verify_records<R: DigestLeaf>(&[R], &LtHash16)`, `verify_snapshot_data<R: DigestLeaf>(&SnapshotData<R>, height, &TrustedSnapshot)` (height label, then identity hash, then accumulator) and `node_identities_at(client, height) -> Result<BTreeMap<NodeId, ed25519::PublicKey>, NyxdError>`, moved from the geolocation client's `node_identities_at` with its error log
- [x] 3.3 Replace `TrustedSnapshot`'s private `from_snapshot` with a public `From<DigestSnapshot>`, and add `AttestedTrustAnchor::verified_records(height) -> Result<VerifiedRecords<S::Record>, VerificationError>` where `S::Record: DigestLeaf`: `trusted_snapshot(height)`, then each source's `snapshot_data(height)` in turn checked with `verify_snapshot_data`, returning the first that passes with the trusted accumulator, else the last failure, else `NoQuorumSnapshotForHeight`
- [x] 3.4 Tests through a test-local record type implementing `DigestLeaf` and `MockAttestationSource` over it: the fold is order-independent and an empty set is `LtHash16::new()`; an added, withheld and altered record each fail with `DigestMismatch`; a mislabelled height, a substituted identity map and an extended identity map each fail with their variant; `verified_records` returns the first verifying source, skips a bad source for a good one, returns the last failure when every source fails, and returns the quorum error when no quorum exists for the height
- [x] 3.5 `cargo test -p nym-contract-anchor` and `cargo check -p nym-contract-anchor --features light-client`
- [x] 3.6 The shared read paths (design Decision 2): `VerifiedRecords<R> { height, accumulator, records, node_identities }` with public fields, `verify_chain_read(anchor, client, height, records)` (the pinned read passed unawaited, run after `trusted_digest`) and `verify_offline(&DigestSnapshot, SnapshotData<R>)`, replacing `VerifiedSnapshotData`. Tests: `verify_offline` returns the records with the snapshot's accumulator and rejects a mislabelled height and a substituted identity map; `verify_chain_read`, through a test anchor and a test type implementing `PinnedMixnetQueryClient`, returns the records with the bond identities, rejects records the digest does not commit, and runs no read when the anchor fails

## 4. Move the directory client (`common/nym-directory-client`)

- [x] 4.1 Delete `DirectoryClientError` and return the shared `VerificationError` (design Decision 4), re-exporting the anchor crate's `error` module beside `anchor` and `proof`: the identity-hash and subset hash checks raise `NodeIdentitiesMismatch` and `DigestMismatch`, the decode failures `MalformedEntry` and `MalformedSubset`; address prechecks go except in the single-entry reads, which return `NyxdError::unavailable_contract_address`; nym-api's refresher names `VerificationError`
- [x] 4.2 In `verify.rs` drop `recompute_accumulator` for the shared one, check records through `verify_records`, and split building `VerifiedDirectory` from the checks so the attested read can build from verified data; `verify_directory` and `verify_directory_offline` keep their signatures
- [x] 4.3 Make `AttestedDirectoryExt::verified_directory` call `verified_snapshot_data` and build with its trusted accumulator, deleting `verify_directory_data`, and replace `DirectoryClient::all_node_identities_at` with the shared `node_identities_at`
- [x] 4.4 `cargo test -p nym-directory-client`, editing tests only where they import a moved function or assert an error now under `Verification`, then `cargo check -p nym-api -p nyx-checkpoint-updater`
- [x] 4.5 Onto the shared read paths (design Decisions 2 and 6): `From<VerifiedRecords<DirectoryEntryRecord>>` for `VerifiedDirectory` and `VerifiedDirectoryWithIdentities`, `DirectoryClient::verified_directory` as `verify_chain_read` over `get_all_directory_entries_at_height` (signature unchanged), `DirectoryClient::trusted_app_hash` replaced by an `anchor()` getter beside `client()` (nym-api's refresher calls `anchor().trusted_app_hash(height)`), and `verify_directory`, `verify_directory_offline` and `AttestedDirectoryExt` (`attested_directory.rs`) deleted. Tests: the entry tamper and signature tests stay; the authorship test goes through `verify_offline`; new tests that the result with identities keeps the raw records and identity hash, and that the attested read builds the same directory as the offline one; the tests repeating shared checks are dropped

## 5. Move the geolocation client and give it the attested read (`common/nym-geolocation-client`)

- [x] 5.1 Delete `GeolocationClientError` and return the shared `VerificationError`, dropping the address prechecks (the pinned reads report a missing address)
- [x] 5.2 In `verify.rs` drop `recompute_accumulator` and `verify_records_against_digest` for the shared steps, keeping `verify_records` and `verify_geolocation_offline` with their signatures; in `client.rs` replace `node_identities_at` with the shared one
- [x] 5.3 Add `attested.rs` with `AttestedGeolocationExt::verified_geolocation(height)` for `AttestedTrustAnchor<S>` where `S: AttestationSource<Record = GeolocationRecord> + Sync`: `verified_snapshot_data`, then `VerifiedGeolocation::from_verified_records` with the served identity map
- [x] 5.4 Tests through `MockAttestationSource<GeolocationRecord>`: the happy path with a self-declaration attributed against the attested identities, a bad source skipped for a good one, and every source tampered failing closed
- [x] 5.5 `cargo test -p nym-geolocation-client`, editing tests only where they import a moved function, called `verify_records_against_digest` (now through `verify_records`) or assert an error now under `Verification`, then `cargo check -p nym-node-status-api`
- [x] 5.6 Onto the shared read paths (design Decisions 2 and 6): `From<VerifiedRecords<GeolocationRecord>>` for `VerifiedGeolocation`, `GeolocationClient::verified_geolocation` as `verify_chain_read` over `get_all_geolocation_records_at_height` (signature unchanged), an `anchor()` getter beside `client()` for consistency with the other clients, and `verify_records`, `verify_geolocation_offline` and `AttestedGeolocationExt` (`attested.rs`) deleted. Tests: the stale `checked_at`, substituted whitelist, tampered set, unknown payload version, multi-page, offline attribution and substituted identity map tests stay, through `verify_offline`; a new test that the attested read builds the same result as the offline one replaces the attested module's three; the tests repeating shared checks are dropped

## 6. Performance crate scaffold (`common/nym-performance-client`)

- [x] 6.1 Create the crate with a `Cargo.toml` modelled on `nym-geolocation-client`'s (no `time`, no payload feature), depending on `nym-performance-contract-common`, `nym-contract-anchor`, `nym-contract-attestation`, `nym-lthash`, `nym-crypto` (`asymmetric`) and `nym-validator-client`, with `nym-contract-attestation` (`mock`), `nym-test-utils` and `tokio` as dev-dependencies. Add it to the root `members` and `[workspace.dependencies]` beside `nym-geolocation-client`
- [x] 6.2 `lib.rs` with a short crate doc (whole-set verification, the routing view, anchors from `nym-contract-anchor`, which also provides the `VerificationError` every read returns); no error module of its own

## 7. Performance verification (`verify.rs`)

- [x] 7.1 `VerifiedPerformance { height, accumulator, entries: BTreeMap<NodeId, LatestPerformance>, node_identities }` and `From<VerifiedRecords<LatestPerformance>>` for it, keying the entries by node
- [x] 7.2 Tests through `verify_offline`, with keys from `nym_test_utils` helpers: a consistent snapshot and data build the result; a withheld entry, an added entry, a score altered from `0.85` to `0.8549`, and an entry moved to another epoch, each fail with `DigestMismatch`; a substituted identity map fails with `NodeIdentitiesMismatch`; an entry with `score: None` verifies and is kept; bonded nodes without an entry are carried in `node_identities`; an empty enumeration verifies against the empty accumulator

## 8. Routing view (`routing.rs`)

- [ ] 8.1 `RoutingView { height, nodes: BTreeMap<NodeId, Option<LatestPerformance>> }` and `VerifiedPerformance::routing_view()`: one entry per node in `node_identities`, with its committed entry or `None`
- [ ] 8.2 Tests: a measured bonded node carries its score and source epoch; a bonded node never measured maps to `None`; a committed entry whose node is not bonded is left out of the view and kept in `entries`; an unscoreable entry stays in the view; an entry from an arbitrarily old epoch is kept unchanged; the view carries the verified height

## 9. RPC-backed client (`client.rs`)

- [ ] 9.1 `PerformanceClient<A, C>` with `new`, `client()`, `anchor()` (as on the directory and geolocation clients), and `verified_performance(height) -> Result<VerifiedPerformance, VerificationError>`: `verify_chain_read` over `get_all_latest_performance_at_height(height)`, then `From`
- [ ] 9.2 Document on the type that its anchor is built with the performance contract's address and the bare item key `storage_keys::DIGEST_STATE.as_bytes()`, never the prefixed raw key

## 10. Performance attested read

- [ ] 10.1 A test through `MockAttestationSource<LatestPerformance>` that `VerifiedPerformance::from(anchor.verified_records(height))` is the result built through `verify_offline` from the same data, routing view included (the source loop itself is tested in 3.4)

## 11. Final verification and hand-off

- [ ] 11.1 `cargo fmt` in both workspaces. In `contracts`: `cargo test -p nym-directory-contract -p nym-geolocation-contract -p nym-performance-contract`. In the root: `cargo test -p nym-contract-anchor -p nym-directory-client -p nym-geolocation-client -p nym-performance-client -p nym-directory-contract-common -p nym-geolocation-contract-common -p nym-performance-contract-common`, and `cargo check -p nym-validator-client -p nym-api -p nym-node-status-api -p nyx-checkpoint-updater --all-targets`
- [ ] 11.2 Read the full `git diff` once:
  - every spec scenario has a test, except the pinned-read scenarios, which the blanket implementations leave to review;
  - every scenario the sibling clients' tests covered stays covered (design Decision 6);
  - nothing is committed, and no `Co-Authored-By` line is added
- [ ] 11.3 Archive the change in the same PR (`openspec archive performance-retrieval-client`, which creates the `performance-retrieval-client` spec, appends the ADDED requirements to `contract-trust-anchor` and `geolocation-retrieval-client`, and replaces the two MODIFIED requirements in `directory-retrieval-client`), replace the generated `## Purpose` placeholder of the new spec, then `git diff openspec/specs` and account for every line
- [ ] 11.4 Draft the PR description in a fenced markdown block
