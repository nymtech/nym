## Why

The current `ProvenTrustAnchor` sources `app_hash` from a plain RPC `header[H+1]` call with no validator-set verification, which means the trust root is only as trustworthy as the RPC server. Replacing that source with a real CometBFT / Tendermint light client - one that verifies >2/3 of the validator set signed each header - removes the honest-RPC assumption for the `trusted_app_hash` call and completes the security model described in `project_directory_contract_trust_model_2026_06_24`.

## What Changes

- Add a new `LightClientAnchor` struct in `common/nym-directory-client/src/anchor/light_client.rs` that implements `DirectoryTrustAnchor` using `tendermint-light-client` for header verification.
- The existing `ProvenTrustAnchor` is kept unchanged for local-dev / testing contexts (no honest-RPC assumption required there); `LightClientAnchor` becomes the recommended production anchor.
- `LightClientAnchor::trusted_app_hash(H)` verifies the header chain from a pinned trusted checkpoint up to `H+1` and returns `header[H+1].app_hash` only after the validator-set signature check passes.
- `LightClientAnchor::trusted_digest(H)` delegates to `trusted_app_hash` (same as `ProvenTrustAnchor`).
- New Cargo dependency: `tendermint-light-client` (from the `informalsystems/tendermint-rs` workspace).
- `DirectoryTrustAnchor` trait surface is unchanged.

## Capabilities

### New Capabilities

- `tendermint-light-client-anchor`: A `DirectoryTrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol (validator-set consensus + signed header checks) starting from a pinned trusted checkpoint, so that `trusted_app_hash` does not require an honest RPC.

### Modified Capabilities

- `directory-retrieval-client`: gains `LightClientAnchor` as a second production anchor behind the same `DirectoryTrustAnchor` trait (the whole-directory and single-entry retrieval paths are unchanged).

## Impact

- `common/nym-directory-client/`: new `src/anchor/light_client.rs`, updated `src/anchor/mod.rs` (re-export), updated `Cargo.toml` (add `tendermint-light-client` dep).
- No changes to `DirectoryClient`, `verify.rs`, `proof.rs`, `key.rs`, or any contract code.
- New optional feature flag on `nym-directory-client` likely needed since `tendermint-light-client` is a heavier dep (skip in WASM / contract builds).
- Consumers that want the production anchor swap `ProvenTrustAnchor::new(client, addr)` for `LightClientAnchor::new(client, addr, checkpoint)`.
