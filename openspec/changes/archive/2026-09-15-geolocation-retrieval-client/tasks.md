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
- [x] 3.4 Add a test constructing one anchor type for two different contract addresses and confirming each resolves its own digest
- [x] 3.5 Add a test that an `AttestedTrustAnchor` constructed for contract A rejects a validly signed snapshot naming contract B, before quorum counting

3.4 uses `AttestedTrustAnchor`, which resolves its digest from signed snapshots and so needs
no RPC. The proven path could not be used: `MockRpcClient` mocks only `commit`/`validators`
and has no ABCI-query support. Its key derivation is covered directly instead, by two
`helpers.rs` tests asserting the digest key varies with both contract and item key, and that
the item key is appended to the contract prefix verbatim (the invariant task 5.2 relies on).

3.5 asserts `QuorumNotReached { needed: 2, agreed: 0 }`. The `agreed: 0` is the substance:
the wrong-contract snapshots were filtered before grouping, so they never counted toward
quorum, which is what makes a per-domain signing-payload tag unnecessary.

## 4. Height-pinned geolocation pagination

- [x] 4.1 Add a height-pinned variant of the geolocation record pagination to `nym-validator-client`, so every page is read at one height rather than through `collect_paged!`
- [x] 4.2 Document on `get_all_geolocation_records` that it pins no height and must not feed a digest comparison

Landed as `PinnedGeolocationQueryClient::get_all_geolocation_records_at_height`, a separate
extension trait bounded on `CosmWasmClient + NymContractsProvider` rather than a new required
method on `GeolocationQueryClient` - the latter would have forced a matching delegation into
`nym-geolocator`'s hand-written impl for no benefit.

Beyond the task: the three `query_contract_smart_at_height` call sites that were open-coded
in `nym-directory-client/src/client.rs` now go through matching `PinnedDirectoryQueryClient`
and `PinnedMixnetQueryClient` traits, so query-message construction lives in
`nym-validator-client` with every other contract query. All three traits are deliberately
partial - only the queries a verifying client needs pinned. The typed
`UnavailableDirectoryContract` / `UnavailableMixnetContract` errors are preserved by an
explicit address check before each call, since the query layer would otherwise report a
missing address as a generic chain-query failure.

## 5. nym-geolocation-client: scaffold and keys

- [x] 5.1 Create `common/nym-geolocation-client`, depending on `nym-contract-anchor`, `nym-contract-attestation`, `nym-geolocation-contract-common` and `nym-validator-client`
- [x] 5.2 Build the digest storage key for the geolocation contract, asserting it resolves to the raw `digest_state` bytes appended to the contract prefix with no length prefix or namespacing

5.1 enables `nym-geolocation-contract-common/payload` - needed from 6.5 onward for
`try_decode_v1`, and safe because the client is not a contracts-workspace member (verified:
nothing under `contracts/` or `common/cosmwasm-smart-contracts/` depends on it, so the `f64`
codec cannot reach a wasm upload). This is the check task 9.2 asks for, made structural.

5.2 lives in `key.rs` as `digest_item_key()` / `digest_state_key()`, mirroring
`nym-directory-client`. Its test builds the expected key from wasmd's layout by hand
(`0x03 || canonical_addr || b"digest_state"`) rather than by calling the helper again, so it
would actually catch a length prefix or namespacing creeping in.
- [x] 5.3 Capture a live ICS23 membership proof of the geolocation `digest_state` key from sandbox, and a non-membership proof of an absent key, and freeze both as offline fixtures with their `app_hash`
- [x] 5.4 Add an offline test that the frozen membership fixture verifies, that a tampered value is rejected, and that a wrong `app_hash` is rejected
- [x] 5.5 Add a test that a proven-absent digest key yields the empty accumulator rather than an error

Captured at sandbox height 18735040 against
`n1yn0mxzlx032kf303rjwut4lsynr3m0falayn8ztf76sxenpfxn9q38ux9p`, both verifying against
`header[18735041].app_hash`. Frozen in `src/fixtures.rs`; sandbox prunes state within a few
hundred blocks, so they cannot be re-fetched at that height.

5.5 could not be captured directly: sandbox's geolocation contract already has a digest item,
so its absence does not exist to prove. The non-membership fixture proves a key the contract
never writes, with the digest key passed as the parameter it now is - the same shape as a read
against a contract that has not yet written one.

Made `proven_contract_digest` public in `nym-contract-anchor` so a client holding a proof it
did not fetch itself (a frozen fixture, or a producer-relayed response) can run the same
verification. Mocking `TendermintRpcClientExt` was the alternative and would have meant
implementing the whole `TendermintRpcClient` surface for one test.

## 6. Verify core

