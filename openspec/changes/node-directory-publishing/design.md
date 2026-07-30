## Context

The verifiable directory is built except for its input side. The directory contract accepts `SetNodeEntry { node_id, label, data, sequence, signature }` authorised by an ed25519 signature from the node's identity key over `node_signing_payload(node_id, label, sequence, data)`, with a gap-free per-node sequence and an admin-governed label whitelist (`KnownLabel::SphinxKeys` -> `"sphinx_key"` is auto-whitelisted). The write plumbing exists in `validator-client` (`DirectorySigningClient::set_node_entry`, `DirectoryQueryClient::{get_node_entries, get_node_entry, get_sequence}`) but has **zero callers**. nym-node already holds both keys a publish needs: the ed25519 identity keypair (the signing authority) and a secp256k1 chain account (`node_chain_address`, the tx relayer). The read/producer side (attestation provider, anchors, retrieval client) is complete and consumes whatever nodes write.

This change is the nym-node caller. It is plumbing-first by explicit decision: the concrete set of payloads is deferred and backfilled later on this branch; `sphinx_key` is wired through as the one real example.

## Goals / Non-Goals

**Goals:**
- A nym-node subsystem that serialises, signs, and submits the node's entries to the directory contract.
- General, reusable plumbing: an event-driven publisher, a closed payload model tied to the contract's label whitelist, reconcile-before-write, a single serialized writer, and clean seams for adding runtime-updatable payloads later.
- Robust operator experience: startup preflight with clear errors, automatic recovery after a later bond/top-up, and absolute failure isolation from node operation.
- `sphinx_key` reconciled on startup and kept current across rotations - written only when it has actually changed since the last publish (like every payload; reconcile-before-write).

**Non-Goals:**
- Defining the final payload set (`NodeDescription`, wireguard, service-providers, keys, ...) *as part of this plumbing change* - only `SphinxKeys` is populated here. The final set is still expected to be defined and backfilled before the branch's PR (owned by the branch author), using this plumbing without structural change. It is out of scope for this spec, not out of scope for the branch.
- Any change to the directory contract's on-chain behaviour or requirements (it already treats `data` as opaque and defines `KnownLabel::SphinxKeys`).
- The `sphinx_key` payload's final wire format and the contract's placeholder `max_size` for it - both are payload-backfill concerns.
- Curated entries, admin writes, delete-on-unbond from the node side (the contract handles unbond cleanup via the mixnet callback).
- Any change to how the retrieval client or attestation producer read data.

## Decisions

### D1: A single `DirectoryPublisher` task = reconcile loop + event wakeups, sole writer

One background task owns all contract writes for the node. It follows the standard controller shape: a periodic **reconcile sweep** (the correctness backbone) plus **event-driven wakeups** (low-latency triggers) on top, both funnelled through the same single consumer loop.

- **Why single writer**: The contract enforces a gap-free per-node sequence. Because the node is the sole legitimate writer for its `node_id`, the only possible sequence race is self-induced. Funnelling every write (set *and* delete) through one task makes serialization structural - no mutex, no lock discipline - and gives one place that owns the sequence tracker, signing, retry, and the reconcile cache.
- **Why reconcile-loop + events (not one or the other)**: The event path alone can drop or miss updates (bounded channel; events emitted while the publisher is dormant); a slow full reconcile sweep is the eventual-consistency safety net that also performs deletion (D10). Events give promptness so a rotation is not held until the next slow sweep. This reconciles the earlier "don't tightly poll static data" preference: the sweep interval is long, and events - not polling - drive prompt updates.
- **Alternatives considered**: Events only - no deletion path, and dropped/dormant-window events are never recovered. Tight unified poll only - re-queries static data far more often than needed and gives no low-latency rotation update.

### D2: The reconcile sweep operates over a *desired snapshot*; producers wake it

The core operation is a **sweep**: compute the node's *desired snapshot* (the current payload every producer would publish), fetch the node's on-chain entries (`get_node_entries`) and the whitelist (`get_allowed_labels`), then set/replace stale-or-missing desired payloads, delete no-longer-desired known-label entries (D10), and warn on unknown-label entries. The sweep runs at startup (the first sweep = the startup snapshot, covering **every** derivable payload **including sphinx**), on a long timer, and on recovery from dormant.

