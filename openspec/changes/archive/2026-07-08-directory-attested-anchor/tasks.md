## 1. Canonical attestation encoding

Lives in `nym-directory-client` itself, not a contract-common crate: neither function is
ever consumed by a contract, only by this crate (verifying) and the not-yet-built nym-api
producer (signing) - the same two-off-chain-peers pairing `recompute_accumulator` already
serves from `verify.rs`. Placement revisited mid-implementation (see `design.md` D3); the
producer, when it lands, can decide then whether to depend on this crate or extract a
shared piece, once its real constraints are known.

- [x] 1.1 Add
  `digest_snapshot_signing_payload(chain_id: &str, contract: &AccountId, height: Height, app_hash: &AppHash, accumulator: &LtHash16, node_identities_hash: &[u8; 32]) -> Vec<u8>`
  to `common/nym-directory-client/src/anchor/attested.rs`, with its own local length-prefixing helper and a
  domain-separation tag (so a snapshot signature can never collide with a `node_signing_payload` signature)
- [x] 1.2 N/A - not a shared crate, no re-export surface; `pub(crate)` within `nym-directory-client`, consumed by
  `attested.rs` itself (task 2/3)
- [x] 1.3 Unit tests: payload is deterministic and field-sensitive (differs on any of chain-id / contract / height /
  app_hash / accumulator / node_identities_hash); length-prefix framing disambiguates adjacent variable-length fields;
  domain tag differs from a representative `node_signing_payload` output
- [x] 1.4 Add `node_identities_hash` (sorted, fixed-width-per-record, plain `blake3` hash - not an LtHash accumulator,
  since it is recomputed fresh each time rather than incrementally updated) to
  `common/nym-directory-client/src/verify.rs`, next to `recompute_accumulator`
- [x] 1.5 Unit tests for `node_identities_hash`: deterministic, sensitive to any `(node_id, identity)` or membership
  change, order-independent (sorts internally so caller iteration order does not matter)

## 2. Attestation types and transport trait (client crate)

`trusted_signers`/`trusted` use `HashSet<ed25519::PublicKey>`, not the originally-sketched
`BTreeSet`: `ed25519::PublicKey` derives `Hash` but not `Ord` (confirmed in
`common/crypto/src/asymmetric/ed25519/mod.rs`), and a signer allowlist has no ordering
requirement to justify adding one - membership testing is all quorum counting needs.

- [x] 2.1 Define
  `DigestSnapshot { chain_id: chain::Id, directory_contract: AccountId, height: Height, app_hash: AppHash, accumulator: LtHash16, node_identities_hash: [u8; 32] }` (
  richer types than the original sketch - all confirmed to have serde support: `chain::Id`/`AccountId`/`Height` via
  hand-written impls, `AppHash` via `cosmrs::tendermint::serializers::apphash`, `LtHash16` via a new `serde` feature on
  `nym-lthash`, see task 1's follow-up) and
  `SignedDigestSnapshot { snapshot: DigestSnapshot, signer: ed25519::PublicKey, signature: Vec<u8> }` in
  `src/anchor/attested.rs` (both `Serialize`/`Deserialize`; signature kept as bytes so malformed data is a verification
  failure, not a decode panic - mirrors `DirectoryNodeEntry`)
- [x] 2.2 Implement
  `SignedDigestSnapshot::verify(&self, trusted: &HashSet<ed25519::PublicKey>, chain_id: &str, contract: &AccountId) -> bool`:
  signer in trusted set AND chain-id + contract match AND ed25519 signature verifies over
  `digest_snapshot_signing_payload(..)` (mirror `node_signature_verifies`); unit tests: valid attestation accepted,
  untrusted signer / mismatched chain-id / mismatched contract / forged or malformed signature all rejected
- [x] 2.3 Define
  `#[async_trait] pub trait AttestationSource { fn identity(&self) -> ed25519::PublicKey; async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError>; async fn snapshot_at(&self, height: Height) -> Result<SignedDigestSnapshot, DirectoryClientError>; }` (
  `identity()` added mid-implementation, ahead of the original sketch - a sync, no-network way for the anchor to
  recognize which source produced a given attestation, used by `refresh()`, task 3.5, to avoid re-querying the seed's
  own source)

## 3. AttestedTrustAnchor core

`refresh()`'s flow was revised mid-implementation (see `design.md` D6): rather than
comparing every source's independently-reported "latest" (which can split across a
cadence boundary if sources are not perfectly in lockstep), it seeds a height from the
first successful `latest_snapshot()` response, then asks every source's `snapshot_at`
that same height and reaches quorum over all of it. The seed is untrusted at that point -
just a discovery hint - so a lying seed only wastes a round-trip, never a false accept.

