## 1. Canonical attestation encoding

Lives in `nym-directory-client` itself, not a contract-common crate: neither function is
ever consumed by a contract, only by this crate (verifying) and the not-yet-built nym-api
producer (signing) - the same two-off-chain-peers pairing `recompute_accumulator` already
serves from `verify.rs`. Placement revisited mid-implementation (see `design.md` D3); the
producer, when it lands, can decide then whether to depend on this crate or extract a
shared piece, once its real constraints are known.

- [x] 1.1 Add `digest_snapshot_signing_payload(chain_id: &str, contract: &AccountId, height: Height, app_hash: &AppHash, accumulator: &LtHash16, node_identities_hash: &[u8; 32]) -> Vec<u8>` to `common/nym-directory-client/src/anchor/attested.rs`, with its own local length-prefixing helper and a domain-separation tag (so a snapshot signature can never collide with a `node_signing_payload` signature)
- [x] 1.2 N/A - not a shared crate, no re-export surface; `pub(crate)` within `nym-directory-client`, consumed by `attested.rs` itself (task 2/3)
- [x] 1.3 Unit tests: payload is deterministic and field-sensitive (differs on any of chain-id / contract / height / app_hash / accumulator / node_identities_hash); length-prefix framing disambiguates adjacent variable-length fields; domain tag differs from a representative `node_signing_payload` output
- [x] 1.4 Add `node_identities_hash` (sorted, fixed-width-per-record, plain `blake3` hash - not an LtHash accumulator, since it is recomputed fresh each time rather than incrementally updated) to `common/nym-directory-client/src/verify.rs`, next to `recompute_accumulator`
- [x] 1.5 Unit tests for `node_identities_hash`: deterministic, sensitive to any `(node_id, identity)` or membership change, order-independent (sorts internally so caller iteration order does not matter)

## 2. Attestation types and transport trait (client crate)

`trusted_signers`/`trusted` use `HashSet<ed25519::PublicKey>`, not the originally-sketched
`BTreeSet`: `ed25519::PublicKey` derives `Hash` but not `Ord` (confirmed in
`common/crypto/src/asymmetric/ed25519/mod.rs`), and a signer allowlist has no ordering
requirement to justify adding one - membership testing is all quorum counting needs.

