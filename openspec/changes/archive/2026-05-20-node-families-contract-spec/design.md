## Context

The node-families CosmWasm contract is the on-chain authority for declaring that two or more Nym nodes belong to the same operator. Route-selection code is expected to consult this data so that the entry and exit gateways of a single NymVPN connection cannot both be in the same family — closing one of the two surveillance windows called out in the proposal (the other being same-subnet detection, which is out of scope here).

The contract is implemented in `contracts/node-families/` with the shared message/type surface in `common/cosmwasm-smart-contracts/node-families-contract/`. The entire feature landed in a single commit (`a21a01cf1a`, "node families (#6715)") on 2026-05-19. No previous CosmWasm contract in the workspace handled families; the design is a fresh build with no migration baseline. The implementation has shipped behind a network-defaults wiring (`NODE_FAMILIES_CONTRACT_ADDRESS` env var, mainnet defaults, sandbox wallet types) — it is live, not aspirational.

This document captures the architectural choices behind the contract as it exists today, so reviewers, integrators (nym-api caches, node-status-api indexers), and future maintainers have a single normative reference. There is no behaviour change being proposed.

## Goals / Non-Goals

**Goals:**
- Capture the trust boundary between the node-families contract and the mixnet contract — which checks each side owns and what each side relies on the other for.
- Pin down the data model: storage maps, secondary indexes, and the rationale for the composite archive keys.
- Pin down the invariants the contract guarantees externally (one family per owner, one family per node, unique normalised family names, monotonic family ids).
- Pin down the cross-contract callback flow that handles node unbonding.
- Document the policy choices around invitation expiry (no background sweeper, expired entries are inert).
- Document the storage-key and event constants as part of the public contract surface, since indexers consume them.

**Non-Goals:**
- Route-selection policy on the client or nym-api side. The contract is a data source; the consumers decide what to do with the data.
- Operator verification ("verified families"). The contract has no notion of vetting beyond ownership of a bonded node.
- Geographic / subnet-distinctness checks. These live entirely outside the contract.
- Sybil resistance. The proposal explicitly accepts that a malicious operator can refuse to declare a family — the contract does not try to detect this.
- Cryptographic family-key signing à la Tor. The Nym design substitutes chain-level identity (bond control via the mixnet contract) for Tor's per-family key signature — see Decision 1.

## Decisions

### Decision 1: Authority is delegated to the mixnet contract; there is no family key

**Choice.** Membership and invitation paths do not require any application-level signature beyond the chain's transaction signature. The contract proves "the sender controls this node" by cross-querying the mixnet contract (`query_nymnode_ownership` / `check_node_existence`), and relies on chain-level replay protection for every transaction.

**Why.** Tor's family-key construction protects against an off-chain attacker that can forge messages to a directory server. On Nym there is no off-chain channel — every state transition is a signed Cosmos SDK transaction whose sender address is authenticated by the chain itself. Bond ownership is already attested on-chain by the mixnet contract. Adding a second signing key per operator would duplicate the trust anchor at no security gain and complicate key management.

**Alternative considered.** A per-family signing key recorded on family creation, with every membership change requiring a signature from that key. Rejected as redundant given chain-level authentication.

**Consequence.** The contract is tightly coupled to the mixnet contract — its instantiate message *requires* the mixnet contract address, and that address is the sole authority for `OnNymNodeUnbond`. The address is bech32-validated at instantiation and persisted; it cannot be changed by a privileged update path (today). Replacing the mixnet contract would therefore require redeploying the families contract.

### Decision 2: Family names are normalised at the storage boundary

**Choice.** Family names are normalised by dropping every non-ASCII-alphanumeric character and lowercasing the rest (`crate::helpers::normalise_family_name`). The normalised form is stored alongside the user-supplied `name` and is what the unique-name index keys on.

**Why.** Users will submit `"My Family"`, `"my-family"`, `"  MyFamily  "` and expect them to be the same entity. A unique index on the raw `name` would let an adversary squat the canonical form with cosmetic variants. Doing the normalisation contract-side rather than client-side means every consumer sees the same uniqueness behaviour without trusting clients.