- [x] 6.1 Recompute the accumulator over `GeolocationRecord::digest_leaf()` for a record set and compare it to a `TrustedDigest`

`verify.rs`: `recompute_accumulator` plus `verify_records_against_digest`, which fails closed
with `GeolocationClientError::DigestMismatch` and returns no records. The fold is identical to
the contract's own `assert_digest_is_refold` (`contracts/geolocation/src/storage.rs:498`), so
client and contract agree by construction rather than by coincidence. `error.rs` wraps
`AnchorError` per the `contract-trust-anchor` error-taxonomy requirement.

Task 6.8 (substituted whitelist fails the recompute) is already covered here, since both entry
classes fold into the one accumulator; it will be re-checked when 6.2 extracts the whitelist.
- [x] 6.2 Extract the verified whitelist from the same record set, and resolve each measured entry's agent against it

`whitelist.rs`: `VerifiedWhitelist::from_verified_records` plus `MeasurementAuthority`
(`Authorised` / `DeAuthorised`). Named `from_verified_records` rather than `from_records`
because reading an authorisation set out of an unverified set authorises nothing - an agent
could simply have been omitted - and the name is the only place that constraint can be stated.

Two cases beyond the task text: an agent present but with `can_measure` withdrawn resolves
`DeAuthorised` (same class of event as removal - the measurement stays genuine, the agent may
no longer measure), and sources naming no agent (self-declared, admin override) resolve to
`None` rather than `DeAuthorised`, since the whitelist has no opinion on entries authorised by
a subject signature or the admin role.
- [x] 6.3 Verify self-declared entries' ed25519 attestations against the subject node's identity key

`attestation.rs`: `self_declaration_status(record, identities) -> Option<AttestationStatus>`,
verifying over `LocationPayload::self_declaration_signing_payload` - the bytes the stored
artifact produces, never a re-serialisation.

Four outcomes rather than a bool, because collapsing them would misreport real states:
`UnknownSubjectIdentity` (an unbonded node is absent, not fraudulent) is kept distinct from
`InvalidSignature`, and `MissingAttestation` flags a self-declared entry the contract could
not have produced. Non-self-declared sources return `None`, matching `source_authority`:
a measurement has no subject signature by design, so it has no status rather than a failing one.

Tests pin every field the payload binds - content, version, `declared_at` and `node_id` - since
each is signed to close a specific substitution (notably `version`, which stops a relayer
storing v1-signed content as v2 and thereby choosing which format consumers believe it is).
- [x] 6.4 Define the returned shape: per-subject measured entries with named `method`, `agent`, `checked_at`, `location` and `authority` fields, plus the self-declared and override slots, and the verified whitelist alongside

`verified.rs`: `VerifiedGeolocation { height, subjects, whitelist }` over
`SubjectEntries { measured, self_declared, overridden }`, built by
`VerifiedGeolocation::from_verified_records`. Nothing here picks a winner between slots -
that is section 8's job, deliberately separate.

Two corrections from review:

Timestamps are `OffsetDateTime`, not raw unix `u64`. Nothing breaks, because the recompute runs
on `GeolocationRecord::digest_leaf()` over the contract's own fields and never on this shape;
and section 8 compares `checked_at` against a maximum age, which wants a real timestamp and a
`Duration`. Conversion clamps rather than panics, though the contract bounds every timestamp by
block time so an out-of-range value cannot be written.

Every self-declared entry carries an attestation, guaranteed twice: `relay` is the only path
that writes `Source::SelfDeclared` and goes through `into_entry`, which always attaches one
(`types.rs:403`); and `digest_leaf` commits `declared_at` and the signature (`types.rs:730`),
so a stripped attestation fails the recompute. The `MissingAttestation` status is therefore
gone, and `declared_at` is non-optional inside a `VerifiedAttestation`. One `Option` now
expresses "no attestation to check" where previously a spurious enum variant and a parallel
`Option<u64>` could disagree.
- [x] 6.5 Decode payloads separately from verification, returning raw bytes and no decoded location where the version is unknown

`DecodedLocation` in `verified.rs`, reached via `decoded_location()` on each of the three entry
types. Nothing filters on it: the raw `LocationPayload` is always present on the entry, and
decoding is a separate call.

Dispatches on the payload's own `version` field and calls `try_decode_v1` only for version 1,
rather than handing an unknown version to `try_decode_v1` to reject. That keeps two different
situations apart - `UnsupportedVersion` (benign: this build is behind, a newer client reads it)
versus `Malformed` (anomalous: the contract stores content opaquely and checks only its size,
so nothing on the write path would have caught it) - and makes the match the obvious place a
version 2 arm goes.
- [x] 6.6 Test: a tampered record set fails the recompute and returns no records
- [x] 6.7 Test: an entry whose agent is absent from the verified whitelist is returned marked de-authorised, not dropped
- [x] 6.8 Test: a substituted whitelist fails the recompute
- [x] 6.9 Test: a record carrying an unknown payload version still verifies, is returned with its raw payload, and does not remove its subject from the set. This is the regression guard for the multi-address version 2 payload

