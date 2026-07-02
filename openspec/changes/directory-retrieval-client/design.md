## Context

The `directory-contract` (archived change `directory-contract`) stores signed node entries `(node_id, label)` and admin-curated entries `(key)` as opaque bytes across two per-class stores, and maintains a single incremental LtHash multiset digest (`LtHash16`, ~2 KB `Item` at the raw key `digest_state`) over all entries. The 32-byte collapse of that accumulator is queryable, and the digest leaf is derived by the shared `EntryKey::digest_leaf` in `nym-directory-contract-common`, so an off-chain party can recompute it byte-for-byte. Nothing reads or verifies the directory yet; this change adds the retrieval client, starting with the paranoid mode that verifies against the chain's own `app_hash` and needs no nym-api infrastructure. The full client trust model (light-client anchoring, curated vs bonded tiers, DKG dealer cross-reference) was explored separately; this change implements the self-contained first slice.

## Goals / Non-Goals

**Goals:**
- A `nym-directory-client` library that retrieves the whole directory and cryptographically verifies it is the complete, untampered set committed on chain at a specific height.
- Paranoid whole-directory read: ICS23 proof of the digest `Item` against `app_hash` + local LtHash recompute + reject-on-mismatch.
- Node ed25519 signature verification and trust-tier classification (node self-authored vs admin-curated).
- Single-node verified read via an IAVL membership proof.
- A trust-anchor seam so normal mode (nym-api quorum) and a full light client slot in later without touching the verify core.

**Non-Goals:**
- A full tendermint light client (pinned checkpoint + validator-set verification): phase 1b, behind the same seam.
- Normal mode (K-of-N nym-api quorum) and the nym-api signed-snapshot endpoint it needs: phase 2.
- The producer side (nodes pushing) and the label/payload taxonomy.
- Interpreting payload semantics (e.g. decoding SphinxKeys): the client is payload-agnostic.
- HTTP-pull fallback for unpublished nodes: acting on the unpublished set requires the per-label payload taxonomy (what each node publishes and how its HTTP shape maps to a directory entry), which is undecided producer work, so it belongs to the consumer integration, not this verifier.
- nym-api / node-status / client integration that consumes the library.

## Decisions

### D1. Trust-anchor abstraction
A `DirectoryTrustAnchor` trait yields a digest the caller is willing to trust at a height: `async fn trusted_digest(&self, height) -> Result<[u8; 32]>`. The verify core (fetch all entries at that height, recompute LtHash, compare, verify signatures) is identical behind it. Phase 1 ships the paranoid impl; normal (nym-api quorum) and light-client-backed impls are additive. Alternative considered: hardcode the ICS23 path into the reader - rejected, it locks out normal mode and couples the reader to proof plumbing.

### D2. One proven digest + local recompute (not per-entry proofs)
Whole-directory integrity is established by proving the single digest `Item` once and recomputing the LtHash locally from all entries. Because the digest is a secure multiset commitment over every leaf, a recompute match authenticates the whole set. Alternative: per-key IAVL proofs for every entry - rejected, O(N) proofs and bandwidth for what one digest proof + O(N) local hashing already gives.

### D3. Single-height consistency (correctness invariant)
The digest proof and every paginated `AllEntries` page are read at one fixed height `H`. `AllEntries` is inherently paginated (bounded page size), so reading at "latest" would interleave pre- and post-write entries into a set that matches no committed digest - a false mismatch, or a masked real tamper. Mechanism: pick a recent finalized `H`; read its `app_hash` (at `header[H+1]`, see D6); `abci_query` the digest proof at `height = H`; run every smart-query page at `height = H` via a height-pinned query (the current `query_contract_smart` targets latest, so a height-parameterised variant is added). The digest query itself is redundant with the proven value and is dropped in favor of the proven digest.

### D4. ICS23 two-layer store proof
`abci_query("store/wasm/key", raw_key, height=H, prove=true)` returns two `ProofOps`: an IAVL existence proof (key -> wasm-store root) and a simple/tendermint proof (wasm-store -> multistore root = `app_hash`). Verify with two chained `ics23::verify_membership` calls (`ics23 0.12`, `host-functions` feature for SHA-256). Alternative: `ibc-core-commitment-types` `MerkleProof::verify_membership`, which wraps the two-layer chain - decide in the phase-0 spike (`ibc-proto` is in the tree; the `ibc` verifier crate is not).