- [x] 3.1 Define `TrustedSnapshot { app_hash: AppHash, accumulator: LtHash16, node_identities_hash: [u8; 32] }` and
  private `AttestedTrustAnchorState { snapshots: BTreeMap<Height, TrustedSnapshot>, latest: Option<Height> }`
- [x] 3.2 Define
  `AttestedTrustAnchor<S> { sources: Vec<S>, trusted_signers: HashSet<ed25519::PublicKey>, quorum: usize, chain_id: chain::Id, directory_contract: AccountId, state: Mutex<AttestedTrustAnchorState> }` (
  `chain_id` typed as `chain::Id`, matching `DigestSnapshot` and `verify`, not the originally-sketched `String`)
- [x] 3.3 Implement `new(sources, trusted_signers, quorum, chain_id, directory_contract)` validating
  `1 <= quorum <= trusted_signers.len()` (error otherwise)
- [x] 3.4 Implement a private
  `reach_quorum(&self, candidates: Vec<SignedDigestSnapshot>) -> Result<(Height, TrustedSnapshot), DirectoryClientError>`:
  filter to valid attestations (via `verify`), group by `(height, app_hash, accumulator, node_identities_hash)` via a
  `HashMap<DigestSnapshot, HashSet<ed25519::PublicKey>>` keyed on a manual `Hash` impl added to `DigestSnapshot` (not
  the linear scan originally planned, once `LtHash16` gained a `Hash` impl to build it from - see design.md D3/D6),
  count DISTINCT signer keys per group, accept the *first* group (in candidate-arrival order, checked as each candidate
  is folded in, so the result is deterministic regardless of `HashMap` iteration order) reaching `quorum`, else
  `QuorumNotReached { needed, agreed }` (`agreed` = the largest distinct-signer count seen across any single group, via
  an order-independent `max()`)
- [x] 3.5 Implement `refresh(&self) -> Result<Height, DirectoryClientError>`: try sources' `latest_snapshot()` in
  shuffled order, one at a time, taking the first successful response as a height seed `H` (see design.md D6 for an open
  latency concern with this vs. querying concurrently); query every *other* source's (via `identity()`, task 2.3)
  `snapshot_at(H)` concurrently; `reach_quorum` over the seed plus all of those responses; insert into `snapshots`, set
  `latest`, return the agreed height
- [x] 3.6 Implement `latest_snapshot_height(&self) -> Result<Height, DirectoryClientError>`: return cached `latest` or
  call `refresh()`
- [x] 3.7 Implement a private `snapshot_for(&self, height) -> Result<TrustedSnapshot, DirectoryClientError>`: cache hit
  on `height` returns immediately; on miss query all sources' `snapshot_at(height)`, `reach_quorum` (verifying the
  returned height equals the requested one, else `NoQuorumSnapshotForHeight(height)` even if some other height reached
  quorum), cache, return; a height the quorum cannot attest at all returns `NoQuorumSnapshotForHeight(height)`

## 4. Default anchor

Placement revised mid-implementation (see `design.md` D8): the compiled-in constants do
not live in `nym-directory-client` itself, but in `nym-network-defaults::mainnet` -
that crate already hardcodes exactly this shape of thing
(`UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY` / `UPGRADE_MODE_ATTESTATION_URL`), and
`nym-directory-client` was already going to depend on it transitively via
`nym-validator-client`. Quorum is derived from the signer count (a majority) rather
than separately hardcoded, so the list and the threshold cannot drift out of sync.

