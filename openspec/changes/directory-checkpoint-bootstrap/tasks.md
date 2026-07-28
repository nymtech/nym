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

- [ ] 4.1 Define the `CheckpointProvider` abstraction (each impl yields a trusted, non-stale `Checkpoint` or nothing)
- [ ] 4.2 Implement the stored provider (reads the persisted head from the `CheckpointStore`; staleness-checked; no root
  sig) and the hardcoded-constant provider (reads and root-verifies the serialized datum from the `network-defaults`
  constant)
- [ ] 4.3 Implement the HTTPS provider fetching + root-verifying from a configurable, env-overridable well-known URL (
  mirror `UPGRADE_MODE_ATTESTATION_URL` -> `.wellknown/directory/checkpoint.json`)
- [ ] 4.4 Implement the loader: try providers in order stored -> hardcoded -> HTTPS, first valid (non-stale,
  sig-verified) wins; derive staleness (`header_time + trusting_period < now`); build `LightClientAnchor` from the
  chosen base; typed errors when all sources fail, no anchor constructed
- [ ] 4.5 Tests (test root key + `MockRpcClient` nyx fixtures): fresh stored head preferred (no network); fresh-boot
  falls back to hardcoded; aged-out seed falls back to HTTPS; bad-sig source rejected; stale checkpoint rejected at
  load; `created_at` does not affect validity

## 5. Verified-head persistence (`nym-directory-client`)

- [ ] 5.1 Define `CheckpointStore { load() -> Option<Checkpoint>, save(&Checkpoint) }`; add a file-backed impl (JSON)
  and a no-op/in-memory impl for tests (shared by the stored provider's read side and the anchor's write side)
- [ ] 5.2 Add `LightClientAnchor::new_with_store(base, store)` where `base` is the checkpoint the loader selected; the
  anchor writes its advanced head to the store (write side only - source selection is the loader's job)
- [ ] 5.3 Persist the advanced head once per producer refresher tick (not per hop)
- [ ] 5.4 Tests: advanced head is written to the store; anchor without a store still verifies/advances; a stored head
  written by one anchor is read back and used to seed a fresh loader run

## 6. Offline minting dev-binary

- [ ] 6.1 Create a maintainer-only binary (not the user-facing nym-cli) that reuses `fetch_checkpoint` + the datum
  encoder + `nym-crypto` signing
- [ ] 6.2 Args via `clap`: root private key as an arg with `env = "..."`; trusted `--rpc`; `--minted-at` override;
  height/pin flag
- [ ] 6.3 Self-verify before writing: construct a `LightClientAnchor` from the minted checkpoint and advance one hop;
  abort on failure
- [ ] 6.4 Regenerate the dedicated constant file wholesale and emit a `// minted <time> from height <h> via <rpc>`
  header comment
- [ ] 6.5 Test: mint -> verify round-trip with a test key; deterministic output given pinned inputs

## 7. Producer wiring (`nym-api/src/directory/cache`)

- [ ] 7.1 Replace the `bail!("unimplemented external checkpoint retrieval")` with the checkpoint loader building a
  `LightClientAnchor` (with a `CheckpointStore` in the node data dir, alongside the existing `on_disk_file` cache)
- [ ] 7.2 Ensure the proven-RPC path (`trusted_rpc_node`) remains the default/rollback path
- [ ] 7.3 Test/verify the producer constructs a light-client source anchor from a (test-key) checkpoint and persists its
  verified head

## 8. Verification gates

- [ ] 8.1 `cargo build`/`check` across touched crates (`nym-directory-attestation`, `nym-directory-client` with and
  without `light-client`, `nym-api`, `network-defaults`, upgrade-mode consumers)
- [ ] 8.2 Run the new + existing directory-client tests (including the `mocks`/`light-client` feature paths)
- [ ] 8.3 `openspec validate directory-checkpoint-bootstrap --strict`

## 9. Deferred: real-key mainnet ceremony (gated on root-key access)

- [ ] 9.1 Run the minting tool with the real mainnet root key to regenerate and commit the hardcoded checkpoint constant
  file
- [ ] 9.2 Publish the `.wellknown/directory/checkpoint.json` file
- [ ] 9.3 Optionally mint dev-key-signed checkpoints for non-mainnet networks