### D5. Raw storage-key reconstruction
Only raw store reads yield proofs (smart queries do not), so the client reproduces the wasm raw key: `0x03 || len-prefix(canonical bech32 addr) || contract_key`. For the digest, `contract_key = b"digest_state"`. For a single entry, `contract_key` is the `cw-storage-plus` `Path` bytes reproduced from the contract's `StoredNodeEntries` / `StoredCuratedEntries` `storage_key` layout (`(node_id, label)` / `String` primary keys). Entry values decode via the existing `NodeEntry` / `CuratedEntry` `try_from_bytes` codecs.

### D6. Phase-1a `app_hash` from a configured RPC
The paranoid anchor fetches the block header for `H+1` from a configured RPC and takes its `app_hash` (CometBFT commits block `H`'s app hash in `header[H+1]`). Phase 1a therefore trusts the RPC for the header only, not for the proof or the entries. Phase 1b swaps in a light client (validator-set verification from a pinned checkpoint) behind D1's trait with no verify-core change. Chosen to de-risk the ICS23 wiring before the larger light-client build.

### D7. Node-signature verification and tiering
Each node entry's ed25519 signature is verified over `node_signing_payload(node_id, label, sequence, data)` against the node's identity key, cross-queried from the mixnet bond (`MixnetContractQuerier`, base58 -> 32 bytes) and cached. Curated entries carry no signature (admin authority). Entries are classified by tier for the caller. The signature is already committed to the digest, so proof + recompute authenticate the set; signature verification adds node-authorship attribution and tier separation on top.

### D8. Partial publication is valid, not an error
The verifier operates over whatever set the digest commits: during rollout only some bonded nodes have published, and the digest commits only that published subset, so a proof + recompute over the returned entries still verifies. The client does not assume all bonded nodes are present and does not treat absence as tamper. Reconciling the published set against the bonded set (and any HTTP fallback for the gap) is left to the consumer, because acting on it needs the payload taxonomy (see Non-Goals).

### D9. Crate placement
A new `common/directory-client` crate, so nym-api, node-status, and standalone consumers all reuse it. Alternative: fold into nym-api - rejected, not reusable and drags the proof stack into a service crate.

### D10. Shared RPC primitives live in validator-client
Two general nyxd/RPC capabilities the paranoid path needs are added to `nym-validator-client` (where `abci_query` and `query_contract_smart` already live), not to the directory-client crate, so any verifiable client can reuse them and the directory-client stays composition-only: (a) a **proof-carrying raw-store query** - a typed helper that runs `abci_query(..., prove=true)` for a raw key at height `H` and surfaces the value + `ProofOps` + response height (the plumbing exists but `prove=true` is never used and is not exposed ergonomically); and (b) **height-pinned contract queries** - a height-parameterised `query_contract_smart` (and the `DirectoryQueryClient` paths built on it) so `AllEntries` pages target an explicit `H` rather than latest. The directory-client composes these; it does not reimplement RPC access.

## Risks / Trade-offs

- [The two-layer ICS23 shape / spec constants are unverified] -> Phase-0 spike against a real `prove=true` read (localnet or testnet) before building the crate around it; decide `ics23` direct vs `ibc` `MerkleProof` there.
- [RPC lacks retained state at `H`] -> Require an archival RPC or a recent-enough `H`; surface a clear, typed error rather than a silent wrong result.
- [`app_hash` off-by-one] -> Always read the app hash from `header[H+1]`; cover with a test that a wrong-height app hash is rejected.
- [Height-pinned smart query missing today] -> Add a height-parameterised smart query; assert all `AllEntries` pages use `H`.
- [Phase 1a trusts the RPC for the header] -> Documented and bounded (header only); hardened in phase 1b via the same seam.
- [Digest recompute must match the contract byte-for-byte] -> Reuse the shared `EntryKey::digest_leaf` + `nym-lthash` (no reimplementation); a differential test recomputes and matches the on-chain digest on a populated localnet.

## Migration Plan

Additive: a new library with no chain, contract, or on-chain change. Rollback is simply not shipping the consumer that adopts it. Sequence: phase 0 spike (confirm the ICS23 two-layer proof) -> phase 1a (paranoid vs trusted-RPC `app_hash`) -> phase 1b (light client) and phase 2 (normal / nym-api quorum) as later changes behind the trust-anchor seam.

## Open Questions

- Exact `ics23` spec constants for the wasm IAVL layer vs the simple-merkle store layer, and whether to depend on `ibc-core-commitment-types` for a ready-made two-layer verifier (spike output).
- Height-selection policy (latest finalized vs a quantized height) and the concrete state-retention requirement on the RPC.
- Identity-key sourcing at scale (bulk mixnet bond query + cache invalidation across epochs).
- For phase 2 (future): whether the nym-api digest snapshot is signed and how the quorum threshold is expressed.
