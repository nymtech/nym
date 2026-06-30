## 1. Crate scaffolding

- [x] 1.1 Created `nym-directory-contract-common` (msg, types, error, constants, helpers, lib) mirroring node-families; builds default + `schema` (14 tests). Has the `Namespace` tag, the `EntryKey` enum (`Node { node_id, label }` | `Curated { key }`) used ONLY as the `AllEntries` cursor/response key and to derive the canonical `digest_leaf` (no storage-key codec - the per-class stores own keys, see 3.1), `NodeEntry { data, updated_at_height, sequence, signature }` / `CuratedEntry` (each with `to_bytes`/`try_from_bytes`) / `LabelConfig` / `KnownLabel`, query responses, and the canonical `node_signing_payload` + per-variant `digest_leaf` encoders (node commits node_id+label+data+signature+sequence; curated commits key+data; height excluded). No `Config` struct, no `CuratedKey` struct (curated key is a plain `String`). Sequence model is gap-free exact-match
- [ ] 1.2 Create `contracts/directory` (contract, transactions, queries, storage, helpers, lib, schema bin), mirroring `contracts/node-families`. Scaffolded: lib + contract entry points (instantiate + migrate done; execute/query stubbed to match arms) + storage definitions/`initialise`; execute/query handlers, queued migrations, and the testing-harness mixnet patch remain
- [ ] 1.3 Depend on the in-house `nym-lthash` crate (group 2, not `lthash-rs`); pin `rust-version` for MSRV 1.86; confirm a minimal `cargo wasm` build passes `cosmwasm-check` (with the `wasm-opt` lowering). (`prost` is NOT a contract dep - it is only for consumer payloads inside `data`, per D-serde.)

## 2. LtHash common library

- [x] 2.1 Created `no_std` `nym-lthash` crate at `common/lthash`, generic over `digest::ExtendableOutput` (digest 0.10 - the workspace pins blake3 `<1.8.4` to stay on digest 0.10). Deps: `digest` (workspace) + `blake3` declared directly (`<1.8.4`, `default-features=false` + `traits-preview` - cargo forbids overriding default-features on an inherited dep). Registered in root members; `rust-version` pinned 1.86 (contracts MSRV). Rationale: `lthash-rs` unmaintained, `commonware-cryptography` not feature-scopable
- [x] 2.2 Implemented generic `LtHash<X: ExtendableOutput>` (1024 x 16-bit lanes = 2 KB): `insert`/`remove`/`Add`/`Sub`/`to_bytes`/`from_bytes` + Default/Clone/PartialEq/Debug; concrete `LtHash16 = LtHash<blake3::Hasher>`
- [x] 2.3 Tests: homomorphic invariants (insert+remove = identity, order-independence, Add == union, Sub undoes Add), empty = zero, byte round-trip
- [x] 2.4 Differential-tested the lane math against `sha3 0.10` (Shake256) as a dev-only XOF (same digest 0.10 trait); multiset-collision caveat documented in the crate docs
- [x] 2.5 `cargo test -p nym-lthash` green (3 tests); blake3-via-digest-0.10 + a contract using it validated to pass `cosmwasm-check`. Workspace stays on digest 0.10 (a full 0.11 upgrade is upstream-gated and unneeded)

## 3. Storage model

- [x] 3.1 TWO per-class stores (`StoredNodeEntries`, `StoredCuratedEntries` in `contracts/directory/src/storage.rs`), following the ecash `StoredDeposits` pattern: cw-storage-plus OWNS the key, only the value is hand-rolled. Each store's key is homogeneous and a `PrimaryKey` - node `(node_id, label)` = `(u32, String)`, curated a single `String` - so keys use `Path` (writes) + `Prefix`/`range_with_prefix`/`KeyDeserialize` (scans), with NO manual key codec, NO `PrimaryKey`-on-enum, NO `prefix_upper_bound`. (This replaced the earlier single-store manual-`EntryKey`-codec design; the enum survives only as the `AllEntries` cursor + digest-leaf key.) Helpers per store: `save`/`may_load`/`remove`/`range(min, max, order)` (lazy iterator over cw-storage-plus `Bound`s - node bounds by the full `(NodeId, String)` composite key, curated by `String`); node adds `node_range(node_id)` (per-node scan via `Prefix::new(NS, &node_id.key())`, backs the per-node query + unbond cleanup). The whole-directory `AllEntries` pull (node-then-curated, sound because the digest is an order-independent multiset) is composed by the query handler over the two `range` iterators - NOT a storage method. `retrieval_limits` consts (DEFAULT/MAX = 50/100 per class) added for the query layer. 7 unit tests (node/curated round-trip, overwrite, per-node isolation, node range ordering+composite bounds+descending, curated range ordering+bound, cross-namespace non-collision)
- [x] 3.2 `NodeEntry { data, updated_at_height, sequence, signature }` and `CuratedEntry { data }` compact `to_bytes`/`try_from_bytes` value codecs in `nym-directory-contract-common` (`types.rs`): node = `updated_at_height || sequence || lp(signature) || data` (fixed fields first, signature length-prefixed, `data` unframed tail); curated = raw `data`. The class is known from which store an entry lives in, so the value carries no discriminant (`DirectoryEntry::try_from_bytes(namespace, _)` dispatch retained for convenience). Reader is `helpers::ValueReader` (errors as `MalformedEntryValue`). 5 unit tests (node/curated round-trip, empty fields, namespace dispatch, truncation rejected). The node's `data` payload format stays a consumer concern, e.g. prost+BTreeMap
- [x] 3.3 Persistent per-node `sequence` map (`u64`), surviving entry deletion - declared in `storage.rs` (`sequences` + `current_sequence`/`increment_account_sequence`)
- [x] 3.4 Allowed-labels map (`label -> LabelConfig { max_size }`) + 128 KiB ceiling constant (`MAX_LABEL_SIZE_CEILING`) - declared in `storage.rs` + `constants.rs`
- [ ] 3.5 Store the full `LtHash16` state as the accumulator `Item` (~2 KB, mutated O(1) per write, stored RAW not base64; `digest_state` Item declared). NOTE: the declared `Item<[u8; nym_lthash::DIGEST_LEN]>` type cannot actually be `save`d/`load`ed - `serde` only derives (de)serialization for arrays up to length 32, so a 2048-byte array fails the `Serialize`/`DeserializeOwned` bound. This must change to raw `Storage::set/get` (consistent with the entry store) or a `Binary`/`Vec<u8>`-backed item before the digest can be persisted. Expose & ICS23-prove the compact 32-byte `LtHash16::out()` (blake3 collapse) as the public digest (comparison-only, not homomorphic). The per-variant leaf encoder is `EntryKey::digest_leaf(&DirectoryEntry)` in `nym-directory-contract-common` (node commits data+signature+sequence; curated commits data; `updated_at_height` excluded) - already implemented - so contract and client agree byte-for-byte
- [x] 3.6 Instantiate: admin = the instantiator (`Admin::set(info.sender)`, not an `InstantiateMsg` field); params = mixnet contract address + initial label set; seeds the whitelist from `KnownLabel::ALL` (each `default_config`) then applies `initial_labels` - implemented in `storage.rs::initialise`

