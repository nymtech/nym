## 1. Mechanical renames, before any new code

Kept as their own commits so the rename churn stays separable from the extraction and from the new crate.

- [x] 1.1 Rename `common/directory-attestation` to `common/nym-contract-attestation`, updating the package name, workspace members and every dependent `Cargo.toml`
- [x] 1.2 Rename `DigestSnapshot::directory_contract` to `contract`, including its use in `digest_snapshot_signing_payload` and `SignedDigestSnapshot::verify`. The signing payload bytes MUST be unchanged: this is a field rename, not a format change, and an existing signature must still verify
- [x] 1.3 Replace `DirectorySnapshotData` with a generic `SnapshotData<R>`, keeping the `serde_as` identity-map encoding in one place, and alias the directory instantiation so nym-api's usage is a type swap
- [x] 1.4 Rename `nym_network_defaults::mainnet::DIRECTORY_ATTESTATION_SOURCES` to `CONTRACT_ATTESTATION_SOURCES` and update `default_trusted_signers`
- [x] 1.5 Run `cargo fmt`, then `cargo check --workspace` and fix every naming fallout, including `nym-api/src/directory/`
- [x] 1.6 Confirm the full workspace test suite passes with no test edits beyond identifier renames

## 2. Extract nym-contract-anchor

- [x] 2.1 Create `common/nym-contract-anchor` with the dependencies `proof.rs` and the anchors actually need, and add it to the workspace
- [x] 2.2 Move `proof.rs` across unchanged, including its offline membership and non-membership fixtures. It has zero directory references, so this should be a pure move
- [x] 2.3 Move the anchor trait and `TrustedDigest`, renaming `DirectoryTrustAnchor` to `TrustAnchor`
- [x] 2.4 Split `DirectoryClientError`: move the proof, quorum, light-client, checkpoint and unavailable-state variants into a core `AnchorError`, and have `DirectoryClientError` wrap it with `#[from]`
- [x] 2.5 Move `anchor/proven.rs`, `anchor/helpers.rs`, `anchor/light_client.rs`, `anchor/attested.rs` and `anchor/checkpoint/` across, along with their tests and fixtures
- [x] 2.6 Move the `light-client` feature gate onto `nym-contract-anchor` and have `nym-directory-client` forward it
- [x] 2.7 Re-export the moved items from `nym-directory-client` so its public surface does not move
- [x] 2.8 Verify the extraction: `cargo test -p nym-directory-client -p nym-contract-anchor` passes with no test edits beyond import paths and renamed identifiers. A test needing a real edit means the move was not mechanical and must be resolved here

Gate result for 2.8: 74 tests before, 74 after (48 in `nym-contract-anchor`, 26 in
`nym-directory-client`). Three deviations beyond import paths and renamed identifiers,
all structural consequences of decisions this change made rather than behaviour drift:

1. `subset.rs`'s two quorum assertions became
   `DirectoryClientError::Anchor(AnchorError::QuorumNotReached { .. })`, since the spec
   forbids redefining the variant. `needed`/`agreed` values are unchanged.
2. Three `LightClientAnchor::new` call sites in `light_client.rs` tests gained the
   digest-key argument (a consequence of folding 3.3 in).
3. `attested.rs`'s `mod verified_directory` test module moved wholesale to
   `nym-directory-client/src/attested_directory.rs`, trading `use super::*` for explicit
   imports. All three test bodies and assertions are byte-identical.

## 3. Generalise the anchors over the contract

3.1-3.3 were pulled forward into the section-2 batch: they are hard prerequisites for the
extraction, not follow-ups. `anchor/helpers.rs` reached into `crate::key::digest_state_key`,
so the anchor tree could not move until the digest key became a parameter.

- [x] 3.1 Change `get_trusted_directory_digest` to take the contract address and the raw digest storage key as parameters, make it `pub`, and rename it accordingly
- [x] 3.2 Hoist the directory's `digest_state_key` into a generic helper over `contract_storage_key`, keeping `nym-directory-client`'s own function as a thin delegate
- [x] 3.3 Rename the `directory_contract` field on `ProvenTrustAnchor`, `LightClientAnchor` and `AttestedTrustAnchor` to `contract`, and thread the digest storage key through their constructors
- [ ] 3.4 Add a test constructing one anchor type for two different contract addresses and confirming each resolves its own digest
- [ ] 3.5 Add a test that an `AttestedTrustAnchor` constructed for contract A rejects a validly signed snapshot naming contract B, before quorum counting