**Alternative considered.** Case-insensitive Unicode normalisation (NFKC + lowercasing). Rejected because CosmWasm contracts run in a deterministic Wasm sandbox where importing `unicode-normalization` materially grows the binary and increases gas cost; the operator population is small enough that ASCII-alphanumerics-only is acceptable.

**Consequence.** Non-ASCII names normalise to whatever ASCII letters remain (`"café"` → `"caf"`, `"名前"` → `""`). Names that normalise to the empty string are explicitly rejected with `EmptyFamilyName`. This is documented behaviour, not a bug.

### Decision 3: Family ids are monotonic and never recycled; `0` is the "no family" sentinel

**Choice.** The contract holds an `Item<NodeFamilyId>` counter that starts unset (treated as `0`) and is incremented to issue the next id. Disbanding a family does not free its id.

**Why.** Off-chain archives (the past-members and past-invitations maps) are keyed by `family_id`. Recycling ids would silently merge the history of two unrelated families and break correlation in downstream indexers. A monotonic counter is the simplest safe option.

**Alternative considered.** Hash-of-name ids. Rejected because the name can be normalised but the index must remain stable across renames (none today, but worth preserving headroom) and because numeric ids serialise more cheaply.

**Consequence.** The id space is `u32`. At realistic operator counts (single thousands) overflow is not a practical concern.

### Decision 4: Both archives use `((family_id, node_id), counter: u64)` keys

**Choice.** `past_family_members` and `past_family_invitations` are keyed by a composite of the `(family, node)` pair plus an explicit per-pair counter (`past_family_member_counter`, `past_family_invitation_counter` — both `Map<(NodeFamilyId, NodeId), u64>`).

**Why.** A node can be invited to (and join, leave, re-join, etc.) the same family multiple times. Using `env.block.time` as a disambiguator is unsafe because multiple transactions can share a block. Maintaining an explicit per-pair counter keeps archival writes O(1) (vs. an O(log n) range scan to find the next free slot) and gives the archive a stable, total order per pair.

**Alternative considered.** Storing every archive entry under a global sequence counter. Rejected because per-family and per-node listings (the dominant query shape) would then need to scan and filter rather than prefix-iterate.

**Consequence.** Archive cursors are composite (`(NodeId, u64)` per-family-scoped, `(NodeFamilyId, u64)` per-node-scoped, `((NodeFamilyId, NodeId), u64)` globally-scoped). They are publicly typed in the common crate so clients pass them back verbatim.

### Decision 5: No background sweeper for expired pending invitations

**Choice.** When an invitation's `expires_at` passes, the entry remains in `pending_family_invitations`. The only paths that clear it are explicit revoke (by the family owner), explicit reject (by the node controller), the unbonding callback (sweeps all invitations for a node), and disband (sweeps all invitations for a family). Read queries surface a boolean `expired` flag so consumers don't have to compare the timestamp themselves.

**Why.** A background sweeper would need either a CosmWasm cron extension (none available) or a per-block hook (no such hook exists in the contract — the only cross-contract entry is `OnNymNodeUnbond`). Triggering sweeps from the families contract's own execute paths only would still leave drift between expiry and removal. Surfacing `expired` lets read consumers decide their own policy without paying contract storage churn.

**Consequence.** `accept_invitation` is the only handler that refuses to act on an expired entry. Revoke and reject are explicit and idempotent under expiry — they are the operator's tool to clean storage. The expired-but-still-pending state is not pathological; it is the documented baseline.

### Decision 6: Owner-gated handlers derive the family from sender ownership, not from arguments

**Choice.** `DisbandFamily`, `InviteToFamily`, `RevokeFamilyInvitation`, and `KickFromFamily` do not accept a `family_id` argument — the family is resolved from the sender's ownership via the `families.owner` unique index.

**Why.** Passing the family id would force every handler to validate that the sender owns the supplied id, with one error path for "wrong family" and another for "no family". Deriving the family from ownership collapses both into the single `SenderDoesntOwnAFamily` error path and makes it impossible to act on someone else's family. The `KickFromFamily` case additionally checks that the target node's current family matches the owner's family, because `family_members` is keyed by node alone and the storage helper would otherwise silently strip a node from an unrelated family.

