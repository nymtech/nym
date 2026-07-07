## 1. Canonical attestation encoding (shared crates)

- [ ] 1.1 Add `digest_snapshot_signing_payload(chain_id: &str, contract: &AccountId, height: u64, app_hash: &[u8], accumulator: &[u8; DIGEST_LEN], node_identities_hash: &[u8; 32]) -> Vec<u8>` to `common/cosmwasm-smart-contracts/directory-contract/src/helpers.rs`, reusing `push_len_prefixed` and prefixing a distinct domain-separation tag (so a snapshot signature can never collide with a `node_signing_payload` signature)
- [ ] 1.2 Re-export it from `nym-directory-contract-common`'s public surface
- [ ] 1.3 Unit tests: payload is deterministic and field-sensitive (differs on any of chain-id / contract / height / app_hash / accumulator / node_identities_hash); length-prefix framing disambiguates adjacent variable-length fields; domain tag differs from the node-entry payload
- [ ] 1.4 Add a canonical hash encoder for the `NodeId -> ed25519 identity` mapping (sorted, length-prefixed pairs, plain cryptographic hash - not an LtHash accumulator, since it is recomputed fresh each time rather than incrementally updated) in an appropriate shared crate (e.g. `nym-mixnet-contract-common`, next to the bond types it hashes)
- [ ] 1.5 Unit tests for the node-identity hash encoder: deterministic, sensitive to any `(node_id, identity)` change, order-independent (sorts internally so caller iteration order does not matter)

## 2. Attestation types and transport trait (client crate)

- [ ] 2.1 Define `DigestSnapshot { chain_id: String, directory_contract: AccountId (or String), height: Height, app_hash: AppHash, accumulator: [u8; DIGEST_LEN], node_identities_hash: [u8; 32] }` and `SignedDigestSnapshot { snapshot: DigestSnapshot, signer: ed25519::PublicKey, signature: Vec<u8> }` in `src/anchor/attested.rs` (Serialize/Deserialize; signature kept as bytes so malformed data is a verification failure, not a decode panic - mirrors `DirectoryNodeEntry`)
- [ ] 2.2 Implement `SignedDigestSnapshot::verify(&self, trusted: &BTreeSet<ed25519::PublicKey>, chain_id: &str, contract: &AccountId) -> bool`: signer in trusted set AND chain-id + contract match AND ed25519 signature verifies over `digest_snapshot_signing_payload(..)` (mirror `node_signature_verifies`)
- [ ] 2.3 Define `#[async_trait] pub trait AttestationSource { async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, DirectoryClientError>; async fn snapshot_at(&self, height: Height) -> Result<SignedDigestSnapshot, DirectoryClientError>; }`

## 3. AttestedTrustAnchor core

- [ ] 3.1 Define `TrustedSnapshot { app_hash: AppHash, accumulator: LtHash16, node_identities_hash: [u8; 32] }` and private `AttestedTrustAnchorState { snapshots: BTreeMap<Height, TrustedSnapshot>, latest: Option<Height> }`
- [ ] 3.2 Define `AttestedTrustAnchor<S> { sources: Vec<S>, trusted_signers: BTreeSet<ed25519::PublicKey>, quorum: usize, chain_id: String, directory_contract: AccountId, state: Mutex<AttestedTrustAnchorState> }`
- [ ] 3.3 Implement `new(sources, trusted_signers, quorum, chain_id, directory_contract)` validating `1 <= quorum <= trusted_signers.len()` (error otherwise)
- [ ] 3.4 Implement a private `reach_quorum(candidates: Vec<SignedDigestSnapshot>) -> Result<(Height, TrustedSnapshot), DirectoryClientError>`: filter to valid attestations (via `verify`), group by `(height, app_hash, accumulator, node_identities_hash)`, count DISTINCT signer keys per group, accept the first group reaching `quorum`, else `QuorumNotReached { needed, agreed }`
- [ ] 3.5 Implement `refresh(&self) -> Result<Height, DirectoryClientError>`: query all sources' `latest_snapshot()` concurrently, `reach_quorum`, insert into `snapshots`, set `latest`, return the agreed height
- [ ] 3.6 Implement `latest_snapshot_height(&self) -> Result<Height, DirectoryClientError>`: return cached `latest` or call `refresh()`
- [ ] 3.7 Implement a private `snapshot_for(&self, height) -> Result<TrustedSnapshot, DirectoryClientError>`: cache hit on `height` returns immediately; on miss query all sources' `snapshot_at(height)`, `reach_quorum` (verifying the returned height matches), cache, return; a height the quorum cannot attest returns `NoQuorumSnapshotForHeight(height)`

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

- [ ] 7.1 Add to `DirectoryClientError`: `QuorumNotReached { needed: usize, agreed: usize }`, `NoQuorumSnapshotForHeight(u64)`, `InvalidQuorumConfig { quorum: usize, signers: usize }`, `NodeIdentitiesHashUnavailable`, and an attestation-transport / decode variant as needed

## 8. Tests (mock transport)

- [ ] 8.1 Add a `MockAttestationSource` (in-memory, serves pre-registered latest + per-height signed snapshots; records call log) - mirror the `MockRpcClient` pattern; helper to build a `SignedDigestSnapshot` from a seeded `KeyPair`
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