On top of the sweep, producers hold `Sender` handles and emit `DirectoryUpdate` wakeups for a targeted reconcile of one payload between sweeps:
- **`KeyRotationController`** emits the current `SphinxKeys` payload right after it mutates keys (pre-announce / swap / purge).
- **Future runtime sources** add one more producer + sender - no change to the publisher core.

- **Why the snapshot covers sphinx at startup**: `KeyRotationController` emits only on key *change*; a normal restart where the node already holds a valid primary key performs no mutation and would emit nothing, so relying on rotation events alone would leave the sphinx entry unpublished until the next rotation. The startup snapshot derives sphinx from the current `ActiveSphinxKeys` directly, and rotation events handle only subsequent deltas (deduped by reconcile-before-write).
- **Alternatives considered**: A poll-based sphinx reconcile reading `ActiveSphinxKeys` with no controller change - doesn't generalise to other runtime updates; the channel is the plumbing the future-updatable-fields goal wants anyway. The cost is a handful of best-effort emit calls in `KeyRotationController`.

### D3: `DirectoryPayload` enum keyed to `KnownLabel` (no trait)

Publishable data is a closed enum, one variant per `KnownLabel`, with `label()` and `to_canonical_bytes()` implemented by matching on the variant.

- **Why**: The set of publishable categories *is* the contract's label whitelist - a closed set. A closed enum models it more precisely than an open trait: it gives compiler-exhaustiveness (every category must be handled), needs no dynamic dispatch or `Box<dyn>`, and makes the label<->payload correspondence a property of the type. A node cannot publish an un-whitelisted label anyway, so open extensibility buys nothing.
- **Alternatives considered**: A `NodeDirectoryPayload` trait with `Box<dyn>` collections - more ceremony, loses exhaustiveness, and models an openness the contract does not permit.

### D4: Payload encoding is prost, and payload types live in a new leaf crate `nym-directory-types`

Payloads are encoded with **prost (protobuf)** via hand-written `#[derive(prost::Message)]` structs (no `.proto`/`protoc`), using `BTreeMap` (never `HashMap`) for any map field. The payload types and their canonical `to_canonical_bytes`/`from_canonical_bytes` live in a new standalone leaf crate **`nym-directory-types`** (deps ≈ `prost` + `thiserror`); the aggregating `DirectoryPayload` enum is node-side; `KnownLabel` stays in `nym-directory-contract-common`.

- **Why prost**: Forward compatibility. A future field added under a new protobuf tag is skipped by older readers and read by new ones - so payloads can grow without breaking legacy readers (far safer than the bincode add/reorder footgun). This is the same encoding the archived directory-contract design settled on. The "protobuf is not canonical" concern does not bite because nothing re-encodes on the verify side: the contract stores and hashes the exact bytes the node wrote, and `BTreeMap` makes the node's own encoding deterministic. `prost 0.13` is already a workspace dependency.
- **Why a new leaf crate, not `nym-directory-attestation` or `nym-directory-contract-common`**: a consumer that only wants to *decode a node entry's `data`* (nym-node as producer, an SDK, a topology provider, a thin reader) should not drag in the attestation crate's weight (`cosmrs`, `blake3`, `nym-lthash`, the quorum/digest/signing machinery) nor be forced under the contract-common crate's rustc-1.86/wasm constraint (the contract treats `data` as opaque and never decodes payloads). A tiny leaf crate gives one encoding truth (node encodes, readers decode the same bytes) with a minimal footprint.
- **`DirectorySubset` is NOT moved and payloads do NOT impl it here**: a payload's essential need is only its own canonical codec, not the quorum-attested-pull contract (`DirectorySubset` is consumed by `quorum_subset_digest`/`fetch_and_verify_subset`; a node's on-chain entry is read via ICS23 / digest-recompute). The trait stays in `nym-directory-attestation`, which is unmodified by this change. If a payload ever needs the attested-pull path, the orphan rule lets `impl DirectorySubset for <payload>` live in `nym-directory-attestation` (it owns the trait and would depend on `nym-directory-types`) - no trait move, no thin-crate dependency on the heavy crate.
- **Alternatives considered**: Payloads in `nym-directory-contract-common` - forces prost under 1.86/wasm for no reason. Payloads in `nym-directory-attestation` - saddles thin readers with attestation baggage. Payloads in nym-node with the client re-deriving the encoding - reintroduces encoder/decoder drift. bincode - no tag-based forward compatibility.