6.6 needed a composed entry point to be assertable as written: verification and grouping were
separately reachable, so nothing ordered them. Added `verify::verify_records`, which checks the
digest and groups only on success. The test contrasts the two paths - grouping alone returns
the rogue subject quite happily, which is precisely what checking first prevents.

6.7 uses a whitelist that exists but omits the writing agent, rather than no whitelist at all:
that is the shape removing an agent actually leaves behind. Asserts both that the entry
survives with its data intact and that an authorised sibling is unaffected.

6.9 goes through `verify_records` so it asserts the part that matters - that an unknown version
*verifies* - not only that the shape survives grouping. A v1 entry alongside it still decodes,
so the unknown version does not poison the rest of the set.

## 7. Client read and the attested path

- [x] 7.1 Implement the whole-set verified read: trusted digest at `H`, height-pinned record pagination at `H`, recompute, compare, group
- [x] 7.2 Source node identities from the chain in the RPC-backed path, mirroring how the directory client reads bonds at `H`

`client.rs`: `GeolocationClient<A, C>::verified_geolocation(height)`, composing the anchor's
`trusted_digest` with `get_all_geolocation_records_at_height` and the bond read, then
`verify_records`. Identities are read at the same height as the records, so a node that bonded
or unbonded afterwards cannot change whether an older self-declaration attributes.

7.2 mirrors rather than shares the bond-to-identity parse: the expensive half (height-pinned
pagination) is already shared via `PinnedMixnetQueryClient`, and sharing the remaining ~15-line
parse would mean adding `nym-crypto` to `nym-validator-client`, which has no crypto dependency
today.
- [x] 7.3 Implement the geolocation attestation source over the generic snapshot types, with a mock for tests

`AttestationSource` gained an associated `Record` type and `directory_data` became
`snapshot_data() -> SnapshotData<Self::Record>`. One trait, no extension traits, no type
parameter on the trait or on the anchor - which never calls it and has no use for one.
`NymApiGeolocationSource` implements the same trait with `Record = GeolocationRecord`;
`MockAttestationSource<R = DirectoryEntryRecord>` is generic, defaulting to the directory's so
suites that only exercise snapshots name nothing.

An associated type, not a generic method. A generic `snapshot_data<R>` was tried first and is
wrong: `get_directory_snapshot_data` composes `get_all_directory_entries`, which returns
concrete `Vec<DirectoryEntryRecord>`, so the directory transport can only ever produce one
record type. Satisfying `snapshot_data<R>` for all `R` would have meant a serde round-trip that
lies about what the source can do. The associated type states the truth instead: a source is
scoped to one contract, therefore to one record type. `AttestedDirectoryExt`'s bound is now
`S: AttestationSource<Record = DirectoryEntryRecord>`, so the requirement is in the signature
rather than discovered at runtime.
- [x] 7.4 Implement the offline verification path: verify records against the attested accumulator and identities against `node_identities_hash`, with no chain connection

`verify::verify_geolocation_offline`, taking the whole `DigestSnapshot` rather than its two
hashes separately - it is the unit a quorum agreed on, and splitting it invites passing values
from two different heights.
- [x] 7.5 Implement the HTTP transport against the parallel geolocation route tree this change's spec fixes

STUBBED by decision: the nym-api producer that serves these routes is a separate change, so
`http.rs` fixes the shape (`NymApiGeolocationSource` over `latest_snapshot` / `snapshot_at` /
`snapshot_data`) and every method returns `GeolocationClientError::NotImplemented` naming what
is missing. Failing loudly rather than returning an empty set matters here: an empty set is
indistinguishable from a verified one with no entries.

The route tree the spec fixes is recorded in the module docs, including the producer
requirement that is not discoverable from either tree alone - one nym-api serving both
contracts must use the same cadence and retained window, so a single height serves a consumer
joining them.
- [x] 7.6 Test: a verified read succeeds across a multi-page record set while a write commits at a later height
- [x] 7.7 Test: the offline path fails closed when either recomputed hash differs from the attested value