- [x] 4.1 Add `DirectoryAttestationSourceConst { api_url: &str, identity_ed25519_bs58: &str }` (compiled-in) and
  `DirectoryAttestationSource { api_url: String, identity_ed25519_bs58: String }` (owned, for env-sourced values) to
  `common/network-defaults/src/network.rs`, and a mainnet-only
  `DIRECTORY_ATTESTATION_SOURCES: &[DirectoryAttestationSourceConst]` (`#[cfg(feature = "network")]`) to
  `common/network-defaults/src/mainnet.rs`, holding the 2 currently-known real mainnet identity keys/URLs (a
  commented-out third entry documents the nym-api that cannot be added yet - see the "External prerequisite"). No
  separate quorum constant (see 4.2). Added
  `pub fn default_directory_attestation_sources() -> Vec<DirectoryAttestationSource>`: reads a
  `DIRECTORY_ATTESTATION_SOURCES` JSON env var when `env_configured()` (mirroring `NYM_APIS`/`NYM_VPN_APIS`), else falls
  back to the compiled mainnet list - the `#[cfg(feature = "env")]` block guarding the env path had to be scoped
  *inside* the always-compiled function (not as an unconditional top-level `use crate::env_configured`/
  `use crate::var_names`), since `network.rs` compiles under `feature = "network"` alone and those items require
  `feature = "env"` too; also wired into `mainnet::export_to_env()`/`export_to_env_if_not_set()` so `setup_env()`
  backfills the var from the compiled default for any real binary that didn't have it in its static `.env` file (
  verified end-to-end against `envs/mainnet.env` - no manual duplication needed there).
- [x] 4.2 Add `nym-network-defaults` as a dependency of `nym-directory-client`; implement
  `AttestedTrustAnchor::majority_quorum(signer_count: usize) -> usize` (`signer_count / 2 + 1`, public) and a private
  `default_trusted_signers()` parsing `default_directory_attestation_sources()`'s bs58 keys into
  `HashSet<ed25519::PublicKey>` (`#[allow(clippy::expect_used)]` - the workspace denies `expect_used` by default);
  implement `AttestedTrustAnchor::with_default_anchor(sources, chain_id, directory_contract)` calling
  `new(sources, default_trusted_signers(), majority_quorum(...), chain_id, directory_contract)`
- [x] 4.3 Unit tests: `majority_quorum` matches simple-majority arithmetic for several signer counts;
  `with_default_anchor` produces an anchor whose `trusted_signers`/quorum match the compiled-in default; `new(...)` with
  a caller-supplied set is unaffected by the default

## 5. DirectoryTrustAnchor impl and re-exports

- [x] 5.1 Implement `trusted_app_hash(H)`: `snapshot_for(H)` then return its `app_hash`
- [x] 5.2 Implement `trusted_digest(H)`: `snapshot_for(H)` then return `TrustedDigest { height: H, accumulator }` (no
  ICS23 proof - the quorum attests the accumulator directly)
- [x] 5.3 Expose the snapshot's `node_identities_hash` for a given height via an anchor-specific accessor (
  `pub async fn trusted_node_identities_hash`, not part of the shared `DirectoryTrustAnchor` trait, so
  `ProvenTrustAnchor` / `LightClientAnchor` are untouched)
- [x] 5.4 Re-export `AttestedTrustAnchor`, `AttestationSource`, `SignedDigestSnapshot`, `DigestSnapshot` from
  `src/anchor/mod.rs`
- [x] 5.5 (added) Unit test `directory_trust_anchor_impl_returns_the_attested_values`: `trusted_app_hash`,
  `trusted_digest`, and `trusted_node_identities_hash` all return values matching a quorum-agreed mock snapshot at a
  given height

## 6. Data-source-agnostic whole-directory verification

The `node_identities_hash` check needed a policy decision not fully spelled out by the
original task text: it had to be OPTIONAL on the shared core, because the existing
RPC-backed path (6.3) has no trusted hash to offer at all (it always authenticates node
identities via a live, unproven chain query, unchanged) and must keep behaving exactly
as before. The new decoupled path (6.4) needs the opposite policy - no hash is a hard
error, not a silent skip (per `design.md` D9). Resolved with one shared core taking
`Option<[u8; 32]>` (`Some` checks it, `None` skips it - serving 6.3 unchanged) plus a
second function layering "must be `Some`" on top for 6.4. Both live in `verify.rs`
(pure, synchronous functions over already-fetched data - no anchor, no async, no
`CosmWasmClient` anywhere in the signature).

