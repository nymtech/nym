## 1. Payload model (new `nym-directory-types` leaf crate + prost encoding)

- [x] 1.1 Create the standalone leaf crate `nym-directory-types` (deps `prost` + `thiserror`; register in the
  workspace) - no dependency on `nym-directory-attestation` or the wasm contract crate.
- [x] 1.2 In `nym-directory-types`, add the `SphinxKeys` payload as a hand-written `#[derive(prost::Message)]` struct (
  `BTreeMap` for any map field; concrete fields per design Open Questions) with canonical `to_canonical_bytes` /
  `from_canonical_bytes`, plus round-trip, determinism, and forward-compat (added-field-ignored-by-old-reader) unit
  tests. Do not touch `nym-directory-attestation` or its `DirectorySubset` trait.
- [x] 1.3 In nym-node, add the closed `DirectoryPayload` enum (one variant per `KnownLabel`, only `SphinxKeys`
  populated) with `label() -> KnownLabel` and `to_canonical_bytes() -> Vec<u8>` by matching on the variant.
- [x] 1.4 Add an `ALL`-driven test that every `DirectoryPayload` variant maps to a distinct `KnownLabel`, so backfilling
  a payload cannot silently mismatch its label.

## 2. Configuration and opt-in gate

- [x] 2.1 Add a `[directory]` config section to nym-node with an `enabled` flag (default false).
- [x] 2.2 Add hidden (`clap(hide = true)`) CLI/env-overridable tuning knobs: sphinx emit debounce, write retry count,
  and dormant/whitelist-refresh back-off interval, with sensible defaults.
- [x] 2.3 Resolve the directory contract address from network details (`Option<String>`); implement the activation
  predicate `enabled && contract_address.is_some()` and its inactive-path logging.

## 3. Publisher write path (single serialized writer)

- [x] 3.1 Create the `nym-node/src/node/directory_publisher/` module and the `DirectoryPublisher` struct holding the
  signing+query client, `node_id`, ed25519 identity key, sequence tracker, and reconcile cache.
- [x] 3.2 Implement the reconcile cache: seed `label -> on-chain bytes` from a single `get_node_entries(node_id)` per
  sweep; update on write/delete success.
- [x] 3.3 Implement `reconcile_and_write(payload)`: diff canonical bytes vs cache; on absent-or-different, sign
  `node_signing_payload(node_id, label, sequence, data)` with the identity key and relay via `set_node_entry`; no-op
  when equal.
- [x] 3.4 Implement `delete_entry(label)`: sign `node_signing_payload(node_id, label, sequence, &[])` and relay via
  `delete_node_entry`; update cache on success.
- [x] 3.5 Implement sequence handling: read expected next sequence via `get_sequence` at startup; on a sequence-mismatch
  rejection, re-read and retry (bounded by the configured retry count). Shared by set and delete.

## 3b. Reconcile sweep and deletion

- [ ] 3b.1 Implement `desired_snapshot()`: gather the current payload every producer would publish (only `SphinxKeys`,
  derived from `ActiveSphinxKeys`, for now).
- [ ] 3b.2 Implement the sweep: refresh the cache + whitelist, `reconcile_and_write` every desired payload, then
  `delete_entry` every published entry whose label is a `KnownLabel` absent from the desired snapshot; never touch
  unknown-label entries.
- [ ] 3b.3 Drive the sweep from a long-interval timer, at startup (first sweep = the startup snapshot), and on recovery
  from dormant.

## 4. Startup preflight and dormant back-off

- [x] 4.1 Implement `node_id` resolution + bonded check via the mixnet contract (lookup this node's active,
  non-unbonding bond by identity); return the `node_id` on success.
- [x] 4.2 Implement the fundability check by querying the chain directly: the relayer account's on-chain
  balance against a minimum threshold, falling back to an active feegrant allowance; treat insufficient
  balance with no feegrant as not-yet-fundable.
- [ ] 4.3 Implement the dormant state machine: on any preflight failure log a clear actionable error (name the fix), go
  dormant, and re-run preflight on the back-off interval; on a later pass resume by triggering an immediate reconcile
  sweep (not by draining the channel); log only on state transitions (no per-recheck spam).

## 5. Label-whitelist reconciliation (version skew)

- [ ] 5.1 Fetch `get_allowed_labels()` at startup into a cached whitelist set; refresh it on the back-off/reconcile
  cadence.
- [ ] 5.2 Guard every write: skip (with a warning naming the label) any payload whose label is not in the current
  whitelist.
- [ ] 5.3 On whitelist refresh, warn for any contract label that does not parse to a `KnownLabel` (node binary may be
  behind); warn-once per unchanged state.

## 6. Event model and producers

- [ ] 6.1 Define `DirectoryUpdate` wakeups and the mpsc channel; implement the publisher's single-consumer loop that
  `select!`s over the sweep timer, the dormant re-check, and the channel, dispatching each wakeup through a targeted
  `reconcile_and_write` (gated by preflight + whitelist), so all writes/deletes are serialized.
- [ ] 6.2 Add a `DirectoryUpdate` `Sender` to `KeyRotationController` and emit the current `SphinxKeys` payload after
  each key mutation (pre-announce / swap / purge); make the emit best-effort so a full/absent channel never disrupts
  rotation. (Startup publication of sphinx is covered by the first sweep, task 3b.3, not this emit.)

## 7. Startup wiring and isolation

- [x] 7.0 Construct the publisher's signing nyxd client (`DirectSigningHttpRpcNyxdClient`) from the node's chain
  mnemonic. Move ownership of the mnemonic + chain client into nym-node proper (currently derived ad-hoc from
  `entry_gateway.mnemonic`, e.g. `node_chain_address()`) so the publisher and `node_chain_address()` share one owner
  instead of re-deriving the wallet.
- [ ] 7.1 In `start_nym_node_tasks`, when the activation predicate holds, build the publisher (identity key, relayer
  client, nym-api + mixnet clients, rotation sender wired to the controller) and spawn it fire-and-forget via
  `shutdown_tracker().try_spawn_named`.
- [ ] 7.2 Verify (by construction + test) that no publisher error path propagates to node startup or the mixnet
  datapath.

## 8. Tests

- [ ] 8.1 Unit-test `reconcile_and_write`: no-op on equal bytes, write on absent/different, cache updated on success.
- [ ] 8.2 Unit-test sequence handling: initial read, mismatch triggers re-read + bounded retry.
- [ ] 8.3 Unit-test preflight outcomes (bonded/unbonded, fundable/not, annotation absent) and the dormant->recovery
  transition with single-logging.
- [ ] 8.4 Unit-test whitelist reconciliation: skip+warn for unwhitelisted label, warn for unknown contract label, resume
  after a refresh re-adds a label.
- [ ] 8.5 Unit-test the sweep's deletion: an orphaned known-label entry (not in the desired snapshot) is deleted; an
  unknown-label entry is never deleted; a desired-but-absent entry is created.
- [ ] 8.6 Test that concurrent/bursty wakeups are serialized through the single writer with correct sequences.
- [ ] 8.7 Test the `KeyRotationController` emit fires on key change and does not affect rotation when the channel is
  unavailable.

## 9. Validation

- [ ] 9.1 `cargo build` + `cargo test` for the touched crates (`nym-directory-types`, nym-node) green.
- [ ] 9.2 `openspec validate node-directory-publishing --strict` passes.
