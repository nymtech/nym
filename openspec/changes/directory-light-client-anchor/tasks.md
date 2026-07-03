## 1. Dependency and feature setup

- [ ] 1.1 Add `tendermint-light-client = "0.40.4"` to `[workspace.dependencies]` in root `Cargo.toml`
- [ ] 1.2 Add `light-client` feature to `nym-directory-client/Cargo.toml` with `tendermint-light-client` as an optional dep under that feature
- [ ] 1.3 Verify `cargo check -p nym-directory-client` (no feature) and `cargo check -p nym-directory-client --features light-client` both compile clean

## 2. Checkpoint and options types

- [ ] 2.1 Define `Checkpoint { height: Height, signed_header: SignedHeader, validators: ValidatorSet, next_validators: ValidatorSet }` in `src/anchor/light_client.rs` (behind `#[cfg(feature = "light-client")]`)
- [ ] 2.2 Add `fetch_checkpoint(client: &C, height: Height) -> Result<Checkpoint, ...>` async helper that calls `commit(height)` + `validators(height, Paging::All)` + `validators(height+1, Paging::All)` and assembles the struct
- [ ] 2.3 Add `nym_default_options() -> Options` helper with Nym mainnet trusting period (confirm unbonding period) and a reasonable clock_drift

## 3. LightClientAnchor core

- [ ] 3.1 Define `LightClientAnchorState { trusted_height: Height, trusted: TrustedBlockState, app_hash_cache: BTreeMap<Height, AppHash> }` (private inner struct)
- [ ] 3.2 Define `LightClientAnchor<C> { client: C, directory_contract: AccountId, state: tokio::sync::Mutex<LightClientAnchorState>, options: Options, verifier: ProdVerifier }` (or equivalent)
- [ ] 3.3 Implement `LightClientAnchor::new(client, directory_contract, checkpoint, options) -> Self` that initialises `trusted` from the checkpoint's validator set and height
- [ ] 3.4 Implement private `step_once(state, next_height)` that fetches `commit(next_height)` + `validators(next_height)` + `validators(next_height+1)`, constructs `UntrustedBlockState`, calls `verifier.verify_update_header(untrusted, trusted, options, now)`, and on `Verdict::Success` advances `trusted` and inserts into `app_hash_cache`
- [ ] 3.5 Implement private `advance_to(state, target_height)` that calls `step_once` for each block from `trusted_height+1` to `target_height`

## 4. DirectoryTrustAnchor impl

- [ ] 4.1 Implement `trusted_app_hash(H)`: lock state, check cache for `H+1`, if miss call `advance_to(H+1)`, return `app_hash_cache[H+1]`
- [ ] 4.2 Implement `trusted_digest(H)`: delegate to `trusted_app_hash(H)` then prove digest_state via ICS23 (same as `ProvenTrustAnchor::trusted_digest`)
- [ ] 4.3 Re-export `LightClientAnchor`, `Checkpoint`, `fetch_checkpoint`, `nym_default_options` from `src/anchor/mod.rs` under `#[cfg(feature = "light-client")]`

## 5. Error handling

- [ ] 5.1 Add `LightClientVerificationFailed(String)` variant to `DirectoryClientError` (or a dedicated sub-error) covering invalid signature set, stale checkpoint (trusting period expired), and unexpected `Verdict` arms

## 6. Tests

- [ ] 6.1 Unit test: constructing `LightClientAnchor` from a checkpoint makes no RPC calls (use a spy/mock client)
- [ ] 6.2 Unit test: `step_once` with a valid signed header advances the trusted state and caches the app hash
- [ ] 6.3 Unit test: `step_once` with an insufficiently-signed header returns `LightClientVerificationFailed`
- [ ] 6.4 Unit test: repeated `trusted_app_hash(H)` for the same `H` returns from cache without a second RPC call
- [ ] 6.5 Unit test: stale checkpoint (time > now - trusting_period) causes `trusted_app_hash` to fail

## 7. Verification

- [ ] 7.1 `cargo test -p nym-directory-client --features light-client --lib` passes
- [ ] 7.2 `cargo test -p nym-directory-client --lib` (no feature) passes
- [ ] 7.3 `cargo build -p nym-directory-client` and `cargo build -p nym-directory-client --features light-client` both succeed
