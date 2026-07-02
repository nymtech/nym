## 1. Spike - de-risk the ICS23 proof (phase 0)

- [ ] 1.1 Against a localnet/testnet directory contract, run `abci_query("store/wasm/key", raw_key, height=H, prove=true)` and inspect the returned `ProofOps` (expect two ops: an IAVL existence proof and a simple-merkle store proof).
- [ ] 1.2 Verify the digest-item proof end to end: chain the two layers (IAVL key -> wasm-store root, simple-merkle wasm-store -> `app_hash`) and confirm it verifies against the header's `app_hash`; pin down the `ics23` spec constants for each layer.
- [ ] 1.3 Decide and record: hand-chain two `ics23::verify_membership` calls vs depend on `ibc-core-commitment-types` `MerkleProof` (`ibc-proto` is present, the `ibc` verifier crate is not).

## 2. Crate scaffolding

- [ ] 2.1 Create `common/directory-client` (`nym-directory-client`) and register it in the workspace members.
- [ ] 2.2 Add dependencies: `nym-directory-contract-common`, `nym-lthash`, `nym-validator-client`, `nym-mixnet-contract-common`, `nym-crypto` (ed25519), `ics23` (with `host-functions`), `tendermint` / `tendermint-rpc`.

## 3. Shared validator-client extensions

- [ ] 3.1 Proof-carrying raw-store query: a typed helper on the nyxd client that runs `abci_query(..., prove=true)` for a raw store key at height `H` and returns the value + `ProofOps` + response height (build on the existing `abci_query`, which never passes `prove=true` today).
- [ ] 3.2 Height-pinned contract query: a height-parameterised `query_contract_smart` (and the `DirectoryQueryClient` paginated paths built on it) so contract reads can target an explicit height `H` instead of latest.

## 4. Chain-proof primitives (directory-client)

- [ ] 4.1 Wasm raw-key builder: `0x03 || len-prefix(canonical bech32 addr) || contract_key`; the digest key is `b"digest_state"`.
- [ ] 4.2 Entry raw-key builder: reproduce the `cw-storage-plus` `Path` bytes for `StoredNodeEntries` `(node_id, label)` and `StoredCuratedEntries` `String` keys (mirror the contract's `storage_key`).
- [ ] 4.3 ICS23 two-layer verifier (per 1.3): parse `ProofOps` (from the 3.1 helper) and verify a raw key/value up to the `app_hash`.
- [ ] 4.4 `app_hash` source: read the header for `H+1` via validator-client and take its `app_hash`; typed errors when the header/state is unavailable.

## 5. Trust anchor

- [ ] 5.1 Define `DirectoryTrustAnchor` (`async fn trusted_digest(&self, height) -> Result<[u8; 32]>`).
- [ ] 5.2 Paranoid impl: ICS23-prove the digest item against the RPC `app_hash` at `H` and return the proven 32-byte digest.

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
