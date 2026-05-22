## Why

The ecash CosmWasm contract (`contracts/ecash/`) is the on-chain anchor of the ticketbook credential pipeline. Clients escrow funds with the contract, which mints a sequential `deposit_id` and persists the ed25519 identity public key that the depositor will later use when requesting a blind signature from nym-api signers (`post_blind_sign` at `nym-api/src/ecash/api_routes/partial_signing.rs:55`). The contract itself does **not** verify ed25519 ownership — that proof lives in `BlindSignRequestBody` and is enforced off-chain; the contract's job is to provide a tamper-evident, gas-efficient on-chain record that signers can read by id. Gateways do not escrow funds here; they only participate on the redemption side (see `RequestRedemption` / `RedeemTickets`).

The contract has shipped, is live, and exists only as Rust source plus inline comments. Several design choices are non-obvious from reading the code alone and have a habit of being re-derived during incident triage:

- The use of `cw_controllers::Admin` as a generic address-equality helper for two distinct slots (`contract_admin` and `multisig`). The wrapper is named for the role it was designed for, but the contract uses it only for its `set` / `assert_admin` / `get` mechanics — `multisig` is not an "admin" in any operational sense, it is a stored pointer to the cw3 contract that gates `RedeemTickets`. Only `contract_admin` carries actual admin semantics (replaceable via `UpdateAdmin`).
- The deposit-storage shortcut that bypasses `cw_storage_plus` JSON serialisation in favour of raw 32-byte ed25519 representation under the `deposit` namespace.
- The tier-stratified bookkeeping (`PoolCounters` + `DepositStatsStorage`) that maintains a global total **and** a per-account custom-price total, while preserving the invariant `default_count + sum(custom_count) == total_deposits_made`.
- The blacklist surface is **fully wired but stubbed**: storage, helpers, reply handler, and event attribute exist; both `ProposeToBlacklist` and `AddToBlacklist` short-circuit to `UnimplementedBlacklisting`. This is intentional and the dead branches are preserved in source as commented-out reference for the eventual redesign.
- The `RedeemTickets` path is legacy dead code — still callable by the multisig, still bumps a counter and emits an event with `moved_to_holding_account="false"`, but no active code path or consumer depends on those side effects. Retained because it has not been cleaned up yet, not because anything requires it. A follow-on cleanup change should consider removing it (breaking-schema, but no known live consumer).

Capturing the spec now — while the original author and reviewers are still around — is materially cheaper than reconstructing it from `git blame` later.

## What Changes

- Introduce a new capability spec `ecash-contract` covering the on-chain CosmWasm contract: instantiation, deposit submission (default + reduced tier), legacy redemption (`RedeemTickets`), redemption-proposal creation (`RequestRedemption`) and reply handling, admin operations (admin transfer, default deposit value, set/remove reduced price), the stubbed blacklist surface, the full query surface, migration (tiered-pricing backfill), and the storage / event / error surface that downstream tooling treats as a public contract.
- Document the **authorisation model**: `contract_admin` (a real admin, replaceable via `UpdateAdmin`) gates pricing / whitelist mutations; `multisig` (a stored cw3 contract pointer using the same `cw_controllers::Admin` wrapper for address-equality only) gates `RedeemTickets`. Per-handler authorisation is enumerated.
- Document the **deposit identity semantics**: the contract stores the claimed bs58-encoded ed25519 public key but does not verify control of the corresponding private key — that proof is performed off-chain by nym-api signers when honoring the `post_blind_sign` request.
- Document the **anti-double-issue boundary**: the contract guarantees globally-unique sequential `deposit_id`s, but de-duplication of credential issuance per deposit is enforced by each nym-api signer's local store (`state.already_issued`), not by the contract.
- Document the **stubbed blacklist surface** as a public-surface fact (current versions always return `UnimplementedBlacklisting`).
- Document the **storage layout** (raw key namespaces, `Item`/`Map` keys, the custom raw-bytes encoding used for the `deposit` namespace) and **event surface** (`deposited-funds`, `ticket_redemption`, plus attributes on auto-generated `wasm` events) since indexers and signers consume these as a stable interface.

No code changes. No migrations. No new dependencies. This is a documentation-only deliverable that ratifies the current implementation as the baseline. A follow-on change `ecash-contract-rustdoc` will tighten in-source rustdoc to mirror the captured spec.

