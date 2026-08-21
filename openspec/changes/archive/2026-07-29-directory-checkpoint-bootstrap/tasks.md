## 1. Datum format and verification (`nym-directory-client`)

- [x] 1.1 Define `SignedCheckpoint { checkpoint: Checkpoint, created_at, root_signature }` (alongside `Checkpoint` in
  `nym-directory-client`) with an advisory `created_at` using the `time::serde::rfc3339` pattern
- [x] 1.2 Build the signing payload `domain_tag || chain_id || height || blake3(proto_encode(checkpoint))` using
  `nym-directory-attestation`'s domain-tag helpers for the wrapper and Tendermint's `Protobuf` encoding for the
  checkpoint; use a checkpoint domain tag distinct from upgrade-mode/snapshot/subset/node-entry
- [x] 1.3 Implement root-signature verification over the payload (recompute the proto commitment on verify); add a typed
  error for signature failure
- [x] 1.4 Unit tests: sign+verify round-trip with a test root key; wrong-key and tampered-checkpoint rejection;
  signer/verifier produce identical committed bytes; domain-separation (tag is load-bearing). Shared real-nyx
  `Checkpoint` fixture relocated from `light_client.rs` tests into `test_support` (single-sourced; both suites reuse it)

## 2. Root-key rename and backward-compatible aliasing (`common/network-defaults`)

- [x] 2.1 Rename the shared const (`mainnet.rs`) and env-var-name (`var_names.rs`) from `UPGRADE_MODE_ATTESTER_*` to a
  domain-neutral `ROOT_ATTESTER_*`
- [x] 2.2 Add a reader that resolves the new canonical env var and falls back to the legacy
  `UPGRADE_MODE_ATTESTER_ED25519_PUBKEY`; register both names in `setup_env` and `export_to_env_if_not_set`
- [x] 2.3 Update the upgrade-mode consumers (`gateway_tasks.rs`, `old_config_v12.rs`, credential-proxy) to the renamed
  accessor; leave the nym-node `--upgrade-mode-attester-public-key` / `NYMNODE_UPGRADE_MODE_ATTESTER_PUBKEY` override
  knobs unchanged
