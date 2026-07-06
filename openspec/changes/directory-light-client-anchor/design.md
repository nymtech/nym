## Context

`ProvenTrustAnchor` currently calls `self.client.header(H+1)` directly and trusts the returned `app_hash` without verifying that >2/3 of the Nym validator set signed that block. If the RPC is compromised or adversarial, it can serve a forged header with a fabricated `app_hash`, defeating the ICS23 proof layer entirely. The `DirectoryTrustAnchor` trait was designed as a seam exactly so this source can be upgraded without touching `DirectoryClient` or the verify core.

The `tendermint-light-client = "0.40.4"` crate (same release family as the `tendermint = "0.40.4"` already in the workspace) provides `PredicateVerifier` - a pure function `verify_update_header(untrusted, trusted, options, now) -> Verdict` that checks whether a new signed header is supported by >2/3 of the current trusted validator set. Phase 1b wires this into a `LightClientAnchor` that maintains a "most recent trusted block" in memory and steps forward one block at a time.

## Goals / Non-Goals

**Goals:**
- Add `LightClientAnchor<C>: DirectoryTrustAnchor` that verifies every block header it returns against the Tendermint validator-set consensus rule before extracting `app_hash`.
- Keep `ProvenTrustAnchor` unchanged (useful for local-dev / tests where the RPC is trusted by construction).
- Gate `LightClientAnchor` behind an optional `light-client` feature flag so contract builds and WASM targets don't pull in the heavier dep.
- The `DirectoryClient` and verify core are entirely untouched.

**Non-Goals:**
- Persistent trusted-state storage. The in-memory trusted block resets on process restart; the operator must provide a fresh checkpoint.
- Multi-peer supervisor mode. A single RPC provider is used; operator is responsible for choosing a reputable one (this is still significantly stronger than raw header trust).
- Auto-fetching the trusting period from chain genesis; the operator configures `Options` explicitly.

## Decisions

### D1: Use `tendermint-light-client` directly, not the Supervisor

The `Supervisor` (multi-peer, bisecting) is the production-grade entry point in the crate, but it requires async IO adapters, peer management, and a storage backend - substantial scaffolding for a first iteration. `PredicateVerifier` (or equivalently `ProdVerifier`) is a pure verifier struct with no async or IO; we call it directly in our own `trusted_app_hash` implementation, stepping one header at a time. Phase 2 can layer the Supervisor on top.

Alternative considered: use the Supervisor with a single-peer in-memory store. Rejected: the adapter API surface is large and the single-peer case buys nothing over our simpler direct approach.

### D2: Skip verification with bisection fallback

To supply `app_hash` at `H` when the trusted state is at `T < H+1`, we first attempt to verify `H+1` directly from `T`. `PredicateVerifier::verify` already handles non-adjacent heights: if the voting power that the trusted validator set contributes to the commit at `H+1` exceeds the trust threshold (1/3 by default), the block is accepted in one shot without touching any intermediate header.

If that direct attempt fails due to insufficient overlap (validator set changed enough that <1/3 of trusted power signed), we bisect: verify the midpoint `M = (T + H+1) / 2` first, update the trusted state to `M`, then retry `H+1` from `M`. This recurses until the gap is small enough (typically 1-2 levels for Nym's stable, small validator set).

Complexity: O(log delta) RPC calls in the worst case; O(1) when the validator set is stable - which covers the typical Nym scenario. A checkpoint 1000 blocks behind costs at most ~10 bisection hops, usually 1.

The implementation is a simple recursive (or iterative) `advance_to(target)` that tries direct verification, then bisects on failure - around 30 lines, no Supervisor required. Each step fetches `commit(K)` + `validators(K)` from the RPC.

Alternative considered: sequential stepping (verify every intermediate block). Rejected: O(delta) is unnecessary given the protocol already supports skip verification, and for a stale checkpoint the cost is prohibitive.

### D3: In-memory cache of verified `AppHash` values

Once a header at height `K` is verified, its `app_hash` is immutable. We store verified `(Height, AppHash)` pairs in a `BTreeMap` inside the mutable anchor state. Repeated queries for the same `H` (e.g., digest proof + single-entry read in the same session) pay only one round of header fetching.

### D4: `Checkpoint` as the constructor argument

The caller provides a `Checkpoint { height: Height, signed_header: SignedHeader, validators: ValidatorSet, next_validators: ValidatorSet }` at construction. The caller is responsible for obtaining this from a trusted source (e.g., a genesis-pinned block distributed with the binary, or an operator-attested recent block). We do not auto-fetch it; fetching the checkpoint from the same RPC we're trying to not trust would be circular.

The `next_validators` field is required by `TrustedBlockState` to verify the next header.

### D5: Feature flag `light-client` on `nym-directory-client`

`tendermint-light-client` is a heavy dep with no-std limitations. Gate it behind `features = ["light-client"]`. The `LightClientAnchor` type and its import are under `#[cfg(feature = "light-client")]`. Consumers opt in explicitly.

### D6: `Options` supplied by caller; no auto-detection

`tendermint_light_client_verifier::Options` requires `trusting_period` and `clock_drift`. These are chain-specific parameters (Nym's unbonding period sets the trusting period ceiling). Rather than hardcoding or fetching from genesis, the caller supplies an `Options` value. We expose a `nym_default_options()` helper with sane defaults for the Nym mainnet trusting period.

## Risks / Trade-offs

- [Cold-start with very stale checkpoint] → Mitigation: bisection keeps this O(log delta), so even 10,000 blocks behind is ~14 hops. For Nym's stable validator set it is typically 1 hop regardless of delta. Log progress at each bisection level so operators can see what is happening.
- [Process restart resets trusted state] → Mitigation: document that a fresh checkpoint must be provided at startup; phase 2 adds persistence. For many use cases (short-lived CLI clients, nym-api restart) the checkpoint simply comes from a well-known recent block bundled with the binary.
- [Single-RPC still trusted for header data (just not for the `app_hash` value)] → Residual: a Byzantine RPC can withhold headers (DoS) but cannot forge a valid signed header since it lacks the validator private keys. A DoS is detectable; a forgery is not. This is a meaningful improvement over the current model.
- [`next_validators` at checkpoint requires two RPC calls at setup] → Mitigation: the `Checkpoint` struct is constructed once; a helper `fetch_checkpoint(client, height)` in the anchor module fetches and assembles it.

## Migration Plan

1. Add `tendermint-light-client = "0.40.4"` under `[workspace.dependencies]` and to `nym-directory-client/Cargo.toml` gated by the `light-client` feature.
2. Implement `LightClientAnchor` in `src/anchor/light_client.rs` behind `#[cfg(feature = "light-client")]`.
3. Re-export `LightClientAnchor` and `Checkpoint` from `src/anchor/mod.rs` under the same cfg gate.
4. No changes to `DirectoryClient`, `ProvenTrustAnchor`, or any consumer; the swap is purely at construction time.
5. Document in the crate README: when to use `ProvenTrustAnchor` (local-dev / tests) vs. `LightClientAnchor` (production).

## Open Questions

- Should we expose a `step_to(height)` method so callers can pre-warm the cache before the first `verified_directory` call? Probably useful for startup latency; defer to implementation.
- Trusting-period defaults for Nym mainnet: need to confirm the unbonding period. Placeholder: 21 days (a common Cosmos default).
