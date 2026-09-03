## 1. Dependency and feature setup

- [x] 1.1 Add `tendermint-light-client = "0.40.4"` to `[workspace.dependencies]` in root `Cargo.toml`
- [x] 1.2 Add `light-client` feature to `nym-directory-client/Cargo.toml` with `tendermint-light-client` as an optional
  dep under that feature
- [x] 1.3 Verify `cargo check -p nym-directory-client` (no feature) and
  `cargo check -p nym-directory-client --features light-client` both compile clean

## 2. Checkpoint and options types

- [x] 2.1 Define
  `Checkpoint { height: Height, signed_header: SignedHeader, validators: ValidatorSet, next_validators: ValidatorSet }`
  in `src/anchor/light_client.rs` (behind `#[cfg(feature = "light-client")]`)
- [x] 2.2 Add `fetch_checkpoint(client: &C, height: Height) -> Result<Checkpoint, ...>` async helper that calls
  `commit(height)` + `validators(height, Paging::All)` + `validators(height+1, Paging::All)` and assembles the struct
- [x] 2.3 Add `nym_default_options() -> Options` helper with Nym mainnet trusting period (confirm unbonding period) and
  a reasonable clock_drift

## 3. LightClientAnchor core

- [x] 3.1 Define
  `LightClientAnchorState { trusted_height: Height, trusted: TrustedBlockState, app_hash_cache: BTreeMap<Height, AppHash> }` (
  private inner struct)
- [x] 3.2 Define
  `LightClientAnchor<C> { client: C, directory_contract: AccountId, state: tokio::sync::Mutex<LightClientAnchorState>, options: Options, verifier: ProdVerifier }` (
  or equivalent)
- [x] 3.3 Implement `LightClientAnchor::new(client, directory_contract, checkpoint, options) -> Self` that initialises
  `trusted` from the checkpoint's validator set and height
- [x] 3.4 Implement private `try_verify_direct(state, target_height)` that fetches `commit(target_height)` +
  `validators(target_height)`, constructs `UntrustedBlockState`, calls
  `verifier.verify(untrusted, trusted, options, now)`, and on `Verdict::Success` advances `trusted` and inserts into
  `app_hash_cache`; returns `Ok(true)` on success, `Ok(false)` on insufficient-overlap failure, `Err` on hard errors
- [x] 3.5 Implement private `advance_to(state, target_height)` that calls `try_verify_direct(target_height)` first; on
  `Ok(false)` bisects by recursing: `advance_to(mid)` then `advance_to(target)` where
  `mid = (current_trusted + target) / 2`; log each bisection level

## 4. DirectoryTrustAnchor impl

- [x] 4.1 Implement `trusted_app_hash(H)`: lock state, check cache for `H+1`, if miss call `advance_to(H+1)`, return
  `app_hash_cache[H+1]`
- [x] 4.2 Implement `trusted_digest(H)`: delegate to `trusted_app_hash(H)` then prove digest_state via ICS23 (same as
  `ProvenTrustAnchor::trusted_digest`)
- [x] 4.3 Re-export `LightClientAnchor`, `Checkpoint`, `fetch_checkpoint`, `nym_default_options` from
  `src/anchor/mod.rs` under `#[cfg(feature = "light-client")]`

## 5. Error handling

- [x] 5.1 Add `LightClientVerificationFailed(String)` variant to `DirectoryClientError` (or a dedicated sub-error)
  covering invalid signature set, stale checkpoint (trusting period expired), and unexpected `Verdict` arms

## 6. Tests

- [x] 6.1 Unit test: `try_verify_direct` with a valid signed header advances the trusted state and caches the app hash
- [x] 6.2 Unit test: `advance_to` with a header whose validator overlap is insufficient triggers bisection (mock client
  records call heights; verify midpoint was fetched before target). Implemented via a tampered checkpoint whose
  `next_validators` is padded with a dominant non-signing fake validator, forcing every skip hop below the 1/3 overlap
  threshold. The `commit_calls()` sequence `[898, 897, 898]` proves the target was attempted, the midpoint fetched, then
  the target retried. Needs two extra real fixtures (commit@24499898, validators@24499899) - see the `REPLACE_ME_*`
  placeholders in `bisection_fixtures`.
- [x] 6.3 Unit test: `advance_to` with stable validator set resolves in a single direct verification (no bisection)
- [x] 6.4 Unit test: repeated `trusted_app_hash(H)` for the same `H` returns from cache without a second RPC call
- [x] 6.5 Unit test: stale checkpoint (time > now - trusting_period) causes `trusted_app_hash` to fail
- [x] 6.6 Unit test (regression): a below-head query re-verifies from the checkpoint instead of erroring; a
  below-checkpoint query returns `HeightBelowCheckpoint`

## 7. Verification

- [x] 7.1 `cargo test -p nym-directory-client --features light-client --lib` passes (15 tests)
- [x] 7.2 `cargo test -p nym-directory-client --lib` (no feature) passes (8 tests)
- [x] 7.3 `cargo build -p nym-directory-client` and `cargo build -p nym-directory-client --features light-client` both
  succeed