7.6: `a_multi_page_set_verifies_while_a_later_write_does_not` covers the property - a 251-record
set (well past the contract's 100-record page limit) read at one height verifies as one unit,
and the same enumeration with a later write spliced in fails as `DigestMismatch`, the trap being
that a pagination bug is then indistinguishable from tampering.

It deliberately does not drive `GeolocationClient::verified_geolocation`. Doing so would test
that the query layer passes the height on every request, which is `nym-validator-client`'s
internal behaviour rather than this client's contract - and it is untestable anyway without a
27-method `CosmWasmClient` mock, since `PinnedGeolocationQueryClient` is blanket-implemented and
coherence forbids a second impl for a mock type.

7.7 covers both halves, and the identity-map half is the one that matters: the records are
genuine and recompute correctly, and only the identity map has been swapped. Without that check
a producer could serve real records alongside its own keys and every self-declaration would
attribute to whatever it chose.

## 8. Resolution policy

- [x] 8.1 Define the policy seam, selecting one of a subject's verified entries and returning it with its provenance
- [x] 8.2 Implement the default policy: override, then measured, then self-declared, **the country the most measurements agree on** then freshest among those, **with no maximum age**
- [x] 8.3 Test each precedence branch, the agreement tally and its tie-break, readability gating selection, and a caller-supplied policy replacing the default

8.2 and 8.3 were re-specified during implementation; design.md's "Policy is a seam with a
default" section is updated to match. Two reversals:

**Maximum age dropped, not parameterised.** Stale data beats no data - ageing a measurement out
drops a subject to a weaker source, or to nothing, because no agent swept it recently, which is
a fact about the sweep rather than the subject. This also removed the `now` argument from the
seam, since nothing left in it depends on the clock.

**Plurality by country, not freshest-wins.** One fresh outlier should not overturn ten older
measurements that agree. The original rejection of majority voting ("needs three agents, there
is one") does not bite, because the tally is over measurements rather than agents.

Consequence, recorded because it is the one place a payload version affects more than display:
selection now needs a decoded country, so decoding moved to retrieval and is stored per entry.
Nothing filters on it - an unreadable entry stays in the set - but it cannot win its slot, so
precedence falls through. An unreadable *override* is the exception and resolves to nothing,
since falling through would serve the value an admin acted to suppress.

## 9. Verification and documentation

- [x] 9.1 `cargo fmt`, then `cargo check --workspace` and `cargo test` for the three touched crates plus `nym-api`

`cargo fmt --all --check` exit 0. `cargo check --workspace --exclude nym-data-observatory` exit
0, zero errors and zero unused-dependency warnings. Touched crates plus `nym-api`: 296 passed.
Full workspace suite: 356 suites, 1793 passed, 0 failed, 73 ignored - against a pre-change
baseline of 352/1717, the growth being this change's four new suites and its tests.

`nym-data-observatory` is excluded throughout: its `sqlx::query!` macros need a live Postgres on
:5432 and it depends on none of the crates this change touches.

Docs build clean for the four crates bar one warning, `Vec<u8>` read as an HTML tag in
`verify.rs` - byte-identical on develop and in a file this change never touched. Two other
pre-existing warnings in `nym-contract-attestation` were fixed, since that crate was renamed
here: a doc link to `build_and_sign_snapshot`, which does not exist, and one to `SubsetDigest`
from `producer.rs`, which does not import it.
- [x] 9.2 Confirm no contracts-workspace member has gained a dependency on the payload feature, since `Location` carries `f64` and CosmWasm rejects float instructions at upload

Verified mechanically, not by inspection. `cargo tree -p nym-geolocation-contract -e features -i
nym-geolocation-contract-common`, run inside `contracts/`, shows `"default"` as the only feature
edge - and the crate declares `schema`, `payload` and `utoipa` with no default list, so default
is empty.

Structurally it cannot happen either: `contracts/` is a separate workspace with its own
`[workspace]` and `resolver = "2"`, and the root workspace's members include no `contracts/`
paths, so cargo feature unification cannot cross between them. `payload` is enabled only by
main-workspace, off-chain members: `nym-node-status-api`, `nym-geolocator`,
`nym-geolocator-requests`, and now `nym-geolocation-client`.
- [x] 9.3 Note in `docs/geolocation/node-status-api-migration.md` that the verifying client now exists, and which of that note's open decisions remain the migration's own

Two of the note's three open decisions are now closed by this change and one survives.

Closed: **which entry to serve** (there is a stated default, and a seam to replace it, so the
choice lives visibly in that service's code rather than falling out of an iteration order) and
**whether to verify** (`GeolocationClient::verified_geolocation`, with the offline path
available but its producer still unbuilt).

Still the migration's own: **what to serve when there is no entry**, which is where the
cold-start cliff moves to. Added a warning it did not previously have: `resolve` returns `None`
for three situations the API must tell apart - no entries, policy declined, and a payload
version this build cannot read - and the third is an alarm rather than a missing-data case that
a country filter must not silently treat as "no location".
- [x] 9.4 Run `openspec validate geolocation-retrieval-client --strict`