**Alternative considered.** Explicit `family_id` argument on every handler. Rejected as ergonomic noise that adds error surface without security gain.

**Consequence.** Accept and reject *do* carry `family_id` in the message, because the sender there is the *invitee* (node controller), not the family owner — they may simultaneously hold invitations from multiple families and must say which one they are acting on.

### Decision 7: Defence-in-depth pre-checks complement unique-index enforcement

**Choice.** `try_create_family` explicitly checks `may_get_owned_family` and `families.idx.normalised_name.item` *before* calling `register_new_family`, even though both invariants are also enforced by the underlying `IndexedMap` `UniqueIndex`. Likewise `add_pending_invitation` checks `may_load` before insert.

**Why.** A `UniqueIndex` violation surfaces as a generic CosmWasm storage error without any context (no family ids, no addresses). Pre-checking yields typed errors (`SenderAlreadyOwnsAFamily { address, family_id }`, `FamilyNameAlreadyTaken { name, family_id }`, `PendingInvitationAlreadyExists { family_id, node_id }`) that downstream wallets and tooling can surface meaningfully. The unique index is retained as a hard backstop so a bug in the pre-check cannot corrupt invariants.

**Consequence.** The cost is one extra storage read per checked invariant per call — negligible relative to the typical multi-write transaction body.

### Decision 8: The unbonding callback archives invitations as `Rejected`, not `Revoked`

**Choice.** When `OnNymNodeUnbond` sweeps a node's pending invitations, each is archived with `FamilyInvitationStatus::Rejected { at: now }`. The `Revoked` status is reserved exclusively for owner-side withdrawal (the explicit `RevokeFamilyInvitation` execute and the all-family sweep during `DisbandFamily`).

**Why.** From the *family's* point of view, an invitation that auto-clears because its target unbonded looks identical to one the target explicitly declined: both are invitee-side terminations beyond the family's control. Lumping them together as `Rejected` keeps the semantic boundary clean — `Revoked` means "the family changed its mind," `Rejected` means "the invitee said no (whether via explicit reject or by leaving the network)." Off-chain indexers that group by status get sensible buckets without needing a separate "auto-rejected" tier.

**Alternative considered.** A new `FamilyInvitationStatus::AutoExpired` (or `NodeUnbonded`) variant. Rejected because it adds enum surface for a distinction that no current consumer cares about, and because the historical record already preserves the *event* that triggered the archival (a `family_node_unbond_cleanup` event fires alongside, and on-chain tx history can be cross-referenced).

**Consequence.** Past-invitation queries cannot distinguish "node controller explicitly rejected" from "node unbonded and the invitation was swept" from the `status` field alone. Consumers that need the distinction must correlate with the emitted event or with the mixnet-contract unbonding history. Adding a new variant later would be a breaking schema change.

### Decision 9: Schema-feature-gated response types in the common crate

**Choice.** The common crate's `msg.rs` puts every `#[cfg_attr(feature = "schema", returns(...))]` annotation behind a `schema` feature, and the response types are only imported when the feature is active.

**Why.** The contract crate itself does not need `cosmwasm-schema` at runtime; only the `bin/schema.rs` schema-emitter (which writes `schema/node-families.json` and the per-message JSON files) does. Gating keeps the production Wasm binary lean.

**Consequence.** When generating schemas, builds must enable the `schema` feature on `nym_node_families_contract_common`. This is automated in the contract's `Makefile` and `bin/schema.rs`.

### Decision 10: Unknown scope ids on paginated queries return an empty page, not an error

**Choice.** Per-family and per-node paginated queries (`GetFamilyMembersPaged`, `GetPendingInvitationsForFamilyPaged`, `GetPastMembersForNodePaged`, etc.) do not verify that the supplied `family_id` or `node_id` corresponds to an existing entity. An unknown scope id silently yields an empty page (`entries: []`, `start_next_after: None`) rather than a `FamilyNotFound`-style error.

