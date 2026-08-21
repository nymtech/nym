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
- [x] 4.2 Entry raw-key builder: `key::node_entry_key` / `curated_entry_key` reproduce the `cw-storage-plus` `Path` bytes for `StoredNodeEntries` `(node_id, label)` and `StoredCuratedEntries` `String` keys on top of `contract_storage_key`; unit-tested by cross-checking against `cw_storage_plus::Path` directly (not a hand-rolled golden).
- [x] 4.3 ICS23 two-layer verifier: `proof::verify_wasm_store_membership(ops, app_hash, key, value)` (hand-chained `calculate_existence_root` + two `verify_membership` calls with `iavl_spec`/`tendermint_spec` + `HostFunctionsManager`, typed `ProofError`), in `common/nym-directory-client/src/proof.rs`.
- [x] 4.4 `app_hash` source: the proven anchor reads `header[H+1]` via `TendermintRpcClientExt::header` and takes its `app_hash`; RPC/header errors map to `AnchorError::Query`.

## 5. Trust anchor

- [x] 5.1 `DirectoryTrustAnchor` trait in `common/nym-directory-client/src/anchor/mod.rs`: `trusted_app_hash(height) -> AppHash` (the trusted chain head - the single root both the digest proof and single-entry proofs check against) and `trusted_digest(height) -> TrustedDigest { height, accumulator: LtHash16 }`. Errors are the unified `DirectoryClientError`.
- [x] 5.2 `ProvenTrustAnchor<C: TendermintRpcClientExt>`: `trusted_app_hash` reads `header[H+1].app_hash`; `trusted_digest` reconstructs `digest_state_key`, `make_raw_abci_query_with_proof` at `H`, verifies via `verify_wasm_store_membership` against `self.trusted_app_hash(H)`, and parses the raw value into `LtHash16` (returns the accumulator, not `out()`, so the verify core compares full accumulators). Uses the narrowest RPC bound, not `CosmWasmClient`. Live/integration test deferred to §8 (needs a deployed directory contract / localnet).

## 6. Verify core

- [x] 6.1 `DirectoryClient::all_entries_at` pages `AllEntries` via `query_contract_smart_at_height`, every page pinned to `H` (its own height-pinned loop rather than the latest-only `get_all_directory_entries`), in `common/nym-directory-client/src/client.rs`.
- [x] 6.2 `verify::recompute_accumulator` folds each `DirectoryEntryRecord::digest_leaf()` into an `LtHash16` and the client compares the full recomputed accumulator to the proven `TrustedDigest.accumulator` (stronger than the 32-byte `out()`), returning `DirectoryClientError::DigestMismatch` on any difference.
- [x] 6.3 Node signature verification: `verify::node_signature_verifies` checks the ed25519 signature over `node_signing_payload` against identity keys bulk-fetched from the mixnet bond (`GetNymNodeBondsPaged` at `H`, base58 -> key, collected into a map in `all_node_identities_at`); a node with any non-verifying signature has `DirectoryNode::verified = false`.
- [x] 6.4 `VerifiedDirectory` separates `curated_entries` (admin authority) from `node_entries` keyed by `NodeId`, each `DirectoryNode` carrying a `verified` flag plus known/unknown label maps - the trust-tier split.
- [x] 6.5 Verifies only over the returned committed records; bonded-but-unpublished nodes are never treated as a failure.
- [x] 6.6 Fail-closed `DirectoryClientError` for anchor (missing header/state, non-verifying proof) and query errors, and `DigestMismatch`; never returns unverified data as verified.

## 7. Single-node verified read

- [x] 7.1 Single-entry verified reads - `DirectoryClient::verified_node_entry` and `verified_curated_entry` - build the entry raw key, read it with a proof at `H`, take the trusted `app_hash` from the trust anchor (`anchor.trusted_app_hash`, NOT re-fetched from the RPC serving the proof - else a malicious RPC could supply a self-consistent forgery), and via `proof::verify_wasm_store_presence` either verify an ICS23 membership proof (decode the value with the contract codec; node entries additionally check the signature against the bonded identity -> `Some(ProvenNodeEntry)`; curated entries carry no signature - the membership proof is the authentication - and return the decoded payload bytes) or a full non-existence proof (`proof::verify_wasm_store_non_membership` -> `Ok(None)`), so absence is distinct from a verification failure (`Err`). Presence is decided by the proof shape, not value emptiness.

## 8. Tests

- [x] 8.1 Differential: proven on a populated localnet (`temp_localnet_test`) - recomputing the digest from all retrieved entries equals the on-chain / proven digest.
- [x] 8.2 Tamper detection: `verify::tests::a_tampered_entry_changes_the_recomputed_digest` (mutating an entry changes the recomputed accumulator, which the client rejects as `DigestMismatch`).
- [x] 8.3 Proof rejection: `proof::tests::verifies_a_live_membership_proof_and_rejects_tampering` rejects a wrong `app_hash` (`StoreVerificationFailed`) and a tampered value (`IavlVerificationFailed`).
- [x] 8.4 Signatures: `verify::tests::node_signature_verification_accepts_valid_and_rejects_forged` (valid / wrong-key / tampered-data / malformed-bytes); curated entries are structurally never signature-checked (the client routes them straight to `curated_entries`).
- [x] 8.5 Partial publication + fail-closed, satisfied by construction: the verify core only iterates the committed subset `AllEntries` returns and never enumerates the bonded set, so a bonded-but-unpublished node cannot cause a failure; and every fallible step propagates via `?` (`trusted_app_hash` / `make_raw_abci_query_with_proof` / `all_entries_at` -> typed `ChainQueryFailure`), with no path returning `Ok` with unverified data. The verification-failure -> `Err` half is exercised offline (wrong `app_hash` -> `StoreVerificationFailed`; tampered entry -> `DigestMismatch`).
- [x] 8.6 Single-node read: the present/absent decision (`verify_wasm_store_presence`) is covered offline both ways - `proof::tests::verifies_a_live_membership_proof_and_rejects_tampering` (Present) and `proof::tests::verifies_a_live_non_membership_proof_and_rejects_tampering` (Absent, via a real non-existence fixture; also rejects a wrong app_hash and a proof-for-a-different-key). The `verified_node_entry` wrapper composes these primitives.
