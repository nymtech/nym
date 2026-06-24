## Context

nym-nodes publish their configuration (identity and sphinx public keys, IP addresses, advertised roles, and similar metadata) only over a per-node HTTP API. Consumers (nym-api, the node-status API, potentially clients) must query each node individually to build a network view. This is slow, fragile, and unverifiable. This change introduces a CosmWasm `directory-contract` so nodes (and a small curated set of entities such as nym-apis) push their data to the chain once and any consumer reads a single, chain-anchored, verifiable source.

The contract is a sibling to the node-families and ecash contracts and follows their crate layout. It reuses the mixnet contract read-only for node identity and ownership via `MixnetContractQuerier` (the node-families pattern). Hard constraints: the contracts workspace is pinned to rustc 1.86 and `cosmwasm-std` is `=2.2.2` (so `Api::ed25519_verify` is available).

This design covers the CONTRACT and the small mixnet-side callback wiring. The retrieval/verification client (light-client bootstrap, root key, the paranoid and normal verification routes, cross-check) is a SEPARATE future change; it is summarized here only to justify what the contract must commit on-chain.

## Goals / Non-Goals

**Goals:**
- A schema-agnostic, push-based key-value directory over opaque deterministic bytes, under two key-classes: node entries and admin-curated entries.
- Writes authenticated by the node's ed25519 identity key with per-node replay protection; curated entries managed by the admin.
- A single O(1)-updatable on-chain integrity digest so a consumer can verify the complete, untampered directory at a height with one chain proof.
- Bounded, self-cleaning lifecycle (per-label sizes + a best-effort unbond callback) with no unbounded iteration in any handler.

**Non-Goals:**
- The retrieval client, light client, root-key/DNS bootstrap, and the paranoid/normal verification routes (separate change).
- Liveness/reachability tracking (owned by the performance/status system).
- A merkle tree or a per-node digest (explicitly out for v1).
- Defining the payload schema of the stored bytes (a consumer concern).

## Decisions

### D1. Unbond cleanup via a best-effort mixnet callback

When a node unbonds, the mixnet contract sends the directory an `OnNymNodeUnbond { node_id }` sub-message (mirroring the existing node-families wiring: a stored `directory_contract_address`, set via a queued migration, plus a `wasm_execute` in the unbond handler). The directory handler is gated by an `UnauthorisedMixnetCallback` check and deletes the node's entries by prefix-iterating `entries.prefix(node_id)` (bounded - see D4) and applying the digest delta. The sub-message is **best-effort (reply-on-error, non-fatal)** so a directory fault cannot block node unbonding. Alternative considered: a permissionless `PruneUnbonded` (no mixnet change, lazy). Rejected in favor of eager atomic cleanup; the best-effort wrapper neutralizes the main downside (blast radius on the mixnet unbond path). Trade-off: with no prune backstop, a callback failure leaves orphans, acceptable because they are harmless to consumers (filtered by the bonded set) and the bounded callback should reliably succeed.

### D2. Storage model, stored signature, and per-node sequence

State holds two explicitly-namespaced key-classes, both folded into the global digest. The namespace is an extensible discriminant carried in both the storage key (distinct cw-storage-plus map namespaces, so raw keys never collide) and the digest leaf (so cross-class entries cannot produce equal leaves), and it doubles as the trust-tier marker:
- node entries (`node` namespace): `(node_id, label) -> { data, updated_at, signature }`;
- curated entries (`curated` namespace): `(curated_id, label) -> data` (admin-managed; see D-curated).