### D5: Reconcile-before-write against a startup-seeded cache

Each sweep seeds/refreshes a `label -> on-chain bytes` cache from one `get_node_entries(node_id)`; both sweep reconciles and event-driven targeted reconciles diff the payload's canonical bytes against the cache and write only on absent-or-different, updating the cache on write success.

- **Why**: Avoids the tx (and gas) entirely when nothing changed - important because the startup snapshot re-derives every payload and a rotation may re-emit an unchanged two-key set. It self-heals and prevents a fresh instance from re-publishing what a prior instance already wrote. The contract's own no-op-on-identical-bytes is a fallback, but client-side diffing skips the tx outright.

### D6: Startup preflight - bonded (mixnet) + fundable (nym-api annotation), both soft-fail

Preflight resolves `node_id` and confirms the bond via the mixnet contract (the publisher needs `node_id` regardless, since every op is keyed by it), then reads `ChainInteractionCapabilities::can_send_transactions()` (`has_sufficient_tokens || is_fee_grant_grantee`) from nym-api's `/v2/nym-nodes/annotation/{node_id}`.

- **Why bonded check**: The contract rejects writes from unbonded/unbonding nodes (`check_node_existence`), so a preflight turns an ugly on-chain rejection into a clear operator-facing log. It is UX, not a security boundary (the contract still enforces it).
- **Why reuse nym-api for fundability**: nym-api already computes per-node fundability including feegrant status; reusing `can_send_transactions()` avoids reimplementing raw chain balance + feegrant-allowance queries in nym-node, and the node already has a `NymApisClient`. The trade-off is slight staleness (nym-api's refresh cadence), which the back-off re-check absorbs.
- **Why soft-fail**: Operators often bond or fund *after* first launch; failing startup or exiting the publisher would be hostile. A failed check logs an actionable error and goes dormant.

### D7: Dormant back-off with automatic recovery

On preflight failure the publisher enters a dormant state and re-runs preflight on a long interval; a later pass resumes publishing by **running an immediate reconcile sweep** (not by draining the channel), then continues normally, without a node restart. It logs on state transitions, not on every re-check.

- **Why sweep-on-recovery**: While dormant, producer wakeups on the bounded channel back up or are dropped by their best-effort send, so a woken publisher cannot trust the channel to still hold the latest state (e.g. a rotation that happened while dormant). Re-deriving the desired snapshot via a sweep makes recovery correct regardless of what the channel dropped - the sweep (D2/D10) is exactly the "re-derive and reconcile everything" operation.
- **Why the rest**: Matches how nodes are operated (bond/fund shortly after start), recovers hands-free, and avoids both write-spam against a node that cannot write and log-spam while dormant.

### D8: Opt-in gate + hidden tuning knobs

A new `[directory]` config section gates the publisher on an `enabled` flag AND a configured contract address. Tuning knobs (retry count, back-off intervals) are CLI/env-overridable but `clap(hide = true)`.

- **Why**: The directory is mid-migration; publishing must be opt-in and inert where the contract is not deployed. Hidden knobs follow the project convention for internal tuning parameters.

### D9: Reconcile against the contract's label whitelist (version skew)

The node's `KnownLabel::ALL` is fixed at compile time; the contract's whitelist is admin-governed at runtime (`get_allowed_labels()`). The publisher fetches the whitelist at startup (alongside the entry cache) and refreshes it periodically, and guards every write: a payload whose label is not in the current whitelist is skipped with a warning rather than written.

- **Why**: A node binary can be ahead of or behind the deployed contract. If the node writes a label the contract has not whitelisted (never added, or `RemoveLabel`-d - removal is non-destructive, so old entries stay but new writes are blocked), the contract rejects the tx; guarding client-side turns that into a clear warning and saves the doomed write. Conversely, when the contract advertises a label that does not parse to a `KnownLabel`, the node logs that its binary may be behind - an upgrade signal - and keeps publishing the labels it does know. Refreshing periodically lets a runtime `AddLabel`/`RemoveLabel` take effect without a node restart.
- **Alternatives considered**: Rely solely on the contract's rejection - correct but noisy (failed txs, unclear operator signal) and wastes gas. Fetch the whitelist only once at startup - simpler but misses runtime governance changes; the periodic refresh is cheap (one query on the existing cadence).

### D10: Deletion of no-longer-desired entries in the reconcile sweep

The reconcile sweep deletes any *published* entry whose label the node recognises (`KnownLabel`) but that is not in the current desired snapshot - cleaning up orphans left by a label removed from the whitelist, or by a payload that is no longer applicable. Entries under labels the node does not recognise are never deleted (warn only). Deletes route through the single writer.

- **Why it is safe/possible**: The contract's `try_delete_node_entry` requires only a bonded identity, the correct sequence, and a valid ed25519 signature over `node_signing_payload(node_id, label, sequence, &[])` - it does **not** require the label to still be whitelisted (verified in the contract). So the node can remove an entry under a since-removed label (`RemoveLabel` is non-destructive and leaves the entry behind); the sweep is the mechanism that actually cleans it up. Deletes need the node bonded, consistent with the preflight gate.
- **Why never delete unknown-label entries**: A newer node instance (or future binary) may have published a label this binary does not know; a downgrade must not destroy data it cannot interpret. Unknown-label entries only produce the "binary may be behind" warning (D9).
- **Alternatives considered**: No deletion (set/replace only) - leaves orphans forever until unbond; rejected because "reconcile to desired state" includes removals. Delete driven by events rather than the sweep - events do not carry "this used to exist and no longer should"; only a full desired-vs-published diff (the sweep) can detect a removal.

## Risks / Trade-offs

- **Touching `KeyRotationController`** → the emit is additive (a `Sender` + calls after existing mutations) and best-effort (a full channel or absent receiver must never disrupt rotation); keep rotation correctness independent of the publisher.
- **nym-api annotation staleness** (a just-funded node reads stale `can_send_transactions() == false`) → the long back-off re-check recovers automatically; the cost is a delay, not a stuck state.
- **Gas cost / griefing** → writes only occur on real changes (D5) and only when funded (D6); sphinx writes are ~once per rotation. No unbounded write loop exists.
- **Deferred payload format** → the `sphinx_key` payload wire format and the contract's placeholder `max_size` are backfill items; the plumbing must not hardcode assumptions that a later format change would break (keep the codec in the shared crate, versionable).
- **Sequence contention with an external relayer** → out of scope; this change relays from the node's own account only. If a gasless relayer is added later, the single-writer invariant still holds as long as it too routes through the publisher.

## Migration Plan

- Ship disabled by default. Networks enable per-operator once the contract is deployed and a contract address is configured in network details.
- Rollout order is unconstrained: the contract, producer, and client already tolerate partial publication (consumers compute `missing = bonded - published` and fall back to the HTTP pull), so nodes can begin publishing incrementally with no coordination.
- Rollback: unset the `enabled` flag (or remove the contract address) - the publisher goes inert; already-published entries remain readable and are cleaned up by the existing unbond callback when a node unbonds.

## Open Questions

- The concrete `sphinx_key` prost payload *fields* (which rotation-tagged x25519 keys, and whether a key carries a proof) and whether the contract's placeholder `max_size` (256) fits the encoded size - deferred to payload backfill. The encoding *mechanism* (prost, `BTreeMap`, in `nym-directory-attestation`) is settled (D4).
- The exact long sweep interval and dormant back-off interval - to be set as hidden tuning defaults during implementation.
- Which additional payloads ship on this branch - to be decided before finalising the branch (the plumbing and encoding are designed to backfill them without structural change).
