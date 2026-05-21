## Context

The ecash CosmWasm contract is the on-chain leg of the ticketbook credential protocol. It receives deposits from clients, mints a globally-unique sequential `deposit_id`, persists the depositor-claimed ed25519 identity public key, and exposes that record to off-chain nym-api signers that perform the blind-signing protocol. The contract is implemented in `contracts/ecash/` with the shared message / type / event surface in `common/cosmwasm-smart-contracts/ecash-contract/`, and is built on the [`sylvia`](https://docs.cosmwasm.com/sylvia) macro framework rather than hand-rolled `instantiate`/`execute`/`query` dispatchers.

The contract has shipped, is in active use, and integrates with two other contracts in the workspace: a `cw4` group contract (referenced via `Config::group_addr`) and a `cw3`-style multisig contract (referenced via the `multisig` `Admin` slot). The mixnet contract is **not** in the trust path here — that is a node-families concern.

This document captures the architectural choices behind the contract as it exists today, so reviewers, integrators (nym-api signers, gateways, indexers), and future maintainers have a single normative reference. There is no behaviour change being proposed.

## Goals / Non-Goals

**Goals:**
- Capture the trust boundary between the contract and nym-api signers: which checks the contract owns (deposit price, deposit-id uniqueness, persistence of claimed pubkey) and which it relies on signers to perform off-chain (ed25519 ownership proof, double-issue prevention, blacklist enforcement).
- Document the **authorisation model**: `contract_admin` is a real admin (pricing / whitelist mutations, replaceable via `UpdateAdmin`); `multisig` is a stored cw3 contract pointer that gates `RedeemTickets`. Both happen to live in `cw_controllers::Admin` slots, but the wrapper is used only as a generic address-equality helper for the second.
- Document the **tiered-pricing data model** (default deposit + per-address `reduced_deposits` overrides) and the statistics invariant tying them together.
- Document the **raw-bytes deposit storage** shortcut and its trade-offs.
- Document the **stubbed blacklist surface** as part of the public schema, including the dead-but-wired code paths.
- Document the **storage-key, event, and reply-id constants** that form the contract's external interface for indexers and upgrades.

**Non-Goals:**
- The off-chain blind-signature protocol or nym-api signer state machine (`already_issued`, blacklist enforcement, DKG epoch gating, partial-signature aggregation).
- The cw3 multisig contract internals (proposal voting, threshold logic).
- The cw4 group contract internals.
- The ticketbook protocol itself (zkSNARK construction, partial vs aggregate signature combination, expiration date signatures, coin-indices signatures).
- The eventual pool contract that `holding_account` is reserved for — its scope and storage transition is out of scope here.
- Re-enabling the blacklist. The redesign is acknowledged as future work; the spec captures the *current* surface, which is stubbed.

## Decisions

### Decision 1: `cw_controllers::Admin` is used as a generic address-equality helper, not as a "two admins" model

**Choice.** The contract has two `cw_controllers::Admin` slots, but only one is an admin in the operational sense:

- `contract_admin` — a real admin. A single Cosmos SDK address that gates pricing mutations (`UpdateDefaultDepositValue`, `SetReducedDepositPrice`, `RemoveReducedDepositPrice`) and admin transfer (`UpdateAdmin`). Set on instantiation to the message sender. Replaceable via the `cw_controllers::Admin::execute_update_admin` handshake (driven by `UpdateAdmin`).
- `multisig` — a stored pointer to the cw3 multisig contract. Used only for its address-equality mechanics: `assert_admin(deps, sender)` reduces to "is the caller equal to the stored address?", which on the ecash contract's side is the entire gating logic for `RedeemTickets`. There is no admin-transfer handshake on this slot and no execute path that mutates it.

**Why this shape.** Putting `multisig` in an `Admin` slot is a misuse of the wrapper's name — `cw_controllers::Admin` was designed for full admin semantics (set / get / assert / transfer), but the contract uses only `set` (once, at instantiation), `get` (in `must_get_multisig_addr`), and `assert_admin` (in `RedeemTickets`). The wrapper happened to be a convenient "remember-this-address-and-let-us-check-equality-later" helper. A reviewer reading `self.multisig.assert_admin(...)` should mentally substitute "is the caller our configured cw3 contract?" — there is no internal multi-signature scheme on the ecash contract itself.

**Alternative considered.** A plain `Item<Addr>` for the multisig pointer with a small inline `assert_caller_is_multisig` helper. Equivalent at runtime; would have made the naming honest. Not changed because storage key `"multisig"` is part of the public surface and renaming it would require a coordinated migration.

**Consequence.** The `"multisig"` storage key is preserved (renaming would break already-deployed contracts) and the spec describes the actual mechanics rather than the suggestive wrapper name. The `multisig` slot's `Admin`-shape transfer machinery is reachable in code but is never exercised by any execute path. New reviewers should read the wrapper choice as historical, not as a statement about authorisation design.

### Decision 2: The contract stores the claimed ed25519 pubkey but never verifies control of the private key

**Choice.** `DepositTicketBookFunds { identity_key }` accepts a bs58-encoded ed25519 public key as an opaque string and persists it under the deposit's id. The contract performs only schema-level validation (the bs58 string must decode to exactly 32 bytes when read back via `Deposit::to_bytes` / `try_from_bytes`).

**Why.** Verifying possession of the corresponding private key on-chain would require either a signature over a contract-supplied challenge (round-trip + chain-state machinery that CosmWasm does not provide ergonomically) or trusting the depositor's Cosmos SDK transaction signature, which is keyed on a separate address and proves nothing about the supplied ed25519 key. The ownership proof is naturally produced where the key is *used*: in the `BlindSignRequestBody` that the depositor later sends to nym-api signers, where they bind a request-specific signature to the claimed key.

**Consequence.** Anyone willing to pay the deposit can submit a deposit claiming any ed25519 pubkey. nym-api signers MUST verify the ed25519 ownership proof at `post_blind_sign` time before issuing a partial blind signature; the contract is not the enforcement point. Spec scenarios make this explicit so that future readers do not mistake the contract for an authentication boundary.

### Decision 3: Deposit-issue de-duplication is per-signer-local, not on-chain

**Choice.** Each `deposit_id` is unique globally (monotonic `u32` counter), but the contract has no notion of "issued" or "consumed." It is the responsibility of each nym-api signer to maintain its own ledger (`state.already_issued(deposit_id)`) and return the cached blinded signature instead of re-signing.

**Why.** The blind-signature protocol is partial per signer — different signers operate independently and must each refuse to re-issue against the same deposit. A contract-side "issued" flag would either need every signer to write back (cross-contract write storm, signer-trust assumption) or would only enforce single-issuance globally without preventing a malicious signer from re-issuing locally. Local ledgers solve the actual threat (a signer being tricked into issuing twice) at the right layer.

**Consequence.** A deposit can be queried any number of times and its `(deposit_id, identity_key)` pair is always retrievable. Replay protection at the credential layer is entirely a signer concern. The spec calls this out as a deliberate boundary so that the absence of an "issued" state on the contract is not read as a bug.

### Decision 4: Deposits are stored as raw 32-byte ed25519 pubkeys under a custom namespace, bypassing `cw_storage_plus` JSON serialisation

**Choice.** `DepositStorage` writes deposits via `storage.set(&storage_key, &bytes)` where `bytes` is the 32-byte raw ed25519 pubkey extracted from the bs58 string. The namespace is `b"deposit"`, keyed by the big-endian `u32` deposit id. Reads use a matching custom `StoredDeposits` reader. This sits alongside a normal `cw_storage_plus::Item<DepositId>` for the counter (`"deposit_ids"`).

**Why.** A bs58-encoded ed25519 public key is ~44 bytes; raw is exactly 32 bytes. The JSON-serialised `Deposit { bs58_encoded_ed25519_pubkey: "..." }` adds the field-name overhead on top. Over a contract lifetime that may accumulate hundreds of thousands of deposits, the savings compound and meaningfully reduce storage gas for paginated reads.

**Alternative considered.** A standard `Map<DepositId, Deposit>` keyed on `DepositId`. Rejected for storage cost; the duplication of a thin custom reader/writer is judged worth the gas savings.

**Consequence.** The `deposit` namespace is *not* a `cw_storage_plus::Map` — it cannot be iterated via `Map::range` and its key encoding is bespoke (`Path::new(b"deposit", ...)`). Any future migration that needs to walk deposits must use `StoredDeposits::deserialize_deposit_record`. Audit notes: this is the one spot where the contract steps outside the framework, deliberately. The spec lists `"deposit"` as a public storage namespace alongside `"deposit_ids"`.

### Decision 5: Deposit ids are sequential `u32`, start at 0, and the counter holds the *next* id

**Choice.** The counter `Item<DepositId>` (`"deposit_ids"`) is unset on a fresh contract and treated as zero. `next_id` returns the current value (the id assigned to the new deposit) and persists `current + 1`. `total_deposits_made` reads the counter directly — it always equals the number of deposits already performed. `latest_deposit` returns the counter as-is too, which is "the *next* id" — currently consumed only via `GetLatestDeposit`, which re-loads the deposit at that id (yielding `None` if the contract has not yet seen a deposit).

**Why.** Counting via the "next id" convention costs zero extra storage compared to a separate count field. The first deposit getting id `0` (rather than `1`) is convenient for protobuf-style optional defaults but is otherwise immaterial.

**Consequence.** A consumer that calls `GetLatestDeposit` on a fresh contract receives `LatestDepositResponse { deposit: None }`. The id space is `u32`; overflow at billions of deposits is not a practical concern but is technically unbounded.

### Decision 6: Three-tier pricing with explicit fall-back rules

**Choice.** Each `DepositTicketBookFunds` is classified at deposit time:

1. If the sent amount equals the configured **default** deposit, the deposit is treated as a default-price deposit (regardless of whether the sender is in the reduced-deposit whitelist).
2. Otherwise, if the sender has a reduced-deposit entry and the sent amount equals that reduced price, the deposit is treated as a reduced-price deposit and attributed to the sender's per-account counters.
3. Otherwise, the transaction errors with `EcashContractError::WrongAmount { received, amount }`, where `amount` is the reduced amount (if the sender is whitelisted) or the default amount.

**Why.** Whitelisted accounts often run automation that pre-dates their whitelist entry, sending the default amount. Treating that as "wrong amount" would break in-flight integrations; treating it as a default-price deposit gracefully degrades. Conversely, a whitelisted account that pays the *wrong* reduced amount is almost certainly misconfigured and should be told so explicitly.

**Consequence.** The statistics buckets (`deposits_with_default_price` vs `deposits_with_custom_price`) reflect what was actually paid, not the account's tier. The reduced-price storage map (`reduced_deposits`) is consulted only when classifying a non-default amount; it is never auto-applied. The invariant captured under Decision 7 binds these together.

### Decision 7: Statistics invariant: `default_count + sum(custom_count_per_account) == total_deposits_made`

**Choice.** Three accumulators run in parallel on every deposit:

- `PoolCounters.total_deposited` (`Item<Coin>`) — global value across all tiers; used by the eventual pool-contract migration.
- `DepositStatsStorage.deposits_with_default_price` (`Item<u32>`) and `..._amounts` (`Item<Coin>`) — global default-price totals.
- `DepositStatsStorage.deposits_with_custom_price` (`Map<Addr, u32>`) and `..._amounts` (`Map<Addr, Coin>`) — per-account custom-price totals.

The contract test suite asserts `default_count + custom_total_count == deposits.total_deposits_made()` after every deposit. `total_deposits_made` is derived from the deposit-id counter (Decision 5), tying everything back to a single source of truth.

**Why.** The query `GetDepositsStatistics` reassembles the picture by joining all three; without the invariant, indexers could observe inconsistencies (default + custom < total, or > total) during multi-tx blocks. Maintaining the invariant requires that *all* deposit writes go through `deposit_ticket_book_funds`. Raw storage writes (`storage.set(...)`) on the `deposit` namespace would silently break this.

**Consequence.** Future code that touches the `deposits` storage *must* also update the corresponding stats counter, or it violates the invariant. The migration path (`queued_migrations::add_tiered_pricing`) handles this by reading the pre-migration totals and backfilling them into `deposits_with_default_price[_amounts]` (since all pre-migration deposits were at the default price). The `assert_counts_consistent` test helper is shipped under `#[cfg(test)]` as the canonical post-deposit assertion.

### Decision 8: `expected_invariants.ticket_book_size` as a coordination tripwire

**Choice.** On instantiation, the contract snapshots `nym_network_defaults::TICKETBOOK_SIZE` into `Item<Invariants> { ticket_book_size }` (`"expected_invariants"`). Every code path that consults the ticketbook size (`get_ticketbook_size`) re-reads the stored value and compares it to the *current* crate constant; on mismatch it errors with `TicketBookSizeChanged { at_init, current }` — a guaranteed-loud failure intended to be impossible to silently ignore.

**Why.** The ticketbook size is a network-wide protocol parameter that pricing decisions bake in (default deposit min, reduced deposit min). If `nym-network-defaults` ever ships a new constant without a coordinated contract migration, every priced operation will fail loudly rather than silently mis-price. The check is a one-storage-read tax per priced operation in exchange for a hard-to-misconfigure deploy.

**Consequence.** Bumping `TICKETBOOK_SIZE` in network-defaults is a coordinated migration: deploy a new contract version, run `MigrateMsg`, which must overwrite `expected_invariants` with the new constant. The current `migrate` handler does *not* do this — it would need an additional queued migration. The spec captures the storage key and the error variant so the failure mode is discoverable.

### Decision 9: Reply IDs are hard-coded numeric constants with no schema metadata

**Choice.** `BLACKLIST_PROPOSAL_REPLY_ID = 7759` and `REDEMPTION_PROPOSAL_REPLY_ID = 2137`. The reply dispatcher returns `EcashContractError::InvalidReplyId { id }` for any unknown id.

**Why.** Reply IDs are private contract metadata — they carry no semantic content for external observers. Hard-coding keeps the dispatch path branchless and is consistent with the rest of the contract codebase. The arbitrary chosen integers do not need to be stable across redeploys, but *are* stable for any given deployed version.

**Consequence.** Tests that mock submessage replies must use these exact integers. Changing them between contract versions would invalidate any in-flight submessage (the in-flight reply would arrive after a chain restart with a different code id and dispatch to "unknown"). The spec lists them as part of the public surface even though no off-chain consumer reads them.

### Decision 10: Stubbed blacklist remains in the public schema as wired-but-unreachable code

**Choice.** `ExecuteMsg::ProposeToBlacklist`, `ExecuteMsg::AddToBlacklist`, `QueryMsg::GetBlacklistedAccount`, `QueryMsg::GetBlacklistPaged`, the `blacklist: Map<BlacklistKey, Blacklisting>` storage, the `Blacklisting` type, the `create_blacklist_proposal` helper, and the `handle_blacklist_proposal_reply` reply handler are all present in source. Both execute handlers short-circuit with `EcashContractError::UnimplementedBlacklisting`; the queries succeed but always return empty results on a freshly deployed contract.

**Why.** The blacklist is an acknowledged-incomplete feature. Removing the schema entries would be a breaking change for any client that already encodes them; removing the storage would obstruct future migrations. Keeping the wiring with explicit `Err(UnimplementedBlacklisting)` plus commented-out original implementations makes the redesign starting point obvious without exposing partial behaviour.

**Alternative considered.** Delete the schema variants outright. Rejected because (a) the schema is consumed by `cosmwasm-schema`-generated TypeScript clients and removal would cascade through gateway / indexer codebases, and (b) keeping the entries documents the intended future shape of the contract.

**Consequence.** The spec captures the stubbed handlers as first-class scenarios that always error, the blacklist queries as scenarios that always succeed with empty data, and the storage map as a public-surface namespace that exists but is unused. Any consumer treating an empty blacklist as a security guarantee is misreading the contract; the only enforcement is the off-chain `aux.ensure_not_blacklisted` check inside nym-api.

### Decision 11: `RedeemTickets` is a legacy multisig-gated path; `RequestRedemption` is the current entry point

**Choice.** Two redemption paths coexist:

- `RequestRedemption { commitment_bs58, number_of_tickets }` — callable by any gateway. Validates `commitment_bs58` is a 32-byte sha256 digest (bs58-decoded). Creates a `Propose` SubMsg to the multisig contract under the title `BATCH_REDEMPTION_PROPOSAL_TITLE = "ecash-redemption"` with `commitment_bs58` as the description. The reply handler records the multisig-issued `proposal_id` and exposes it via `Response::set_data(proposal_id.to_be_bytes())` so the gateway sees it back.
- `RedeemTickets { n, gw }` — callable only by the multisig (`self.multisig.assert_admin(...)`). Bumps `pool_counters.tickets_requested_and_not_redeemed += n`. Emits `Event::new("ticket_redemption").add_attribute("moved_to_holding_account", "false")`. The `gw` argument is intentionally unused at runtime — preserved in source comments as `"_ = gw;"` so that chain scrapers can attribute the redemption to a gateway by reading the raw transaction body.

**Why.** Originally `RedeemTickets` was the only redemption path and moved funds into the holding account. That mechanism was deprecated; the live path is the commitment-anchored `RequestRedemption` flow, which writes the proposal id and lets the multisig run the actual transfer logic. **`RedeemTickets` is dead code that has not been cleaned up yet.** It is still callable from any held multisig grant, still emits a recognisable event, and still bumps `tickets_requested_and_not_redeemed` — but no active code path or operational workflow relies on those side effects. The "_ = gw;" pattern and the `moved_to_holding_account="false"` attribute are remnants of the deprecated semantics, not load-bearing surface.

**Consequence.** New gateway code should call `RequestRedemption`. `RedeemTickets` is documented in the spec as "legacy, multisig-only, near-noop" to describe current behaviour accurately, without claiming any consumer depends on it. A follow-on cleanup change should consider removing the handler (breaking-schema for any client still encoding `RedeemTickets`, but no live consumer is known to do so). Until then, the spec describes the handler honestly and the rustdoc follow-on can flag it for deletion.

### Decision 12: Migration backfills default-price statistics from pre-tiered counters

**Choice.** `MigrateMsg { initial_whitelist }` runs `queued_migrations::add_tiered_pricing`, which:

1. Reads `deposits.total_deposits_made(storage)` and `pool_counters.total_deposited` from the pre-migration state.
2. Writes those values verbatim into `deposit_stats.deposits_with_default_price` and `deposits_with_default_price_amounts`.
3. Validates each entry in `initial_whitelist` (denom matches, amount strictly less than default, amount at least the ticketbook size) and saves it into `reduced_deposits`.

`cw2::ensure_from_older_version` and `set_build_information!` run alongside.

**Why.** Before the tiered-pricing migration, every deposit was made at the (single) default price. The migration formalises that invariant by populating the default-tier counters with the historical totals, so `GetDepositsStatistics` returns coherent numbers immediately after migration. Validating whitelist entries during migration prevents instantly-broken state on the new code path.

**Consequence.** The migration is **one-way** — it assumes the pre-migration state had no custom-price counters. Re-running it on an already-migrated state would clobber the default-tier accumulators with the *current* `total_deposits_made` (which now includes custom-price deposits). The spec calls this out; `queued_migrations.rs` is named with the expectation that future deltas will add new entries (each guarded by a version check) rather than mutate `add_tiered_pricing`.

## Risks / Trade-offs

- **[Pubkey-claim spoofing on deposit]** → Anyone with the configured funds can submit a deposit claiming any ed25519 pubkey, exhausting that pubkey's ability to be re-deposited at the same id later. **Mitigation:** nym-api signers verify ownership in `post_blind_sign`; a spoofed deposit is unredeemable. Documented as the trust boundary in Decision 2.
- **[Statistics invariant relies on entry-point discipline]** → Any code path that bypasses `deposit_ticket_book_funds` (raw storage writes, direct `next_id` calls in migrations) silently breaks `default + custom == total`. **Mitigation:** invariant enforced by the `assert_counts_consistent` test helper; documented as a maintenance contract.
- **[Stubbed blacklist creates a false expectation]** → Operators reading the schema may think the contract enforces blacklisting. **Mitigation:** spec scenarios marked explicit; `UnimplementedBlacklisting` error variant exists specifically to be a discoverable runtime indicator.
- **[Multisig address is locked at instantiation]** → If the cw3 contract is redeployed, the ecash contract orphans (redemption proposals can no longer be created against the new multisig). **Mitigation:** documented as an operational constraint; a future migration could add `UpdateMultisigAddress`. Today there is no such path.
- **[Holding account is reserved but unused]** → Funds never transfer to `holding_account` in the current contract; the field exists for the future pool-contract transition. **Mitigation:** documented; `Config` query exposes it, so off-chain monitoring can verify it remains the expected address.
- **[`RequestRedemption` gas grows with commitment churn]** → Each call creates a multisig proposal. A gateway issuing high-frequency redemptions accrues many open proposals. **Mitigation:** out of scope here; multisig-side concern.
- **[Reply-id collisions across upgrades]** → Hard-coded `7759` / `2137` cannot collide today but could if a future upgrade adds a third reply variant. **Mitigation:** noted; new reply ids should pick distinct integers and never reuse old ones.

## Migration Plan

Not applicable to this change — this is a documentation-only artefact for code that has already shipped. The contract's most recent state-altering migration was `add_tiered_pricing` (Decision 12), which has already been executed on live deployments.

Future spec deltas that *do* change behaviour should:

1. Add a new function to `queued_migrations.rs` rather than amending `add_tiered_pricing`.
2. Bump `CARGO_PKG_VERSION` in `contracts/ecash/Cargo.toml`.
3. Coordinate the `MigrateMsg` invocation with the chain-governance migration transaction that pushes the new code id.
4. Document any new storage namespace, event, or reply id under the corresponding spec requirement.

## Resolved Questions

Three of the four questions considered during the spec walk-through were resolved on 2026-05-21 by keeping current behaviour:

- **`holding_account` updatability** → keep locked at instantiation. The eventual pool-contract transition is handled as a fresh contract deploy rather than a mid-life mutation; matches the node-families precedent of treating cross-contract pointers as redeploy events. Avoiding an admin-gated update means admin compromise cannot retarget where future redemption funds would flow.
- **`multisig` address updatability** → keep locked at instantiation. Same rationale: if the cw3 multisig is redeployed under a new address, the ecash contract is redeployed alongside it. Not exposing an update path prevents admin compromise from hijacking redemption-finalisation authority.
- **`update_admin` renunciation** → keep the handler's mandatory-`Some(new_admin)` shape. Renunciation would leave pricing, whitelist, and admin-gated paths permanently unreachable — a one-way foot-gun with no operational benefit at the contract's current maturity.

## Open Questions

One question remains open and is deferred to a follow-on change rather than to this spec:

- **Stubbed blacklist final disposition.** `ProposeToBlacklist`, `AddToBlacklist`, `GetBlacklistedAccount`, and `GetBlacklistPaged` remain in the public schema; the execute handlers always return `UnimplementedBlacklisting`; the `blacklist` storage map, `Blacklisting` type, `create_blacklist_proposal` helper, and `handle_blacklist_proposal_reply` reply handler are wired but unreachable. The choice between (a) finalising the redesign, (b) removing the stubbed schema entries (breaking change), or (c) leaving them as-is is left to the blacklist redesign change owner. This spec captures the current surface — the stubbed schema is part of the contract surface today and the spec describes that faithfully (Decision 10, Requirement "Stubbed blacklist execute handlers", Requirement "Blacklist queries succeed and return empty").
