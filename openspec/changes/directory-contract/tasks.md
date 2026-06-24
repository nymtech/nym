## 1. Crate scaffolding

- [ ] 1.1 Create `common/cosmwasm-smart-contracts/directory-contract` (msg, types, error, constants, lib), mirroring `node-families-contract`
- [ ] 1.2 Create `contracts/directory` (contract, transactions, queries, storage, helpers, lib, schema bin), mirroring `contracts/node-families`
- [ ] 1.3 Add `prost` dep and depend on the in-house `lthash` crate (group 2, not `lthash-rs`); pin `rust-version` for MSRV 1.86; confirm a minimal `cargo wasm` build passes `cosmwasm-check` (with the `wasm-opt` lowering)

## 2. LtHash common library

- [x] 2.1 Created `no_std` `nym-lthash` crate at `common/lthash`, generic over `digest::ExtendableOutput` (digest 0.10 - the workspace pins blake3 `<1.8.4` to stay on digest 0.10). Deps: `digest` (workspace) + `blake3` declared directly (`<1.8.4`, `default-features=false` + `traits-preview` - cargo forbids overriding default-features on an inherited dep). Registered in root members; `rust-version` pinned 1.86 (contracts MSRV). Rationale: `lthash-rs` unmaintained, `commonware-cryptography` not feature-scopable
- [x] 2.2 Implemented generic `LtHash<X: ExtendableOutput>` (1024 x 16-bit lanes = 2 KB): `insert`/`remove`/`Add`/`Sub`/`to_bytes`/`from_bytes` + Default/Clone/PartialEq/Debug; concrete `LtHash16 = LtHash<blake3::Hasher>`
- [x] 2.3 Tests: homomorphic invariants (insert+remove = identity, order-independence, Add == union, Sub undoes Add), empty = zero, byte round-trip
- [x] 2.4 Differential-tested the lane math against `sha3 0.10` (Shake256) as a dev-only XOF (same digest 0.10 trait); multiset-collision caveat documented in the crate docs
- [x] 2.5 `cargo test -p nym-lthash` green (3 tests); blake3-via-digest-0.10 + a contract using it validated to pass `cosmwasm-check`. Workspace stays on digest 0.10 (a full 0.11 upgrade is upstream-gated and unneeded)

## 3. Storage model

- [ ] 3.1 Define the extensible namespace discriminant and distinct cw-storage-plus maps for the `node` and `curated` classes (raw keys must not collide)
- [ ] 3.2 `NodeEntry { data, updated_at, signature }` and `CuratedEntry { data }`; entry payloads encoded with prost (`BTreeMap` maps only)
- [ ] 3.3 Persistent per-node `sequence` map (`u64`), surviving entry deletion
- [ ] 3.4 Allowed-labels map (`label -> LabelConfig { max_size }`) + 128 KiB ceiling constant
- [ ] 3.5 Store the full `LtHash16` state as the accumulator `Item` (~2 KB, mutated O(1) per write); expose & ICS23-prove the compact 32-byte `LtHash16::out()` (blake3 collapse) as the public digest (comparison-only, not homomorphic). Canonical leaf encoder `canonical(namespace, id, label, value)` (length-prefixed) lives in/alongside `nym-lthash` so contract and client agree byte-for-byte
- [ ] 3.6 Instantiate config: admin, mixnet contract address, initial label set

## 4. Digest

- [ ] 4.1 Wire the in-house LtHash16 with O(1) insert/remove helpers
- [ ] 4.2 Apply the digest delta on every write and delete (subtract old leaf, add new leaf)
- [ ] 4.3 Determinism + correctness tests: stable encoding, recompute-equals-stored, cross-namespace non-collision

## 5. Node write / auth path

- [ ] 5.1 Cross-query mixnet (`MixnetContractQuerier`) for existence (bonded and not unbonding) and the base58 identity key; decode to 32 bytes
- [ ] 5.2 Verify the ed25519 signature over `node_id || label || sequence || data` via `deps.api.ed25519_verify`
- [ ] 5.3 Enforce strict per-node sequence increase; reject stale/replayed/cross-slot-lifted signatures (including replay-after-delete)
- [ ] 5.4 Validate label allowed + `data` within `max_size`; store entry (+ signature + `updated_at`); update digest
- [ ] 5.5 Node self-delete handler (signed, sequence-advancing); update digest

## 6. Admin path

- [ ] 6.1 `AddLabel` / `SetLabel` / `RemoveLabel` (admin-only; ceiling enforced; removal non-destructive)
- [ ] 6.2 `SetCuratedEntry` / `RemoveCuratedEntry` (admin-only); update digest

## 7. Unbond callback (cross-contract)

- [ ] 7.1 Directory `OnNymNodeUnbond { node_id }` handler gated by `UnauthorisedMixnetCallback` (sender must be the configured mixnet contract)
- [ ] 7.2 Delete the node's entries via bounded prefix iteration + digest deltas; make it idempotent
- [ ] 7.3 Mixnet contract: add `directory_contract_address` to `State` + `InstantiateMsg` + a queued migration
- [ ] 7.4 Mixnet contract: emit a best-effort (reply-on-error, non-fatal) `OnNymNodeUnbond` sub-message in the unbond handler

## 8. Queries

- [ ] 8.1 `entry`, `entries_for`, paginated `all_entries` (both namespaces), `digest`, `sequence`, `allowed_labels`, `config`
- [ ] 8.2 Confirm provable reads: the digest `Item` and individual entries at deterministic raw keys (document the key layout for clients)

## 9. Tests

- [ ] 9.1 Write/auth: valid write, invalid signature, stale/replayed sequence, replay-after-delete, unknown/unbonding node, disallowed label, oversized data, any-relayer
- [ ] 9.2 Admin: label add/ceiling/non-admin/non-destructive-remove; curated set/remove/non-admin
- [ ] 9.3 Digest: update on write/delete, identical-data no-op, recompute-equals-stored, cross-namespace non-collision
- [ ] 9.4 App-level test for the mixnet unbond -> directory callback (deps-level handler tests do not dispatch sub-messages)
- [ ] 9.5 `cosmwasm-check` the optimized artifact

## 10. Schema + client wiring

- [ ] 10.1 Generate the JSON schema (schema-gen bin)
- [ ] 10.2 validator-client: directory query/signing traits (mirroring the dkg/node-families patterns)