## 4. Digest

- [ ] 4.1 Wire the in-house LtHash16 with O(1) insert/remove helpers
- [ ] 4.2 Apply the digest delta on every write and delete (subtract old leaf, add new leaf)
- [ ] 4.3 Determinism + correctness tests: stable encoding, recompute-equals-stored, cross-namespace non-collision

## 5. Node write / auth path

- [ ] 5.1 Cross-query mixnet (`MixnetContractQuerier`) for existence (bonded and not unbonding) and the base58 identity key; decode to 32 bytes
- [ ] 5.2 Verify the ed25519 signature over `node_id || label || sequence || data` via `deps.api.ed25519_verify`
- [ ] 5.3 Enforce gap-free exact-match per-node sequence (signed sequence must equal the expected next; advance only on success); reject stale/replayed/jumped-ahead/cross-slot-lifted signatures (including replay-after-delete)
- [ ] 5.4 Validate label allowed + `data` within `max_size`; store entry (+ signature + `updated_at`); update digest
- [ ] 5.5 Node self-delete handler (signed, sequence-advancing); update digest

## 6. Admin path

- [ ] 6.1 `AddLabel` / `SetLabel` / `RemoveLabel` (admin-only; ceiling enforced; removal non-destructive)
- [ ] 6.2 `SetCuratedEntry { key, data }` / `RemoveCuratedEntry { key }` (admin-only; `key` is a single admin-chosen path string, no label/suffix structure); update digest

## 7. Unbond callback (cross-contract)

- [ ] 7.1 Directory `OnNymNodeUnbond { node_id }` handler gated by `UnauthorisedMixnetCallback` (sender must be the configured mixnet contract)
- [ ] 7.2 Delete the node's entries via bounded prefix iteration + digest deltas; make it idempotent
- [ ] 7.3 Mixnet contract: add `directory_contract_address` to `State` + `InstantiateMsg` + a queued migration
- [ ] 7.4 Mixnet contract: emit a best-effort (reply-on-error, non-fatal) `OnNymNodeUnbond` sub-message in the unbond handler

## 8. Queries

- [ ] 8.1 `Admin`, `NodeEntry`, `CuratedEntry { key }`, `NodeEntries`, paginated `AllCuratedEntries { start_after: String }`, paginated `AllEntries` (both classes, node-then-curated via `storage::all_entries`), `Sequence`, `Digest`, `AllowedLabels` (no `config` query)
- [ ] 8.2 Confirm provable reads: the digest `Item` and individual entries at deterministic raw keys (document the cw-storage-plus `Path` key layout - `length_prefixed(namespace) ++ length_prefix-all-but-last(key)` - for clients)

## 9. Tests

- [ ] 9.1 Write/auth: valid write, invalid signature, stale/replayed sequence, replay-after-delete, unknown/unbonding node, disallowed label, oversized data, any-relayer
- [ ] 9.2 Admin: label add/ceiling/non-admin/non-destructive-remove; curated set/remove/non-admin
- [ ] 9.3 Digest: update on write/delete, identical-data no-op, recompute-equals-stored, cross-namespace non-collision
- [ ] 9.4 App-level test for the mixnet unbond -> directory callback (deps-level handler tests do not dispatch sub-messages)
- [ ] 9.5 `cosmwasm-check` the optimized artifact

## 10. Schema + client wiring

- [ ] 10.1 Generate the JSON schema (schema-gen bin)
- [ ] 10.2 validator-client: directory query/signing traits (mirroring the dkg/node-families patterns)