## Capabilities

### New Capabilities

- `ecash-contract`: the CosmWasm contract that escrows deposits for ticketbook credentials, mints sequential deposit ids, stores claimed ed25519 identity keys for off-chain signer validation, gates redemption finalisation behind a multisig, and tracks tier-stratified deposit statistics.

### Modified Capabilities

_None — there are no existing specs in `openspec/specs/` for the ecash contract and this change does not alter on-chain behaviour._

## Impact

- **Affected code**: none modified. The spec is derived from `contracts/ecash/` and `common/cosmwasm-smart-contracts/ecash-contract/` at HEAD on `develop`.
- **Affected consumers** (documented for traceability, not changed):
  - `nym-api/src/ecash/` — reads deposits by id via `state.get_deposit(deposit_id)`, then at `post_blind_sign` (`partial_signing.rs:55`) enforces (a) per-signer-local double-issue prevention by checking `state.already_issued(deposit_id)` against its local `issued_partial_signature` store and returning the cached signature on a hit, and (b) ed25519 ownership by parsing `deposit.bs58_encoded_ed25519_pubkey` and verifying `request.signature` against the plaintext `IssuanceTicketBook::request_plaintext(&request.inner_sign_request, request.deposit_id)` (`nym-api/src/ecash/deposit.rs::validate_deposit`). Additional gating performed at the same entry point — signer-epoch eligibility (`ensure_signer`), expiration-date sanity, DKG-not-in-progress (`ensure_dkg_not_in_progress`), and an off-chain blacklist check (`aux.ensure_not_blacklisted`) — is independent of contract state.
  - Clients — submit `DepositTicketBookFunds` to escrow funds and register a claimed ed25519 identity key.
  - Gateways (legacy + current) — request batch redemption via `RequestRedemption` (current) or receive the transfer via `RedeemTickets` (legacy multisig-gated path); they do not call `DepositTicketBookFunds`.
  - The cw3 multisig contract — receives `Propose` messages for batch redemption and (eventually) blacklisting; the ecash contract reads back the resulting `proposal_id` in its reply handler.
  - The cw4 group contract — referenced via `Config::group_addr`; intended to gate the (currently stubbed) blacklist proposal flow to voting members.
  - Chain indexers — consume the `deposited-funds` event (`deposit-id` attribute), the `ticket_redemption` event, and the `proposal_id` attribute on the auto-generated `wasm` event from redemption-proposal replies.
- **Dependencies**: none. CosmWasm storage layout (raw key strings: `"contract_admin"`, `"multisig"`, `"config"`, `"pool_counters"`, `"expected_invariants"`, `"deposit_ids"`, `"deposit"`, `"reduced_deposits"`, `"blacklist"`, `"deposits_with_default_price"`, `"deposits_with_default_price_amounts"`, `"deposits_with_custom_price"`, `"deposits_with_custom_price_amounts"`) is part of the spec surface — changing those keys is a breaking change for already-deployed contracts and must be treated as such by any future delta. The two reply IDs (`BLACKLIST_PROPOSAL_REPLY_ID = 7759`, `REDEMPTION_PROPOSAL_REPLY_ID = 2137`) are similarly load-bearing.
- **Non-goals**: the off-chain blind-signature protocol, nym-api signer state machine (`already_issued`, blacklist enforcement, DKG epoch gating), the ticketbook protocol itself, gateway-side commitment construction, the multisig contract internals, and the (currently stubbed) blacklist redesign. These all consume the contract or are consumed by it but live outside its boundary and will get their own specs in follow-on changes.
- **Known limitation — blacklist is stubbed**: `ExecuteMsg::ProposeToBlacklist` and `ExecuteMsg::AddToBlacklist` are part of the public schema but always return `EcashContractError::UnimplementedBlacklisting`. The reply handler for `BLACKLIST_PROPOSAL_REPLY_ID`, the `blacklist` storage map, and the `Blacklisting` type are all wired in code but unreachable from the public ExecuteMsg surface. Existing queries (`GetBlacklistedAccount`, `GetBlacklistPaged`) succeed but will return empty results on a freshly deployed contract. The spec documents this as the current contract surface; the redesign is out of scope.