- [x] 2.1 Define `DigestSnapshot { chain_id: chain::Id, directory_contract: AccountId, height: Height, app_hash: AppHash, accumulator: LtHash16, node_identities_hash: [u8; 32] }` (richer types than the original sketch - all confirmed to have serde support: `chain::Id`/`AccountId`/`Height` via hand-written impls, `AppHash` via `cosmrs::tendermint::serializers::apphash`, `LtHash16` via a new `serde` feature on `nym-lthash`, see task 1's follow-up) and `SignedDigestSnapshot { snapshot: DigestSnapshot, signer: ed25519::PublicKey, signature: Vec<u8> }` in `src/anchor/attested.rs` (both `Serialize`/`Deserialize`; signature kept as bytes so malformed data is a verification failure, not a decode panic - mirrors `DirectoryNodeEntry`)
- [x] 2.2 Implement `SignedDigestSnapshot::verify(&self, trusted: &HashSet<ed25519::PublicKey>, chain_id: &str, contract: &AccountId) -> bool`: signer in trusted set AND chain-id + contract match AND ed25519 signature verifies over `digest_snapshot_signing_payload(..)` (mirror `node_signature_verifies`); unit tests: valid attestation accepted, untrusted signer / mismatched chain-id / mismatched contract / forged or malformed signature all rejected
- [x] 2.3 Define `#[async_trait] pub trait AttestationSource { fn identity(&self) -> ed25519::PublicKey; async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError>; async fn snapshot_at(&self, height: Height) -> Result<SignedDigestSnapshot, DirectoryClientError>; }` (`identity()` added mid-implementation, ahead of the original sketch - a sync, no-network way for the anchor to recognize which source produced a given attestation, used by `refresh()`, task 3.5, to avoid re-querying the seed's own source)

## 3. AttestedTrustAnchor core

`refresh()`'s flow was revised mid-implementation (see `design.md` D6): rather than
comparing every source's independently-reported "latest" (which can split across a
cadence boundary if sources are not perfectly in lockstep), it seeds a height from the
first successful `latest_snapshot()` response, then asks every source's `snapshot_at`
that same height and reaches quorum over all of it. The seed is untrusted at that point -
just a discovery hint - so a lying seed only wastes a round-trip, never a false accept.

- [x] 3.1 Define `TrustedSnapshot { app_hash: AppHash, accumulator: LtHash16, node_identities_hash: [u8; 32] }` and private `AttestedTrustAnchorState { snapshots: BTreeMap<Height, TrustedSnapshot>, latest: Option<Height> }`
- [x] 3.2 Define `AttestedTrustAnchor<S> { sources: Vec<S>, trusted_signers: HashSet<ed25519::PublicKey>, quorum: usize, chain_id: chain::Id, directory_contract: AccountId, state: Mutex<AttestedTrustAnchorState> }` (`chain_id` typed as `chain::Id`, matching `DigestSnapshot` and `verify`, not the originally-sketched `String`)
- [x] 3.3 Implement `new(sources, trusted_signers, quorum, chain_id, directory_contract)` validating `1 <= quorum <= trusted_signers.len()` (error otherwise)
- [x] 3.4 Implement a private `reach_quorum(&self, candidates: Vec<SignedDigestSnapshot>) -> Result<(Height, TrustedSnapshot), DirectoryClientError>`: filter to valid attestations (via `verify`), group by `(height, app_hash, accumulator, node_identities_hash)` via a `HashMap<DigestSnapshot, HashSet<ed25519::PublicKey>>` keyed on a manual `Hash` impl added to `DigestSnapshot` (not the linear scan originally planned, once `LtHash16` gained a `Hash` impl to build it from - see design.md D3/D6), count DISTINCT signer keys per group, accept the *first* group (in candidate-arrival order, checked as each candidate is folded in, so the result is deterministic regardless of `HashMap` iteration order) reaching `quorum`, else `QuorumNotReached { needed, agreed }` (`agreed` = the largest distinct-signer count seen across any single group, via an order-independent `max()`)
- [x] 3.5 Implement `refresh(&self) -> Result<Height, DirectoryClientError>`: try sources' `latest_snapshot()` in shuffled order, one at a time, taking the first successful response as a height seed `H` (see design.md D6 for an open latency concern with this vs. querying concurrently); query every *other* source's (via `identity()`, task 2.3) `snapshot_at(H)` concurrently; `reach_quorum` over the seed plus all of those responses; insert into `snapshots`, set `latest`, return the agreed height
- [x] 3.6 Implement `latest_snapshot_height(&self) -> Result<Height, DirectoryClientError>`: return cached `latest` or call `refresh()`
- [x] 3.7 Implement a private `snapshot_for(&self, height) -> Result<TrustedSnapshot, DirectoryClientError>`: cache hit on `height` returns immediately; on miss query all sources' `snapshot_at(height)`, `reach_quorum` (verifying the returned height equals the requested one, else `NoQuorumSnapshotForHeight(height)` even if some other height reached quorum), cache, return; a height the quorum cannot attest at all returns `NoQuorumSnapshotForHeight(height)`

## 4. Default anchor

- [ ] 4.1 Define compiled-in default anchor constants (Nym-SA-owned nym-api identity keys + a default quorum threshold; concrete key material and endpoints TBD at implementation time)
- [ ] 4.2 Implement a default-anchor constructor (e.g. `AttestedTrustAnchor::with_default_anchor(sources, chain_id, directory_contract)`) that builds the anchor with the default `trusted_signers`/quorum, alongside the fully-configurable `new(...)` for callers who want to override
- [ ] 4.3 Unit test: the default-anchor constructor produces an anchor whose `trusted_signers`/quorum match the compiled-in default; `new(...)` with a caller-supplied set is unaffected by the default

## 5. DirectoryTrustAnchor impl and re-exports

- [ ] 5.1 Implement `trusted_app_hash(H)`: `snapshot_for(H)` then return its `app_hash`
- [ ] 5.2 Implement `trusted_digest(H)`: `snapshot_for(H)` then return `TrustedDigest { height: H, accumulator }` (no ICS23 proof - the quorum attests the accumulator directly)
- [ ] 5.3 Expose the snapshot's `node_identities_hash` for a given height via an anchor-specific accessor (not part of the shared `DirectoryTrustAnchor` trait, so `ProvenTrustAnchor` / `LightClientAnchor` are untouched)
- [ ] 5.4 Re-export `AttestedTrustAnchor`, `AttestationSource`, `SignedDigestSnapshot`, `DigestSnapshot` from `src/anchor/mod.rs`

## 6. Data-source-agnostic whole-directory verification

- [ ] 6.1 Extract the body of `DirectoryClient::verified_directory` (digest recompute, per-entry authorship attribution) into a function that accepts pre-fetched `records` and `node_identities`, plus the trusted `accumulator` and `node_identities_hash`, and needs no `CosmWasmClient` at all
- [ ] 6.2 Add a node-identity hash recompute check (mirroring `recompute_accumulator`) using the encoder from 1.4, failing closed (`DigestMismatch`-equivalent) on any mismatch
- [ ] 6.3 Make `DirectoryClient::verified_directory` a thin wrapper: fetch records + node identities via `self.client` as today, then call the extracted function - existing RPC-backed callers (any anchor) see no behavior change
- [ ] 6.4 Add a new entry point (free function or method not requiring `C: CosmWasmClient`) that verifies a whole-directory fetch given caller-supplied records + node identities and an anchor whose snapshot carries `node_identities_hash` - errors clearly if the anchor does not provide one (e.g. `NodeIdentitiesHashUnavailable`) rather than skipping authorship verification silently

## 7. Error handling

- [ ] 7.1 Add to `DirectoryClientError`: ~~`QuorumNotReached { needed: usize, agreed: usize }`~~, ~~`NoQuorumSnapshotForHeight(u64)`~~, ~~`InvalidQuorumConfig { quorum: usize, signers: usize }`~~ (all three added early, needed by task 3), `NodeIdentitiesHashUnavailable`, and an attestation-transport / decode variant as needed

## 8. Tests (mock transport)

- [ ] 8.1 Add a `MockAttestationSource` (in-memory, serves pre-registered latest + per-height signed snapshots + its `identity()`; records call log) - mirror the `MockRpcClient` pattern; helper to build a `SignedDigestSnapshot` from a seeded `KeyPair`. A non-call-logging version of this already exists as `MockSource` in `attested.rs`'s own test module (task 3); task 8 can promote/extend it rather than starting fresh
- [ ] 8.2 Unit test: K distinct trusted signers agreeing on identical values yields a trusted snapshot; `trusted_app_hash` and `trusted_digest` return the attested values
- [ ] 8.3 Unit test: fewer than K valid agreeing signers returns `QuorumNotReached`
- [ ] 8.4 Unit test: a duplicated signer key is counted once (does not reach quorum on its own)
- [ ] 8.5 Unit test: an attestation from an untrusted signer, or with an invalid signature, or with a mismatched chain-id / contract, is ignored (not counted toward quorum)
- [ ] 8.6 Unit test: signers disagreeing on `(app_hash, accumulator, node_identities_hash)` such that no group reaches K is rejected
- [ ] 8.7 Unit test: `refresh()` pins the latest agreed height; a later `trusted_app_hash(H)` for a cached height is served without re-querying sources (call log)
- [ ] 8.8 Unit test: a recent past height within the window is verified via `snapshot_at`; a height the quorum cannot attest returns `NoQuorumSnapshotForHeight`
- [ ] 8.9 Unit test: `new` with `quorum > signers` or `quorum == 0` returns `InvalidQuorumConfig`
- [ ] 8.10 Unit test: whole-directory verification via the decoupled entry point (6.4) succeeds against caller-supplied records + node identities matching the trusted snapshot, and fails closed on a mismatch in either the accumulator or the node-identities hash
- [ ] 8.11 Unit test: the decoupled entry point (6.4) against an anchor without a `node_identities_hash` returns `NodeIdentitiesHashUnavailable`
- [ ] 8.12 Unit test: `DirectoryClient::verified_directory` (RPC-backed path, 6.3) is behaviorally unchanged for `ProvenTrustAnchor` / existing tests

## 9. Verification

- [ ] 9.1 `cargo test -p nym-directory-contract-common --lib` passes (new payload + node-identity-hash tests)
- [ ] 9.2 `cargo test -p nym-directory-client --lib` passes (attested anchor tests + decoupled verification tests + existing tests)
- [ ] 9.3 `cargo build -p nym-directory-client` and `cargo build -p nym-directory-client --features light-client` both succeed (attested anchor is not feature-gated and must build in both)