Each node entry stores its ed25519 `signature` (~64 bytes) so the entry is self-authenticating offline (a consumer with the node's identity key can verify authorship with no chain interaction). Replay protection is a **per-node monotonic sequence** (one `u64` per `node_id`), not per-slot: the node signs `node_id || label || sequence || data`, and the contract requires the sequence to strictly increase. Per-node (rather than per-`(node,label)`) bounds the sequence state to O(1) per node so unbond cleanup is fully bounded, at the cost of globally ordering a node's writes (acceptable: writes are rare). This prevents replay including replay-after-delete.

### D3. Admin-mutable label whitelist (non-destructive removal)

Allowed labels live in contract state as `Map<label, LabelConfig { max_size }>`, evolved by admin-only `AddLabel`/`SetLabel`/`RemoveLabel` (no code migration). Writes validate `label` is allowed. `RemoveLabel` is **non-destructive**: it blocks new writes/updates under that label but leaves existing entries readable and in the digest (cascade-delete would be unbounded iteration). The contract stays category-governed but schema-agnostic.

### D4. Caps: per-label size, no per-node count cap, hard ceiling

No per-node entry-count cap - a node has at most one entry per allowed label, so its footprint is governed by the (admin-controlled) label set. Each write validates `data.len() <= allowed_labels[label].max_size`. A contract-level hard ceiling of **128 KiB** bounds any `max_size` (fat-finger rail atop the chain tx-size limit). Because the label set is governed and small, prefix-iteration on unbond is bounded.

### D5. Writes require bonded-and-not-unbonding

Writes and self-deletes require `check_node_existence == true` (bonded AND not unbonding). An unbonding node is rejected, so it cannot re-populate entries the unbond callback just cleared.

### D6. Anyone may relay

The ed25519 identity-key signature plus the per-node sequence is the authority; the tx sender is unchecked. A node need not fund its blockchain mnemonic, and relayers can submit on its behalf. Spam is self-funded and rejected (bad signature or stale sequence).

### D7/D8. Single global LtHash digest over both key-classes

The contract maintains one global incremental multiset digest (LtHash16, ~2 KB `Item`) over all entries, `leaf = canonical(namespace, id, label, value)` (length-prefixed, namespace included so cross-class entries cannot collide) across both node and curated entries. On every write: `digest -= LtHash(old_leaf); digest += LtHash(new_leaf)` (O(1), reads only the entry being changed). It lets a consumer verify the complete, untampered set at a height with one ICS23 proof of the digest plus a local recompute. Feasibility confirmed: a cosmwasm contract computing an LtHash with `blake3` (via the `digest 0.10` `ExtendableOutput` trait) passed `cosmwasm-check` (pure integer math, no float opcodes). The implementation is the in-house `nym-lthash` crate at `common/lthash` (see Risks). Why a digest and not an on-chain merkle tree: the chain already merkelizes state into the app_hash (per-key IAVL proofs are free); a digest adds the one missing thing - a compact whole-set commitment - at O(1) per write versus O(log N) gas + O(N) state for a tree. A secure multiset hash (LtHash / MSet-Add) is required; naive XOR/sum is forgeable.

### D-curated. Curated entries (admin-managed) + DKG as a client-side source

Curated entries are a second key-class, same `(id, label) -> bytes` shape, written only by the admin (`SetCuratedEntry`/`RemoveCuratedEntry`); the admin assigns the curated id; no per-label whitelist applies (the admin is the gate). They are folded into the same global digest. Their purpose is to make off-chain aggregators (nym-apis) authenticatable on-chain. Complementarily and with zero contract coupling, the verification client may also source the core nym-api set from the coconut-DKG contract (`DealerDetails` carries `ed25519_identity` + `announce_address` for current-epoch dealers, ICS23-provable against the same app_hash). Client Tier-1 set = union(DKG dealers, curated entries).

### D-serde. prost derive + BTreeMap; hash the stored bytes

Entry payloads use `prost` with `#[derive(Message)]` on hand-written structs (no `.proto`, no `protoc`). All map fields MUST be `BTreeMap` (never `HashMap`) for deterministic bytes. The "protobuf is not canonical" warning does not bite because the digest hashes the STORED bytes the node produced (nobody re-serializes). The tag-based, unknown-field-tolerant format lets old and new consumers read the same blob. Alternatives: borsh (deterministic but positional, no forward-compat) and bilrost (canonical-by-construction). The contract treats values as opaque Binary regardless.

### D9. Query surface

Smart queries (no proofs): `entry(id, label)`, `entries_for(id)` (bounded prefix scan), `all_entries(start_after, limit)` (paginated enumeration for the whole-directory pull), `digest()`, `sequence(node_id)`, `allowed_labels()`, `config()`. Proofs come from RAW store reads (smart queries produce none): the `digest` Item and individual entries at their deterministic keys, ICS23-verified against the app_hash. No merkle queries; no per-node digest.

## Risks / Trade-offs

- [LtHash implementation] Use the in-house `nym-lthash` crate at `common/lthash`: generic over `digest::ExtendableOutput` (digest 0.10 - the workspace pins blake3 `<1.8.4` to stay on digest 0.10), `blake3` production XOF, `sha3 0.10` dev-only differential-test XOF, concrete `LtHash16 = LtHash<blake3::Hasher>`. Chosen over the unmaintained `lthash-rs` and the un-scopable `commonware-cryptography`. blake3 is declared directly (not `workspace = true`) to set `default-features = false` for no_std/wasm. Built + tested, and a contract using it validated to pass `cosmwasm-check`; `rust-version` pinned 1.86 (contracts MSRV).
- [Replay correctness] A flaw in the per-node sequence/binding reopens replay. -> Sign `node_id||label||sequence||data`, require strict increase, cover with adversarial tests (replay, cross-slot lift, stale sequence, replay-after-delete).
- [Best-effort callback leaves orphans] A failing/over-large callback leaves entries for an unbonded node, with no prune backstop. -> Bounded by the governed label set; orphans are harmless (consumers filter by the bonded set); revisit a permissionless prune if churn grows.
- [Identity-key encoding] Base58 in the bond vs raw 32 bytes for `ed25519_verify`. -> Decode and length-check; typed error on malformed keys.
- [prost determinism discipline] A future `HashMap` map field breaks determinism. -> Enforce/document the BTreeMap rule and add a serialization determinism test.
- [State retention / pruning] ICS23 proofs at height H need a node that retained H. -> A client/ops concern (separate change); flagged.
- [Cross-contract coupling] Depends on the mixnet address + query surface. -> Store the address at instantiate; pin via `MixnetContractQuerier`.

## Migration Plan

1. Deploy `directory-contract` with the mixnet address in instantiate config; run the mixnet queued migration to set `directory_contract_address`. Additive; nothing depends on it yet.
2. nym-node gains a push client (separate change) and publishes on change.
3. Consumers gain a read path (separate change), computing `missing = bonded(mixnet) - published` and falling back to the HTTP pull for missing nodes during migration.
4. The verification client (separate change) implements the paranoid/normal routes over the digest, with aggregator auth via curated entries + the DKG dealer set.

Rollback: additive and fallback-guarded - disabling the push client and read path reverts to current behavior with no data migration. The mixnet sub-message is best-effort, so it can be left in place harmlessly.

## Open Questions

- Final LtHash implementation (crate vs vendored) confirmed building under rustc 1.86.
- The curated-id scheme (string handle vs counter) and the initial label set + per-label sizes at instantiate.
- Exact mixnet query returning a node's identity key for a `NodeId`, and the DKG dealer query path used by the (separate) client.
- Instantiate parameters (admin, mixnet address, initial labels, ceiling) and whether the admin is a governance multisig.
