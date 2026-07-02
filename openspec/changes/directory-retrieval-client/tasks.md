## 1. Spike - de-risk the ICS23 proof (phase 0)

- [x] 1.1 Retrieved a live sample proof (mixnet `admin` item, `query_contract_raw_with_proof`) and decoded the `ProofOps`: two ops confirmed - `ics23:iavl` (key -> wasm-store root) and `ics23:simple` `key="wasm"` (wasm-store -> `app_hash`).
- [x] 1.2 Verified end to end: `proof::verify_wasm_store_membership` + `proof::tests::verifies_a_live_membership_proof_and_rejects_tampering` (live mixnet `admin` sample) - `calculate_existence_root` for the subroot, then `verify_membership` with `iavl_spec()` / `tendermint_spec()` against the `app_hash` from `header[H+1]` (off-by-one confirmed); passes on the sample, rejects a wrong app_hash and a tampered value.
- [x] 1.3 Decided: hand-chain `ics23::verify_membership` directly (not `ibc-core-commitment-types`) - `ibc-rs` saves ~0 net code for a fixed 2-op proof while adding a dependency tree. See design D4.

## 2. Crate scaffolding

- [x] 2.1 Created `common/nym-directory-client` (`nym-directory-client`), registered in the workspace.
- [x] 2.2 Dependencies added: `nym-directory-contract-common`, `nym-lthash`, `nym-validator-client` (http-client), `nym-mixnet-contract-common`, `nym-crypto` (asymmetric), `ics23` (host-functions), `thiserror`. (`tendermint` types come through validator-client; add directly only if needed.)

## 3. Shared validator-client extensions

- [x] 3.1 Proof-carrying raw-store query: `query_contract_raw_with_proof(addr, key, height) -> ProvableAbciQueryResponse` (with `query_contract_raw_at_height` + `make_raw_abci_query_with_proof`), returning value + `ProofOps` + height; `ProvableAbciQueryResponse::map` added.
- [x] 3.2 Height-pinned contract query: `query_contract_smart_at_height` added; `query_contract_smart` now delegates to it with `None`. (Wire the `DirectoryQueryClient` paginated paths through it in the verify core, task 6.1.)

## 4. Chain-proof primitives (directory-client)

- [x] 4.1 Wasm raw-key builder: `contract_storage_key(contract, key) = 0x03 || canonical_addr || key` (no length prefix; 32-byte addrs) - hoisted into `nym-validator-client` (`nyxd::cosmwasm_client::contract_storage_key`) so `query_contract_raw_with_proof` and consumers share ONE layout; unit-tested against the live `admin` sample. Directory-specific `digest_state_key` in `nym-directory-client/src/key.rs` delegates to it.
- [ ] 4.2 Entry raw-key builder: reproduce the `cw-storage-plus` `Path` bytes for `StoredNodeEntries` `(node_id, label)` and `StoredCuratedEntries` `String` keys (mirror the contract's `storage_key`), on top of `contract_storage_key`. Needed for §7 single-node proofs.
- [x] 4.3 ICS23 two-layer verifier: `proof::verify_wasm_store_membership(ops, app_hash, key, value)` (hand-chained `calculate_existence_root` + two `verify_membership` calls with `iavl_spec`/`tendermint_spec` + `HostFunctionsManager`, typed `ProofError`), in `common/nym-directory-client/src/proof.rs`.
- [x] 4.4 `app_hash` source: the proven anchor reads `header[H+1]` via `TendermintRpcClientExt::header` and takes its `app_hash`; RPC/header errors map to `AnchorError::Query`.

## 5. Trust anchor

- [x] 5.1 `DirectoryTrustAnchor` trait (`async fn trusted_digest(&self, height) -> Result<TrustedDigest, AnchorError>`, `TrustedDigest { height, accumulator: LtHash16 }`) in `common/nym-directory-client/src/anchor.rs`.
- [x] 5.2 `ProvenTrustAnchor<C: TendermintRpcClientExt>`: reconstructs `digest_state_key`, `make_raw_abci_query_with_proof` at `H`, fetches `header[H+1].app_hash`, `verify_wasm_store_membership`, and parses the raw value into `LtHash16` (returns the accumulator, not `out()`, so the verify core compares full accumulators). Uses the narrowest RPC bound, not `CosmWasmClient`. Live/integration test deferred to §8 (needs a deployed directory contract / localnet).

## 6. Verify core

- [ ] 6.1 Fetch all entries at `H` via the height-pinned `get_all_directory_entries` (all pages pinned to `H`, using 3.2).
- [ ] 6.2 Recompute `LtHash16` over each `DirectoryEntryRecord::digest_leaf()`, compare `out()` to the trusted digest, reject on mismatch.
- [ ] 6.3 Node signature verification: verify the ed25519 signature over `node_signing_payload` against identity keys cross-queried from the mixnet bond (`MixnetContractQuerier`, base58 -> 32 bytes) with a cache; flag entries whose signature does not verify as unauthenticated.
- [ ] 6.4 Classify each returned entry by trust tier (node self-authored vs admin-curated) in the returned type.
- [ ] 6.5 Verify over the committed subset only; do not treat unpublished bonded nodes as a verification failure.
- [ ] 6.6 Typed fail-closed errors for missing header / state / non-verifying proof (never return unverified data as verified).

## 7. Single-node verified read

- [ ] 7.1 Single-entry read: build the entry raw key, verify an ICS23 membership proof against the `app_hash` at `H`, decode via the entry value codec, and report an absent entry distinctly from a verification failure.

## 8. Tests

- [ ] 8.1 Differential: on a populated localnet, recompute the digest from `get_all_directory_entries` and assert it equals the on-chain / proven digest.
- [ ] 8.2 Tamper detection: a mutated entry yields a recompute mismatch and is rejected.
- [ ] 8.3 Proof rejection: a forged proof and a wrong-height `app_hash` are both rejected.
- [ ] 8.4 Signatures: an invalid node signature is flagged unauthenticated; curated entries are not signature-checked.
- [ ] 8.5 Partial publication verifies over the committed subset; pruned/unavailable state fails closed with a typed error.
- [ ] 8.6 Single-node read: present entry verifies, absent entry is reported as absent.
