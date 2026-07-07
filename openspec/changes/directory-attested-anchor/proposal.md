## Why

The directory retrieval client can already anchor trust two ways: `ProvenTrustAnchor` (trusts a configured RPC for `app_hash`) and `LightClientAnchor` (verifies validator-set signatures, but needs a caller-supplied checkpoint). The remaining gap in the trust model (`project_directory_contract_trust_model_2026_06_24`) is a deployable bootstrap. The planned checkpoint / root-key layer (steps 1b/1c) is blocked: it requires generating and hardcoding a root key that does not exist yet, plus an initial checkpoint and a weak-subjectivity refresh flow.

The trust model already describes a second, independent authority we can stand up today: a K-of-N quorum of curated nym-api (Tier-1) identity keys, which already exist. `AttestedTrustAnchor` implements the `DirectoryTrustAnchor` seam using that quorum. Instead of a light client vouching for `app_hash` via validator signatures, a configurable quorum of nym-apis signs a directory snapshot, and the client accepts it only when K distinct trusted signers agree on identical values. This unblocks a verifiable-retrieval deployment without minting a root key, and slots behind the same trait so the verify core is untouched.

Two further requirements shaped this beyond the original sketch:

- Deployments should not have to hand-maintain a list of trusted nym-api keys with no sensible default. A small, stable, overridable default anchor removes that burden for the common case (trusting Nym SA's own nym-apis) while leaving the door open for an operator who does not want that trust. That override path has a real-world dependency this change does not solve: nym-api does not yet expose its own identity key unconditionally (see Non-Goals).
- Some deployments cannot or do not want to hold a direct chain RPC connection at all. The original sketch only removed the RPC dependency from *establishing* trust (`app_hash`); the directory data itself, and the node-identity bindings needed to attribute entries to their authors, still required a direct, trusted RPC connection regardless of anchor. Closing that gap for whole-directory retrieval is the other half of this change.

A third direction - generalizing attestation from "the whole directory" to *any* canonical subset a nym-api can publish (per-label slices, the current Tier-1 signer set itself derived from on-chain state, and eventually non-directory nym-api endpoints) - was explored alongside this but is explicitly out of scope here; see Non-Goals and `design.md`'s "Future direction".

## What Changes

- Add `AttestedTrustAnchor<S>` in `common/nym-directory-client/src/anchor/attested.rs`, a third `DirectoryTrustAnchor` implementation backed by a K-of-N quorum of configured nym-api identity keys.
- Define the signed snapshot attestation: the canonical signing-payload encoding lives in `nym-directory-contract-common` (next to `node_signing_payload`); the signed wrapper type and quorum verification live in the client crate. The snapshot commits to `{height, app_hash, accumulator, node_identities_hash}` - the directory digest AND a hash over the current `NodeId -> ed25519 identity` bindings from the mixnet contract, both computed by the signing nym-api from its own trusted chain access.
- Ship a small, hardcoded default anchor set (Nym-SA-owned nym-api identity keys/endpoints) as the out-of-the-box `trusted_signers`/sources for `AttestedTrustAnchor`, overridable by any caller who wants a different trust root.
- Decouple whole-directory verification from the client's own RPC connection: given a trusted snapshot, directory entries and node identities fetched from *any* untrusted source can be verified by local hash-recompute (`recompute_accumulator`, already existing, plus a new equivalent for node identities) with no chain query at all. The existing RPC-backed convenience path on `DirectoryClient` is preserved unchanged for callers that do hold a chain connection.
- Add an `AttestationSource` transport trait (fetch latest snapshot / fetch snapshot at a height) so the anchor is transport-agnostic and testable with a mock; the concrete HTTP transport and the nym-api producer endpoint are deferred to a follow-up.
- Height model: the anchor discovers the quorum-agreed LATEST snapshot and can also verify a specific recent height within the producers' retained window (for clients running behind, or straddling a sphinx-key rotation range transition).
- `ProvenTrustAnchor` and `LightClientAnchor` are unchanged; the `DirectoryTrustAnchor` trait surface is unchanged. Their node-identity lookups keep using the existing (unproven) smart query via the caller's RPC connection.
- Postpone the checkpoint / root-key bootstrap layer (steps 1b/1c) until a root key exists.

## Capabilities

### New Capabilities

- `directory-attested-anchor`: A `DirectoryTrustAnchor` implementation that establishes the trusted `app_hash`, directory digest, and node-identity binding from a K-of-N quorum of nym-api (Tier-1) identity keys signing a snapshot, ships with a small overridable default trust root, and requires no root key, no light-client checkpoint, and (for whole-directory retrieval) no direct chain RPC connection.

### Modified Capabilities

- `directory-retrieval-client`: gains `AttestedTrustAnchor` as a third anchor behind the same `DirectoryTrustAnchor` trait, and gains a way to verify a whole-directory fetch sourced from outside the client's own chain connection (single-entry ICS23 retrieval is unchanged).

## Impact

- `common/nym-directory-client/`: new `src/anchor/attested.rs` (anchor + attestation types + `AttestationSource` trait + default anchor constants), updated `src/anchor/mod.rs` (re-exports), new error variants in `src/error.rs`, and a new data-source-agnostic verification entry point in `src/client.rs` (existing RPC-backed methods become thin wrappers around it). No new runtime deps (reuses `nym-crypto` ed25519 + `async-trait` + `serde`), so no feature gate.
- `common/cosmwasm-smart-contracts/directory-contract/` (`nym-directory-contract-common`): new `digest_snapshot_signing_payload` canonical encoder next to `node_signing_payload`, extended to bind `node_identities_hash` alongside `app_hash`/`accumulator`, so a future producer reproduces identical bytes.
- A new canonical encoder for the `NodeId -> ed25519 identity` mapping hash (home TBD at implementation time - likely `nym-mixnet-contract-common`, next to the bond types it hashes).
- `verify.rs` / `client.rs` gain the decoupled verification path described above; `proof.rs`, `key.rs`, the contract logic, and `nym-api` are otherwise untouched (the producer endpoint is a follow-up).
- Consumers that want attested mode construct `AttestedTrustAnchor::new(sources, trusted_signers, quorum, chain_id, contract)` (or the default-anchor constructor), call `latest_snapshot_height()`, then drive `verified_directory(height)` as before, or the new no-RPC verification entry point if they sourced the data themselves.

## Non-Goals

- The nym-api producer endpoint that computes and signs snapshots (separate follow-up; this change defines the shared attestation format it must emit).
- A concrete HTTP `AttestationSource` implementation (lands with the producer, so the wire format is defined once, end to end).
- The contract-side directory refresh-cadence / snapshot-retention parameter (TBD; a producer / contract concern).
- The checkpoint / root-key bootstrap (steps 1b/1c), explicitly postponed until a root key exists.
- **nym-api identity-key exposure and possession-proof.** A prerequisite for D8's override path to be usable by operators who are not coconut-dkg dealers: today a nym-api's ed25519 identity key is only discoverable incidentally via DKG dealer registration (`DealerDetails.ed25519_identity`), so an operator running a nym-api purely to serve directory attestations has no API-exposed way to have their instance's identity key learned at all. Named here as its own nym-api-side follow-up, distinct from the producer endpoint above (that one signs snapshots; this one establishes who can be asked to). Protocol details - including a challenge-response mechanism so a caller can confirm live possession of the claimed key, not just an unauthenticated claim - are deferred to that follow-up.
- **Generalized canonical-subset attestation** (per-label subsets, a `Tier1SignerSet` subset derived from coconut-dkg dealers and curated entries, and eventually non-directory nym-api endpoints): explored at length during design but deferred to a named follow-up change. See `design.md`'s "Future direction" section for what was worked out, so it is not lost.