**Why.** The underlying `cw_storage_plus::IndexedMap::range` over a `MultiIndex` prefix has no native existence check on the prefix itself — an unknown prefix is simply a range that yields zero entries. Surfacing this as an error would require an extra storage read per query to confirm the scope exists, and the per-family/per-node membership maps don't carry a "this scope was ever populated" sentinel anyway (a disbanded family has zero remaining members, and so does one that never had any). Treating "no entries" identically in both cases collapses two paths into one. Callers who care about the distinction can pair the listing with `GetFamilyById { family_id }` (which *does* return `Option<NodeFamily>`).

**Alternative considered.** Returning `FamilyNotFound { family_id }` (or analogous) when the scope id is unknown. Rejected because (a) it inflates the cost of every paginated read by one extra storage check, (b) it makes paginate-then-filter consumer patterns more awkward without payoff, and (c) the "scope known but empty" and "scope unknown" cases are observationally identical to most consumers.

**Consequence.** Spec scenario "Unknown scope id yields an empty page rather than an error" makes this explicit. Tooling that surfaces "family not found" errors needs to perform its own existence check via the single-family query — the listing endpoints do not provide it.

## Risks / Trade-offs

- **[Mixnet contract address is locked at instantiation]** → If the mixnet contract is ever redeployed at a different address, the families contract becomes orphaned (the unbonding callback path no longer fires). **Mitigation:** documented as an operational constraint; a future migration could add an `UpdateMixnetContractAddress` admin-only execute. Today there is no such path.
- **[No sybil resistance]** → A malicious operator running multiple nodes can simply not declare a family. **Mitigation:** out of scope here; the proposal explicitly accepts this and points to operator verification as a follow-on.
- **[Expired invitations bloat storage]** → A family that issues invitations and never sweeps them accrues dead entries. **Mitigation:** disband sweeps all of them at once; the family can also revoke individually. The `expired` flag on read queries means consumers don't need them gone to act correctly.
- **[Counter overflow in archive maps]** → `u64` per pair counter cannot realistically overflow but is technically unbounded. **Mitigation:** not a real concern at any practical operator/node scale.
- **[Cross-contract query gas cost]** → Every invite/accept/reject/leave/kick does at least one `query_nymnode_ownership` or `check_node_existence`. **Mitigation:** these are simple `Item`/`Map` reads on the mixnet contract side; cost is bounded.
- **[Disband cost scales with pending invitation count]** → Disband sweeps all of a family's pending invitations in a single transaction. A family with thousands of pending invitations could push past gas limits. **Mitigation:** documented; in the worst case the owner can pre-revoke invitations in batches.

## Migration Plan

Not applicable to this change — this is a documentation-only artefact for code that has already shipped. The contract itself is initially deployed via the standard `cosmwasm_contracts.rs` orchestrator wiring (which the anchor commit added), and its `migrate` entry point uses `cw2::ensure_from_older_version` to gate version bumps. There is no state migration in the anchor commit; `queued_migrations.rs` is a stub awaiting future need.

Future spec deltas that *do* change behaviour should:

1. Document the storage-key change (if any) and add a corresponding entry to `queued_migrations`.
2. Bump `CARGO_PKG_VERSION` in `contracts/node-families/Cargo.toml`.
3. Coordinate the `MigrateMsg` invocation with the chain-governance migration tx that pushes the new code id.

## Resolved Questions

The three questions considered during the spec walk-through, with the team's resolutions on 2026-05-20:

- **Mixnet contract address updatability** → keep locked at instantiation. The mixnet contract is not expected to be redeployed under a new address; if it ever is, the families contract is redeployed alongside it. Avoiding the admin-gated `UpdateMixnetContractAddress` execute means admin compromise cannot hijack the unbonding callback authority.
- **Length-limit units** → keep byte-counted via `String::len`. Operator-supplied family names are overwhelmingly ASCII in practice; pulling in `unicode-segmentation` or even paying the `chars().count()` cost is not justified for short identifiers.
- **Rename / update handlers** → do not add. The only post-creation mutation remains the `members` counter. Owners who want to change a name or description disband and recreate (the fee is refunded). Keeps the execute surface, event surface, and test surface minimal.

No questions remain open.