- [x] 2.4 Add a dedicated regenerated constant file (`mainnet/directory_checkpoint.rs`, empty placeholder = "no compiled
  checkpoint") + `var_names::DIRECTORY_CHECKPOINT`; wired into `export_to_env` and `export_to_env_if_not_set`
- [x] 2.5 Test (network-defaults): legacy env var alone is promoted to the canonical name; canonical takes precedence
  when both set. Also fixed a shadowing bug - added the legacy->canonical promotion in
  `fix_deprecated_environmental_variables` (mirroring the `NYXD`/`NYM_API` pattern), without which non-mainnet `.env`
  files (which still use the legacy name) were shadowed by the mainnet-default backfill of the canonical name

## 3. Trusting period and single source of truth

- [x] 3.1 Raise the production trusting period to 18 days in `nyx_default_options` (`anchor/light_client.rs`)
- [x] 3.2 Expose the trusting period from one location (`NYX_TRUSTING_PERIOD` const in `anchor/light_client.rs`); both
  `nyx_default_options` and the loader's staleness check read it
- [x] 3.3 Add a doc/invariant note that the trusting period must stay strictly below the nyx unbonding period (21 days)
  with margin, on `NYX_TRUSTING_PERIOD`

## 4. Checkpoint providers and loader (`nym-directory-client`)

- [x] 4.1 Define the `CheckpointProvider` abstraction (async `candidate()` yields a verified `Checkpoint` or nothing;
  staleness centralized in the loader) - `anchor/checkpoint_source.rs`
- [x] 4.2 Implement the stored provider (reads the persisted head from the `CheckpointStore`; no root sig) and the
  hardcoded-constant provider (parses + root-verifies the JSON datum; empty = absent)
- [x] 4.3 Implement the HTTPS provider - transport injected via a `CheckpointFetcher` trait (assoc. `Error:
  std::error::Error`; returns the concrete `SignedCheckpoint`), so the lib stays HTTP-free; root-verifies before use.
  The concrete reqwest fetcher + the env-overridable well-known URL are supplied by nym-api in §7
- [x] 4.4 Implement the loader `load_checkpoint(providers, now)`: try in order stored -> hardcoded -> HTTPS, first
  non-stale candidate wins; staleness = `header_time + NYX_TRUSTING_PERIOD <= now`; `NoValidCheckpointSource` if none.
  (Returns the chosen `Checkpoint`; anchor construction is the caller's job via §5.2)
- [x] 4.5 Tests: stored preferred over hardcoded; fresh-boot falls back to hardcoded; aged-out seeds fall back to HTTPS;
  bad-sig hardcoded + HTTPS rejected; stale candidates yield no source; HTTPS transport failure ignored

## 5. Verified-head persistence (`nym-directory-client`)

- [x] 5.1 Define `CheckpointStore { load() -> Option<Checkpoint>, save(&Checkpoint) }`; `FileCheckpointStore` (JSON) +
  `InMemoryCheckpointStore` for tests (shared by the stored provider's read side and the anchor's write side) -
  `anchor/checkpoint_source.rs`
- [x] 5.2 Add `LightClientAnchor::new_with_store(base, store)` (write side; `new` delegates with no store).
  `verify_hop` now returns the verified block as a `Checkpoint`, `walk_to` returns the head checkpoint it reached, so
  the anchor persists the exact verified head (not a re-fetch, which a lying RPC could poison)
- [x] 5.3 Persist on the forward `advance_to` branch only - once per forward advance, after bisection settles (not per
  hop); a below-head query (throwaway clone) never persists. Producer-side per-tick cadence follows in §7
- [x] 5.4 Tests: advanced head is persisted (file store) and reseeds a fresh loader run via the stored provider; anchor
  without a store still verifies/advances

## 6. Offline minting dev-binary

- [x] 6.1 Maintainer-only binary `tools/internal/nyx-checkpoint-updater` (not the user-facing nym-cli), reusing
  `Checkpoint::fetch` (= `fetch_checkpoint`) + `SignedCheckpoint::new` (datum encoder + `nym-crypto` signing)
- [x] 6.2 Args via `clap`: root private key with `env`; trusted `--rpc`; `--minted-at` override; `--height` pin
  (default `latest - 2`, so both the checkpoint's `height + 1` set and the `height + 2` block the self-verify hop needs
  are already committed); plus `--repo-root`/`--out` for locating the datum file
- [x] 6.3 Self-verify before writing: advance the minted checkpoint one light-client hop via
  `verify_checkpoint_advances_one_hop` (extracted from `verify_hop`; no `LightClientAnchor`, hence no dummy directory
  contract address), aborting before persistence on failure
- [x] 6.4 Write the `SignedCheckpoint` datum wholesale as JSON to the sibling `directory_checkpoint.json` (embedded by a
  static `directory_checkpoint.rs` via `include_str!`) - no Rust source is rewritten and no header comment is emitted:
  provenance (`created_at` + height) is authenticated inside the datum, and the source `--rpc` belongs in the commit
  message
- [x] 6.5 Tool tests dropped as redundant - mint->verify round-trip, wrong-key/tamper rejection, JSON datum parse-back +
  root-signature verification, and serde determinism are all covered by `nym-directory-client` lib tests
  (`checkpoint/mod.rs` §1.4 and `checkpoint/provider.rs`), which already own the shared real-nyx fixture. `fetch` and the
  self-verify hop ride on the existing `light_client.rs` anchor tests

## 7. Producer wiring (`nym-api/src/directory/cache`)

- [x] 7.1 Replaced the `bail!("unimplemented external checkpoint retrieval")`: the light-client branch resolves the
  root pubkey, checkpoint datum and well-known URL from env (`ROOT_ATTESTER_ED25519_PUBKEY` / `DIRECTORY_CHECKPOINT` /
  `NYX_TRUSTED_CHECKPOINT_URL`, backfilled from the compiled-in mainnet consts by `setup_env`), runs `load_checkpoint`
  over the full stored -> hardcoded -> HTTPS provider chain, and builds a `LightClientAnchor` seeded with the result +
  a `FileCheckpointStore` at `directory_checkpoint_head.json` alongside the on-disk directory cache. The HTTPS provider
  is optional and non-fatal (`build_https_provider` returns `None` for an empty/malformed URL or unbuildable fetcher, so
  it can never short-circuit the higher-priority sources). Refactored into `AnchorWithChainId` + `build_*` helpers; the
  proven-RPC path (7.2) is untouched.
- [x] 7.2 Proven-RPC path (`trusted_rpc_node`) remains the default/rollback path: the `if config.debug.trusted_rpc_node`
  branch (`build_proven_trust_anchor`) is unchanged; only the former `bail!` else-branch was implemented
- [x] 7.3 Dropped as redundant (like 6.5). The nym-api producer uses the concrete `QueryHttpRpcNyxdClient`, so an
  end-to-end test would need a mock tendermint JSON-RPC HTTP server + multi-height block/validator/contract fixtures,
  yet would only re-prove behavior already covered where a `MockRpcClient` can be injected: anchor construction +
  forward advance + head persistence (`light_client.rs`), provider ordering/staleness/bad-sig (`provider.rs`), and datum
  sign/verify (`checkpoint/mod.rs`). The env->provider resolution glue is the only nym-api-specific logic and is not
  worth a process-global env-var test

## 8. Verification gates

- [x] 8.1 `cargo build` green across every touched crate: `nym-directory-attestation`; `nym-directory-client` both
  without features and with `light-client,https-checkpoint-fetcher`; `nym-network-defaults`; `nyx-checkpoint-updater`;
  `nym-api`; and `nym-node` (upgrade-mode consumer)
- [x] 8.2 `cargo test -p nym-directory-client --features light-client,https-checkpoint-fetcher` = 68 passed / 0 failed
  (covers the mock-RPC anchor, provider-ordering, persistence and datum sign/verify paths); `nym-network-defaults` test
  (legacy attester env promotion) green
- [x] 8.3 `openspec validate directory-checkpoint-bootstrap --strict` = valid

## 9. DEFERRED FOLLOW-UP: real-key mainnet ceremony (gated on root-key access)

Not a spec change and not implementable in this change - it is an operational ceremony gated on access
to the mainnet root key, so it is intentionally left unchecked and archived alongside this change as a
standing follow-up. The behavior it activates ("load the checkpoint from the hardcoded datum") is
already specified and archived; §9 only populates the data.

- [ ] 9.1 Run `nyx-checkpoint-updater` with the real mainnet root key against a trusted RPC to regenerate and commit the
  populated `common/network-defaults/src/mainnet/directory_checkpoint.json` datum (currently the empty placeholder)
- [ ] 9.2 Publish the well-known `checkpoint.json` file at the `NYX_TRUSTED_CHECKPOINT_URL` endpoint (mainnet const
  currently the empty placeholder in `common/network-defaults/src/mainnet.rs`)
- [ ] 9.3 Optionally mint dev-key-signed checkpoints for non-mainnet networks
