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
- [x] 2.2 Add hidden (`clap(hide = true)`) CLI/env-overridable tuning knobs: write retry count and
  dormant/whitelist-refresh back-off interval, with sensible defaults.
- [x] 2.3 Resolve the directory contract address from network details (`Option<String>`); gate the publisher on the
  `enabled` flag, and treat `enabled` with no configured contract address as a hard startup error (fail fast on
  misconfiguration) rather than a silent no-op.

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

- [x] 3b.1 Implement `desired_snapshot()`: gather the current payload every producer would publish (only `SphinxKeys`,
  derived from `ActiveSphinxKeys`, for now).
- [x] 3b.2 Implement the sweep: refresh the cache + whitelist, `reconcile_and_write` every desired payload, then
  `delete_entry` every published entry whose label is a `KnownLabel` absent from the desired snapshot; never touch
  unknown-label entries.
- [x] 3b.3 Drive the sweep from a long-interval timer, at startup (first sweep = the startup snapshot), and on recovery
  from dormant.

## 4. Startup preflight and dormant back-off

- [x] 4.1 Implement `node_id` resolution + bonded check via the mixnet contract (lookup this node's active,
  non-unbonding bond by identity); return the `node_id` on success.
- [x] 4.2 Implement the fundability check by querying the chain directly: the relayer account's on-chain
  balance against a minimum threshold, falling back to an active feegrant allowance; treat insufficient
  balance with no feegrant as not-yet-fundable.
- [x] 4.3 Implement the dormant state machine: on any preflight failure log a clear actionable error (name the fix), go
  dormant, and re-run preflight on the back-off interval; on a later pass resume by triggering an immediate reconcile
  sweep (not by draining the channel); log only on state transitions (no per-recheck spam).

## 5. Label-whitelist reconciliation (version skew)

- [x] 5.1 Fetch `get_allowed_labels()` at startup into a cached whitelist set; refresh it on the back-off/reconcile
  cadence.
- [x] 5.2 Guard every write: skip (with a warning naming the label) any payload whose label is not in the current
  whitelist.
- [x] 5.3 On whitelist refresh, warn for any contract label that does not parse to a `KnownLabel` (node binary may be
  behind); warn-once per unchanged state.

## 6. Event model and producers

- [x] 6.1 Define `DirectoryUpdate` wakeups and the mpsc channel; implement the publisher's single-consumer loop that
  `select!`s over the sweep timer, the dormant re-check, and the channel, dispatching each wakeup through a targeted
  `reconcile_and_write` (gated by preflight + whitelist), so all writes/deletes are serialized.
- [x] 6.2 Add a directory-publisher event `Sender` to `KeyRotationController` and emit the current `SphinxKeys` payload
  after a pre-announce only; make the emit best-effort so a full/absent channel never disrupts rotation. Swap and purge
  need no emit: swap does not change the published key *set* (only which key is primary), and a purged key belongs to a
  previous rotation a correct client never selects - the periodic sweep reconciles both. Both the emit and the sweep's
  `desired_snapshot` derive the payload from one shared `ActiveSphinxKeys` helper, so they are byte-identical and cannot
  clobber each other. (Startup publication of sphinx is covered by the first sweep, task 3b.3, not this emit.)

## 7. Startup wiring and isolation

- [x] 7.0 Construct the publisher's signing nyxd client (`DirectSigningHttpRpcNyxdClient`) from the node's chain
  mnemonic. Move ownership of the mnemonic + chain client into nym-node proper (currently derived ad-hoc from
  `entry_gateway.mnemonic`, e.g. `node_chain_address()`) so the publisher and `node_chain_address()` share one owner
  instead of re-deriving the wallet.
- [x] 7.1 In `start_nym_node_tasks`, when the activation predicate holds, build the publisher (identity key, relayer
  client via the shared `NyxClient`, node details + shared `ActiveSphinxKeys` handle, rotation sender wired to the
  controller) and spawn it fire-and-forget via `shutdown_tracker().try_spawn_named`.
- [x] 7.2 Verify (by construction) that no publisher *runtime* error path propagates to node startup or the mixnet
  datapath: `run()` returns `()`, is spawned fire-and-forget, and routes every query/write/outage failure to its own
  dormant/back-off + return-to-preflight handling. The only startup-blocking path is the deliberate fail-fast on the
  `enabled`-without-contract-address misconfiguration (see task 2.3).

## 8. Tests

- [x] 8.1 Unit-test `reconcile_and_write`: no-op on equal bytes, write on absent/different, cache updated on success.
- [x] 8.2 Unit-test sequence handling: a mismatch triggers a re-read + retry to success; a persistent mismatch surfaces
  an error after the bounded retry budget is exhausted.
- [x] 8.3 Unit-test preflight outcomes (bonded/unbonded, fundable via balance, fundable via feegrant, not-fundable) and
  the dormant->recovery transition.
- [x] 8.4 Unit-test whitelist reconciliation: skip a non-whitelisted label, warn (once) for an unknown contract label,
  resume writing after a refresh re-adds the label.
- [x] 8.5 Unit-test the sweep: a desired-but-absent entry is created and an unknown-label entry is never touched; the
  deletion primitive is tested directly (the sweep's orphan-deletion branch is currently unreachable because
  `desired_snapshot` emits every known label - see note below).
- [x] 8.6 Test that successive writes through the single writer use gap-free, increasing sequences.
- [ ] 8.7 Test the `KeyRotationController` emit fires on key change and does not affect rotation when the channel is
  unavailable. (Not unit-tested: the controller is impractical to construct in isolation - it needs a full `Config`, a
  disk-backed `SphinxKeyManager`, and a `RotationConfig`. The emit is verified structurally instead: it is called only
  from the `PreAnnounce` arm, is a best-effort `try_send` that cannot disrupt rotation, and its payload content is
  covered by the `directory_sphinx_keys()` publisher tests.)

## 9. Validation

- [x] 9.1 `cargo build` + `cargo test` for the touched crates (`nym-directory-types`, nym-node) green.
- [x] 9.2 `openspec validate node-directory-publishing --strict` passes.