## 4. Height-pinned geolocation pagination

- [ ] 4.1 Add a height-pinned variant of the geolocation record pagination to `nym-validator-client`, so every page is read at one height rather than through `collect_paged!`
- [ ] 4.2 Document on `get_all_geolocation_records` that it pins no height and must not feed a digest comparison

## 5. nym-geolocation-client: scaffold and keys

- [ ] 5.1 Create `common/nym-geolocation-client`, depending on `nym-contract-anchor`, `nym-contract-attestation`, `nym-geolocation-contract-common` and `nym-validator-client`
- [ ] 5.2 Build the digest storage key for the geolocation contract, asserting it resolves to the raw `digest_state` bytes appended to the contract prefix with no length prefix or namespacing
- [ ] 5.3 Capture a live ICS23 membership proof of the geolocation `digest_state` key from sandbox, and a non-membership proof of an absent key, and freeze both as offline fixtures with their `app_hash`
- [ ] 5.4 Add an offline test that the frozen membership fixture verifies, that a tampered value is rejected, and that a wrong `app_hash` is rejected
- [ ] 5.5 Add a test that a proven-absent digest key yields the empty accumulator rather than an error

## 6. Verify core

- [ ] 6.1 Recompute the accumulator over `GeolocationRecord::digest_leaf()` for a record set and compare it to a `TrustedDigest`
- [ ] 6.2 Extract the verified whitelist from the same record set, and resolve each measured entry's agent against it
- [ ] 6.3 Verify self-declared entries' ed25519 attestations against the subject node's identity key
- [ ] 6.4 Define the returned shape: per-subject measured entries with named `method`, `agent`, `checked_at`, `location` and `authority` fields, plus the self-declared and override slots, and the verified whitelist alongside
- [ ] 6.5 Decode payloads separately from verification, returning raw bytes and no decoded location where the version is unknown
- [ ] 6.6 Test: a tampered record set fails the recompute and returns no records
- [ ] 6.7 Test: an entry whose agent is absent from the verified whitelist is returned marked de-authorised, not dropped
- [ ] 6.8 Test: a substituted whitelist fails the recompute
- [ ] 6.9 Test: a record carrying an unknown payload version still verifies, is returned with its raw payload, and does not remove its subject from the set. This is the regression guard for the multi-address version 2 payload

## 7. Client read and the attested path

- [ ] 7.1 Implement the whole-set verified read: trusted digest at `H`, height-pinned record pagination at `H`, recompute, compare, group
- [ ] 7.2 Source node identities from the chain in the RPC-backed path, mirroring how the directory client reads bonds at `H`
- [ ] 7.3 Implement the geolocation attestation source over the generic snapshot types, with a mock for tests
- [ ] 7.4 Implement the offline verification path: verify records against the attested accumulator and identities against `node_identities_hash`, with no chain connection
- [ ] 7.5 Implement the HTTP transport against the parallel geolocation route tree this change's spec fixes
- [ ] 7.6 Test: a verified read succeeds across a multi-page record set while a write commits at a later height
- [ ] 7.7 Test: the offline path fails closed when either recomputed hash differs from the attested value

## 8. Resolution policy

- [ ] 8.1 Define the policy seam, selecting one of a subject's verified entries and returning it with its provenance
- [ ] 8.2 Implement the default policy: override, then measured, then self-declared, freshest `checked_at` among measured, with maximum age as a parameter
- [ ] 8.3 Test each precedence branch, the freshest-wins tie-break, the age bound making an entry ineligible, and a caller-supplied policy replacing the default

## 9. Verification and documentation

- [ ] 9.1 `cargo fmt`, then `cargo check --workspace` and `cargo test` for the three touched crates plus `nym-api`
- [ ] 9.2 Confirm no contracts-workspace member has gained a dependency on the payload feature, since `Location` carries `f64` and CosmWasm rejects float instructions at upload
- [ ] 9.3 Note in `docs/geolocation/node-status-api-migration.md` that the verifying client now exists, and which of that note's open decisions remain the migration's own
- [ ] 9.4 Run `openspec validate geolocation-retrieval-client --strict`
