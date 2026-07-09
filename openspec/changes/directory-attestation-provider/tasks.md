## 1. Contract snapshot cadence

Add the snapshot interval as a plain on-chain `Item` (see `design.md` D5): not in the
LtHash digest, not in the signed snapshot. Admin-gated mutation, exactly like the
existing directory-contract admin ops (reference `NYM_DIRECTORY_CONTRACT_STORAGE`
directly, no `let storage` alias). Breaking change is acceptable - contract not deployed.
Mind the `ed25519-zebra` / `contracts/Cargo.lock` build-break (pin 4.0.3 locally if it
resurfaces).

- [x] 1.1 Add a `snapshot_interval: u64` storage `Item` (blocks) under its own storage key in `nym-directory-contract`,
  set at instantiate; validate it is positive (reject `0`)
- [x] 1.2 Add an `InstantiateMsg` field for the initial interval (and a sensible default in the downstream instantiate
  wiring)
- [x] 1.3 Add a `SnapshotInterval` query variant + handler returning the current interval
- [x] 1.4 Add an admin-gated `UpdateSnapshotInterval { interval }` execute variant + handler (same admin check as the
  other admin ops; reject non-admin, reject `0`)
- [x] 1.5 Wire the new instantiate field through the downstream constructors: `network-defaults`, contract-generator,
  localnet orchestrator, and the wallet if it builds the directory instantiate msg
- [x] 1.6 Contract unit tests: interval set at instantiate + queryable; admin update changes it; non-admin update
  rejected; `0` rejected at instantiate and update; the global digest is unchanged by an interval set/update (the
  interval is not a digest leaf)

## 2. `nym-directory-attestation` crate (shared protocol)

New crate at `common/directory-attestation/` (package `nym-directory-attestation`),
light deps only - no `nym-validator-client` (see `design.md` D1). This is the library
imported by the producer (nym-api, later nym-node) and the verifying client.

- [ ] 2.1 Scaffold `Cargo.toml` (deps: `nym-crypto`, `serde`, `cosmrs`/`tendermint`, `nym-lthash` with `serde`+`Hash`,
  `blake3`, `nym-mixnet-contract-common`, `async-trait`) and register it as a workspace member
- [ ] 2.2 Move `DigestSnapshot`, `SignedDigestSnapshot`, `digest_snapshot_signing_payload`, and
  `SignedDigestSnapshot::verify` from `nym-directory-client::anchor::attested` into this crate (make them `pub`),
  keeping their existing behavior byte-for-byte
- [ ] 2.3 Move `node_identities_hash` from `nym-directory-client::verify` into this crate (`pub`); leave
  `recompute_accumulator` in `nym-directory-client` (consumer-only - the producer reads the on-chain digest / verifies
  via its anchor rather than recomputing)
- [ ] 2.4 Move the `AttestationSource` trait here (`pub`); the concrete HTTP impl stays in `nym-directory-client` (task
    3)
- [ ] 2.5 Move the corresponding unit tests (snapshot payload determinism/field-sensitivity, domain-tagging, `verify`
  accept/reject, `node_identities_hash` determinism/sensitivity) into this crate; confirm they pass unchanged
- [ ] 2.6 Add the generic subset types:
  `trait DirectorySubset { const SUBSET_ID: &'static str; fn canonical_bytes(&self) -> Vec<u8>; }`,
  `SubsetDigest { chain_id, height, subset_id, hash: [u8; 32] }`, `SignedSubsetDigest { digest, signer, signature }`,
  `AttestedSubset<T: DirectorySubset> { signed_digest, data }` (all `Serialize`/`Deserialize`; signature/pubkey typed
  but decode-tolerant per the `SignedDigestSnapshot` precedent)
- [ ] 2.7 Add the subset canonical hash encoder (domain-tagged, distinct from the snapshot tag;
  `hash = blake3(SUBSET_ID || height || canonical_bytes(data))` with length-prefixed framing) and
  `SignedSubsetDigest::verify(trusted, chain_id)` mirroring `SignedDigestSnapshot::verify`
- [ ] 2.8 Add the signer-agnostic producer core: `build_and_sign_snapshot(inputs, &keypair) -> SignedDigestSnapshot` and
  `sign_subset<T: DirectorySubset>(chain_id, height, &data, &keypair) -> AttestedSubset<T>` (pure over pre-fetched
  inputs; no chain/RPC/HTTP)
- [ ] 2.9 Unit tests: subset digest deterministic + tamper-sensitive (data change flips the hash), `verify`
  accept/reject (untrusted signer, wrong chain-id, forged signature), and a dummy-subset round-trip (`sign_subset` ->
  `SignedSubsetDigest::verify` -> recompute `canonical_bytes` matches `digest.hash`)

## 3. `nym-directory-client` (consumer)

