## 1. Crate scaffolding

- [ ] 1.1 Create `common/cosmwasm-smart-contracts/directory-contract` (msg, types, error, constants, lib), mirroring `node-families-contract`
- [ ] 1.2 Create `contracts/directory` (contract, transactions, queries, storage, helpers, lib, schema bin), mirroring `contracts/node-families`
- [ ] 1.3 Add `lthash-rs` + `sha3` + `prost` deps; pin `rust-version` for MSRV 1.86; confirm a minimal `cargo wasm` build passes `cosmwasm-check` (with the `wasm-opt` lowering)

## 2. Storage model

- [ ] 2.1 Define the extensible namespace discriminant and distinct cw-storage-plus maps for the `node` and `curated` classes (raw keys must not collide)
- [ ] 2.2 `NodeEntry { data, updated_at, signature }` and `CuratedEntry { data }`; entry payloads encoded with prost (`BTreeMap` maps only)
- [ ] 2.3 Persistent per-node `sequence` map (`u64`), surviving entry deletion
- [ ] 2.4 Allowed-labels map (`label -> LabelConfig { max_size }`) + 128 KiB ceiling constant
- [ ] 2.5 Global digest `Item`; canonical leaf encoder `canonical(namespace, id, label, value)` (length-prefixed)
- [ ] 2.6 Instantiate config: admin, mixnet contract address, initial label set

## 3. Digest

- [ ] 3.1 Wire LtHash16 (`lthash-rs` + SHAKE) with O(1) insert/remove helpers
- [ ] 3.2 Apply the digest delta on every write and delete (subtract old leaf, add new leaf)
- [ ] 3.3 Determinism + correctness tests: stable encoding, recompute-equals-stored, cross-namespace non-collision

## 4. Node write / auth path

- [ ] 4.1 Cross-query mixnet (`MixnetContractQuerier`) for existence (bonded and not unbonding) and the base58 identity key; decode to 32 bytes
- [ ] 4.2 Verify the ed25519 signature over `node_id || label || sequence || data` via `deps.api.ed25519_verify`
- [ ] 4.3 Enforce strict per-node sequence increase; reject stale/replayed/cross-slot-lifted signatures (including replay-after-delete)
- [ ] 4.4 Validate label allowed + `data` within `max_size`; store entry (+ signature + `updated_at`); update digest
- [ ] 4.5 Node self-delete handler (signed, sequence-advancing); update digest

## 5. Admin path

- [ ] 5.1 `AddLabel` / `SetLabel` / `RemoveLabel` (admin-only; ceiling enforced; removal non-destructive)
- [ ] 5.2 `SetCuratedEntry` / `RemoveCuratedEntry` (admin-only); update digest

## 6. Unbond callback (cross-contract)

- [ ] 6.1 Directory `OnNymNodeUnbond { node_id }` handler gated by `UnauthorisedMixnetCallback` (sender must be the configured mixnet contract)
- [ ] 6.2 Delete the node's entries via bounded prefix iteration + digest deltas; make it idempotent
- [ ] 6.3 Mixnet contract: add `directory_contract_address` to `State` + `InstantiateMsg` + a queued migration
- [ ] 6.4 Mixnet contract: emit a best-effort (reply-on-error, non-fatal) `OnNymNodeUnbond` sub-message in the unbond handler

## 7. Queries

- [ ] 7.1 `entry`, `entries_for`, paginated `all_entries` (both namespaces), `digest`, `sequence`, `allowed_labels`, `config`
- [ ] 7.2 Confirm provable reads: the digest `Item` and individual entries at deterministic raw keys (document the key layout for clients)

## 8. Tests

- [ ] 8.1 Write/auth: valid write, invalid signature, stale/replayed sequence, replay-after-delete, unknown/unbonding node, disallowed label, oversized data, any-relayer
- [ ] 8.2 Admin: label add/ceiling/non-admin/non-destructive-remove; curated set/remove/non-admin
- [ ] 8.3 Digest: update on write/delete, identical-data no-op, recompute-equals-stored, cross-namespace non-collision
- [ ] 8.4 App-level test for the mixnet unbond -> directory callback (deps-level handler tests do not dispatch sub-messages)
- [ ] 8.5 `cosmwasm-check` the optimized artifact

## 9. Schema + client wiring

- [ ] 9.1 Generate the JSON schema (schema-gen bin)
- [ ] 9.2 validator-client: directory query/signing traits (mirroring the dkg/node-families patterns)