- [x] 6.1 Extract the body of `DirectoryClient::verified_directory` (digest recompute, per-entry authorship attribution)
  into
  `verify::verify_directory(height, records, node_identities: &HashMap<..>, trusted_accumulator, trusted_node_identities_hash: Option<[u8; 32]>)`,
  needing no `CosmWasmClient` at all
- [x] 6.2 Added the node-identity hash recompute check (using `node_identities_hash` from 1.4) inside
  `verify_directory`, gated on `trusted_node_identities_hash` being `Some`; failing closed with the SAME
  `DigestMismatch` variant used for the accumulator check (task 7 adds no separate "hash mismatch" variant - both are
  the same class of integrity failure)
- [x] 6.3 `DirectoryClient::verified_directory` is now a thin wrapper: fetch records + node identities via `self.client`
  as before, then `verify_directory(.., None)` - the RPC-backed path never had a trusted node-identities hash to check,
  so `None` reproduces its exact prior behavior for every anchor
- [x] 6.4 Added
  `verify::verify_directory_offline(height, records, node_identities, trusted_accumulator, trusted_node_identities_hash: Option<[u8; 32]>)`:
  a free function (not a method, not bound to any anchor type or `DirectoryClient`) layering "the hash must be `Some`"
  over `verify_directory`, returning `NodeIdentitiesHashUnavailable` on `None` - the caller resolves the hash themselves
  from whichever anchor they hold (today, only `AttestedTrustAnchor::trusted_node_identities_hash`) and passes it in,
  rather than this function being generic over anchor types