Depend on the new crate; delete moved code; re-export to preserve every downstream
`use` path (see `design.md` D1). Add the concrete HTTP transport + client-side subset
and whole-directory consumption.

- [ ] 3.1 Add `nym-directory-attestation` as a dependency; delete the moved items; re-export `DigestSnapshot`/
  `SignedDigestSnapshot`/`AttestationSource` (and the subset types) from `src/anchor/mod.rs` so `AttestedTrustAnchor`
  and existing consumers compile unchanged
- [ ] 3.2 Add `DirectoryClientError::AttestationTransport` (wrapping the HTTP client + decode failure) - the variant the
  prior change deferred for lack of a call site
- [ ] 3.3 Implement `HttpAttestationSource` (`AttestationSource` impl): `identity()` (configured/known signer key for
  the URL), `latest_snapshot()` -> GET the producer's latest endpoint, `snapshot_at(H)` -> GET the per-height endpoint;
  deserialize `SignedDigestSnapshot`, mapping transport/decode errors to 3.2
- [ ] 3.4 Add the client subset path: `quorum_subset_digest<T>(sources, height) -> Result<[u8; 32], _>` (fetch each
  source's `SignedSubsetDigest`, `reach_quorum` on the hash reusing the anchor's distinct-signer counting) and
  `fetch_and_verify_subset<T>(source, quorum_hash) -> Result<T, _>` (fetch one `AttestedSubset<T>`, recompute
  `canonical_bytes` and require `== digest.hash == quorum_hash`, counting the embedded `signed_digest` as at most one
  vote)
- [ ] 3.5 Add a whole-directory-from-a-nym-api fetch that pulls entries + node identities over HTTP and verifies them
  via the existing `verify_directory_offline` against `AttestedTrustAnchor`'s `accumulator` + `node_identities_hash` (
  see `design.md` D8) - not the subset mechanism
- [ ] 3.6 Tests: `HttpAttestationSource` against a mock HTTP server (latest + at-height, plus a transport-error case);
  subset quorum happy path + fail-closed (tampered data, sub-quorum, disagreeing hashes); whole-directory-from-http
  happy + fail-closed

## 4. nym-api producer

Wire the library into nym-api (see `design.md` D6/D7/D10). Reuse
`AppState.identity_keypair`, the `DescribedNodes`/mixnet caches, and the `nyxd` client.

- [ ] 4.1 Producer config: retained-window count `N` (default ~3), `settle_lag` in blocks (default ~5), and the
  source-anchor selection (default `ProvenTrustAnchor` against the api's own RPC; allow `LightClientAnchor`; never
  `AttestedTrustAnchor`)
- [ ] 4.2 A retained-window store in `AppState`: `BTreeMap<Height, (SignedDigestSnapshot, VerifiedDirectory)>` (or
  equivalent), holding the last `N` cadence snapshots plus their full verified directories
- [ ] 4.3 A periodic producer task (reusing nym-api's cache-refresh cadence pattern): read `snapshot_interval` from the
  contract; on crossing a cadence boundary `H` (once `H+1` is available for `app_hash`), fetch+verify the directory at
  `H` via the configurable source anchor, compute `node_identities_hash`, and `build_and_sign_snapshot` with the
  identity keypair; insert into the store, prune to `N`
- [ ] 4.4 HTTP routes: `latest` (settle-lagged: greatest retained `H` with `tip >= H + settle_lag`), snapshot at a
  specific retained height, and the full verified directory at a retained height; register under nym-api's versioned
  route tree with `utoipa` annotations
- [ ] 4.5 Give `SignedDigestSnapshot` (and the full-directory response shape) whatever `ToSchema`/serde the routes
  need - in the attestation crate if clean, else a thin `nym-api-requests` wrapper
- [ ] 4.6 Tests: cadence-height selection + retention/pruning; settle-lag applied to `latest`; snapshot round-trips
  through the route DTOs; producer refuses to attest a directory its source anchor fails to verify

## 5. Verification

- [ ] 5.1 `cargo build` + `cargo test` for `nym-directory-attestation`
- [ ] 5.2 `cargo build` + `cargo test` for `nym-directory-client` (default and `--features light-client`) - moved-type
  re-exports keep it compiling and green
- [ ] 5.3 `cargo build` + `cargo test` for `nym-directory-contract` (+ `nym-directory-contract-common`): the cadence
  tests (1.7)
- [ ] 5.4 `cargo build` + `cargo test` for `nym-api`: producer tests (4.6)
- [ ] 5.5 Confirm the downstream instantiate wiring (network-defaults / generator / localnet / wallet) still builds
  after the new instantiate field (1.6)
- [ ] 5.6 (Use `cargo build`/`check` + `test`, not `clippy`, for verification, per project preference)