- [x] 6.5 (added) Unit tests in `verify.rs`: success + authorship attribution; `None` skips the identity check (locks in
  6.3's contract); accumulator mismatch and node-identities-hash mismatch both fail closed with `DigestMismatch`;
  `verify_directory_offline` requires `Some` and returns `NodeIdentitiesHashUnavailable` otherwise, succeeds with a
  matching hash

## 7. Error handling

- [x] 7.1 Add to `DirectoryClientError`: ~~`QuorumNotReached { needed: usize, agreed: usize }`~~,
  ~~`NoQuorumSnapshotForHeight(u64)`~~, ~~`InvalidQuorumConfig { quorum: usize, signers: usize }`~~ (all three added
  early, needed by task 3), `NodeIdentitiesHashUnavailable` (added, used by 6.4). The attestation-transport / decode
  variant is deliberately NOT added: nothing in this change constructs one - the concrete HTTP `AttestationSource` is a
  Non-Goal deferred to the nym-api producer follow-up (see `design.md`), and its real transport/decode failure shape (
  HTTP client error type, wire encoding) is unknown until that change exists. Adding a variant now would be a guess with
  no call site to validate it against; that follow-up should add whatever fits its actual error surface.

## 8. Tests (mock transport)

Most of 8.2-8.11 turned out to already be satisfied by tests written eagerly alongside
tasks 3/5/6, rather than needing new ones written now - cross-referenced below by name
instead of duplicated. 8.1 and 8.7 were genuine gaps (the old `MockSource` had no call
log) and are newly done. 8.12 is flagged, not done - see its note.

- [x] 8.1 Promoted `MockSource` (task 3's own test module) into `MockAttestationSource`: added an `AttestationCallLog` (
  `latest_snapshot: usize`, `snapshot_at: Vec<Height>`) behind `Arc<Mutex<_>>`, `#[derive(Clone)]` on the source
  itself (clones share the same log, mirroring `MockRpcClient`'s pattern) so a test can keep a handle after moving a
  clone into an anchor's `sources`, plus `latest_snapshot_calls()`/`snapshot_at_calls()` accessors. The seeded-`KeyPair`
  signed-snapshot builder already existed (`mock_source`, `signed_snapshot`/`signed_snapshot_with`); reused, not rebuilt
- [x] 8.2 Covered by `reach_quorum_accepts_k_distinct_agreeing_signers` (quorum reached) +
  `directory_trust_anchor_impl_returns_the_attested_values` (task 5.5 - `trusted_app_hash`/`trusted_digest`/
  `trusted_node_identities_hash` all match a quorum-agreed mock snapshot)
- [x] 8.3 Covered by `reach_quorum_fails_with_fewer_than_k_agreeing_signers`
- [x] 8.4 Covered by `reach_quorum_counts_a_duplicated_signer_once`
- [x] 8.5 Covered by `reach_quorum_ignores_untrusted_or_invalid_attestations` (untrusted signer, forged signature) plus
  `verify_rejects_an_untrusted_signer` / `verify_rejects_a_mismatched_chain_id_or_contract` /
  `verify_rejects_a_forged_or_malformed_signature` (task 2.2, exercising the same `verify()` gate `reach_quorum` calls
  internally)
- [x] 8.6 Covered by `reach_quorum_rejects_disagreeing_signers`
- [x] 8.7 New: `refresh_pins_the_height_and_a_cached_query_does_not_requery_sources` - after `refresh()`, exactly one
  source was asked for `latest_snapshot` (the seed) and exactly one `snapshot_at` call happened in total (the confirm
  round, excluding the seed itself); a subsequent `trusted_app_hash(H)` for that now-cached height leaves both
  call-count totals unchanged. Summed across both mock sources rather than asserting on a specific one, since which
  source is chosen as seed is randomized (verified stable across repeated runs)
- [x] 8.8 Covered by `snapshot_for_returns_the_cached_value_on_a_second_call` (first call is a genuine `snapshot_at`
  -backed fetch, not yet cached) + `snapshot_for_rejects_a_height_no_quorum_can_attest` (`NoQuorumSnapshotForHeight`)
- [x] 8.9 Covered by `new_rejects_zero_quorum` + `new_rejects_quorum_exceeding_signer_count`
- [x] 8.10 Covered by `verify_directory_offline_succeeds_with_a_matching_hash` (new) +
  `verify_directory_offline_fails_closed_on_accumulator_mismatch` (new) +
  `verify_directory_offline_fails_closed_on_node_identities_hash_mismatch` (new) - added directly against
  `verify_directory_offline` itself for precise coverage, rather than relying on `verify_directory`'s equivalent tests (
  task 6.5) transitively covering its thin wrapper
- [x] 8.11 Covered by `verify_directory_offline_requires_a_node_identities_hash` (task 6.5) - since 6.4 does not take an
  anchor generically (see `design.md`'s task-6 addendum), "an anchor without one" is exercised as the caller passing
  `None`, which is the only way this ever happens regardless of anchor type
- [ ] 8.12 NOT done - flagged rather than built. `DirectoryClient::verified_directory`'s RPC-backed path needs a mock
  `CosmWasmClient` (for `query_contract_smart_at_height`, serving `AllEntries` / `GetNymNodeBondsPaged` responses) plus
  a mock RPC header lookup for `ProvenTrustAnchor::trusted_app_hash`. `nym-validator-client`'s existing
  `MockRpcClient` (already a dev-dependency here, used by `light_client.rs`'s tests) only implements `commit`/
  `validators` - `perform` (which `query_contract_smart_at_height` needs) is `unimplemented!()`, so it cannot serve this
  test as-is. Building this needs either extending `MockRpcClient` in `nym-validator-client` (a different crate,
  benefits future work there too) or hand-rolling a local mock implementing the `CosmWasmClient` +
  `NymContractsProvider` trait surface (a nontrivial amount of boilerplate for traits with many required methods). Given
  there is also no pre-existing test to regress against (this is a new test, not a check against prior behavior), and
  the refactor itself (6.3) is a small, directly-reviewed 1:1 extraction, this was left for a follow-up decision rather
  than building either option unprompted

## 9. Verification

- [x] 9.1 `cargo test -p nym-directory-contract-common --lib` passes (new payload + node-identity-hash tests)
- [x] 9.2 `cargo test -p nym-directory-client --lib` passes (attested anchor tests + decoupled verification tests +
  existing tests)
- [x] 9.3 `cargo build -p nym-directory-client` and `cargo build -p nym-directory-client --features light-client` both
  succeed (attested anchor is not feature-gated and must build in both)
