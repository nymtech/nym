## Context

This change is the first of two dependency removals that precede the cosmwasm-std 3.0.11 upgrade of the contracts workspace. sylvia and the cw-plus crates (cw3, cw4, cw20, cw3-fixed-multisig, cw20-base) are the only contract dependencies with no std-3 release. Both are removed while the workspace is still on std 2.2.2, so each replacement can be checked byte for byte against the original it replaces (message JSON, storage keys), which stops being possible once the originals leave the dependency graph. The std bump comes after this change and the cw-plus replacement have merged.

sylvia upstream: the last release is 2.0.0-rc.1 (June 2025, against std 3.0.0-rc.0). The std-3 port, CosmWasm/sylvia#495 (branch `upgrade-to-cosmwasm-std-3.0.0`, head `32925e6548`), does not compile against std 3.0.1 or 3.0.11, its own test suite fails at that commit, and the repository has had no push since 2025-06-30.

Current contract (develop `7fad306899`):

- `contracts/ecash/src/contract/mod.rs` puts `#[entry_points] #[contract] #[sv::error(EcashContractError)]` on `impl NymEcashContract`. The struct holds the storage handles (`contract_admin`, `multisig`, `config`, `pool_counters`, `expected_invariants`, `deposit_stats`, `reduced_deposits`, `blacklist`, `deposits`) and is built by a `const fn new()`. There are 22 `#[sv::msg]` handlers: 1 instantiate, 10 query, 9 exec, 1 reply (the legacy form taking a raw `Reply`, under `#[allow(deprecated)]`), 1 migrate. Handlers take sylvia's `InstantiateCtx`/`ExecCtx`/`QueryCtx`/`MigrateCtx`/`ReplyCtx`; `contract/helpers.rs` takes `ExecCtx` in `create_redemption_proposal` and `create_blacklist_proposal`.
- The artifact exports `instantiate`, `execute`, `query`, `sudo`, `migrate`, `reply`. `sudo` comes from sylvia with an empty `SudoMsg`, so every sudo call fails today.
- Reply ids: `BLACKLIST_PROPOSAL_REPLY_ID = 7759`, `REDEMPTION_PROPOSAL_REPLY_ID = 2137` (`contracts/ecash/src/constants.rs`).
- Two copies of the messages exist. The contract parses sylvia's generated types; every client, and the schema binary (`contracts/ecash/src/bin/schema.rs`), use the hand-written `cw_serde` types in `common/cosmwasm-smart-contracts/ecash-contract/src/msg.rs`. Nothing enforces that they match. `contract/helpers.rs` serialises the hand-written `ExecuteMsg::RedeemTickets` into a multisig proposal that sylvia's type later parses.
- Differences between the copies: sylvia has a separate `get_required_deposit_amount` query variant (handler delegating to `get_default_deposit_amount`), where the hand-written `QueryMsg` declares `get_required_deposit_amount` and `GetRequiredDepositAmount` as aliases of `GetDefaultDepositAmount`; the hand-written `ExecuteMsg` declares `update_deposit_value` as an alias of `UpdateDefaultDepositValue` (the variant's name before the tiered-pricing change; mainnet code 65 accepted it, code 71 does not). sylvia's exec and query deserialisers first parse into an intermediate `serde-cw-value` tree, check the single top-level key against a fixed name list, and reject anything else with "Unsupported message received: ... Messages supported by this contract: ...". Its `InstantiateMsg` and `MigrateMsg` are plain structs with the same fields as the hand-written ones. Neither copy denies unknown fields.
- Tests: 38. In `contract/test.rs`, 18 tests call handlers with sylvia ctx structs (32 call sites). `multitest.rs` has 4 tests on sylvia's `CodeId`/`Proxy`/`App`. The other 16 (`queued_migrations.rs`, `deposit.rs`, `deposit_stats.rs`) touch no sylvia. No test exercises the reply handler.
- Nothing outside `contracts/ecash` depends on the `nym-ecash` crate: coconut-dkg's testable features pull the multisig and group contracts, and nym-api uses only `nym-ecash-contract-common`. The blast radius is the crate itself and the contracts lockfile, from which `sylvia`, `sylvia-derive` and their private dependencies (`konst`, `serde-cw-value`, `convert_case`, `proc-macro-error`, `proc-macro-crate`, ...) leave.
- Optimised artifact on develop (`opt-level` 3, `wasm-opt -Os`): 522,123 B, max 63 locals per function, 2,865 total locals.

Mainnet (read 2026-10-30): ecash at `n1r7s6aksyc6pqardx88k3rkgfagwvj4z4zum9mmz2sfk3zm2mha0sd4dnun`, code 71, a sylvia build. History: instantiated on code 52, migrated to 65 (msg `{}`), then to 71 at height 23436443 (msg `initial_whitelist`, the tiered-pricing migration). cw2 records `crate:nym-ecash 0.1.0`. The chain runs nyxd 0.61.16 (wasmd 0.61, CosmWasm 3.x VM); a std-2.2.2 artifact runs there unchanged.

## Goals / Non-Goals

**Goals:**

- No breaking change in either direction: every client built before this change works against the new contract, and every client built after it works against the deployed sylvia contract (code 71).
- `sylvia` leaves `contracts/ecash/Cargo.toml` and the `contracts/` workspace.
- The contract parses `nym-ecash-contract-common`'s message types through plain `#[entry_point]` functions, as the other contracts do.
- Wire compatibility with the sylvia build is proven by a test written while sylvia is still in the graph, and pinned afterwards by a golden fixture.
- All 38 existing tests keep passing in ported form; the redemption reply path gains a test.
- The optimizer builds the artifact and `cosmwasm-check` passes on it.

**Non-Goals:**

- The std 3 bump and its fallout (opaque `StdError`, lost `PartialEq` on error enums, `Uint256` coin amounts).
- Replacing cw3/cw4: `Config::group_addr: Cw4Contract` and the cw3 `ProposalResponse` query stay as they are; the cw-plus replacement change switches them.
- A mainnet migration, and any change to `MigrateMsg` or the migrate handler's behaviour.
- Removing `RedeemTickets` or the blacklist stubs, or tightening message parsing.

## Decisions

### Rewrite on plain entry points; do not fork sylvia (settled before this change)

The fork would mean owning about 8.8k lines of proc-macro code (7.4k in `sylvia-derive`, 1.45k in `sylvia`) with no upstream to track, starting from a port that does not compile. Moving to sylvia 2 instead would also reject the legacy reply form and renumber reply ids, so ecash's reply handler and tests change either way. The rewrite estimate is ~150 new lines of dispatch (22 arms), 22 handler signatures plus 2 helpers moved to plain types, mechanical edits at 32 test call sites and ~270 lines to port the 4 multitests: ~400-500 lines touched.

### Dispatch on `nym-ecash-contract-common`'s types (settled before this change)

The entry points deserialise into the types clients already send, and the sylvia-generated copy goes. This makes the mismatch between the two copies impossible rather than tested-for. The schema binary already writes the schema from these types, so `contracts/ecash/schema/` and the TypeScript clients generated from it do not change. Entry points follow `contracts/node-families/src/contract.rs` (`#[entry_point]` functions matching on the common crate's messages) and tests use the `TestableNymContract` harness from `nym-contracts-common-testing`.

### Keep `NymEcashContract` and its methods; change only their signatures

Handlers stay methods on `NymEcashContract` with their current names and message-field parameters; the sylvia ctx parameter is replaced by the context pieces each handler uses (`DepsMut` or `Deps`, `Env`, `MessageInfo`). `instantiate` and `migrate` take their message structs whole, because `InstantiateMsg` carries three address strings that a positional call could swap without a compile error. The five entry points live in a new `contract/entry_points.rs`; each builds `NymEcashContract::new()` and matches on the message. The reply entry point keeps the current id dispatch. Alternative: free functions split into `transactions.rs`/`queries.rs` as in node-families. Rejected: it moves every handler for no gain, since the struct already owns the storage handles, and it would bloat the diff that reviewers must check for unchanged logic.

### Compatibility holds in both directions

Clients to old contract: `nym-ecash-contract-common` is not modified, so a client built after this change serialises the same bytes as one built before it; `serde` aliases affect parsing only, and serialisation always emits the canonical snake_case names the sylvia build accepts. The schema and the TypeScript clients come from the same untouched crate. Old clients to new contract: the golden fixture proves every message the sylvia types serialise is accepted by the new parse path with the same variant; response JSON is unchanged because the handlers already return the common crate's response types through the same std 2.2.2 serialiser; storage keys, events, `set_data` payloads, error variants and reply ids are unchanged handler bodies. The only observable differences between the builds are the two extra accepted names, the error text for an unrecognised message name (fails on both builds, parsed by nothing), and the missing `sudo` export (governance-only, always errored).

### Accept the aliases the common crate declares

The contract becomes a superset of the deployed one: `update_deposit_value` and `GetRequiredDepositAmount` are newly accepted, `get_required_deposit_amount` keeps working (now as an alias rather than its own variant, same result). Both newly accepted names are harmless: one is admin-gated, the other is a read-only query. Alternatives: strip the aliases from `nym-ecash-contract-common` (a deserialisation change in a crate published on crates.io, for no benefit), or give the contract its own alias-free copy (brings back the two-copies problem this change removes).

### Unknown fields stay ignored (settled before this change)

`cw_serde` adds only `#[serde(crate = ..)]` and `rename_all = "snake_case"` in both std 2.2.2 and 3.0.11, never `deny_unknown_fields`, and sylvia's generated enums carry no `deny_unknown_fields` either. The rewrite changes nothing here.

### Accept the parse-path differences

sylvia's intermediate value tree, its top-level name check and its "Unsupported message received" text go; std's direct `serde` parse takes over, as in every other contract. For every message sylvia's own types serialise, both paths accept the same JSON (the golden test below proves it). For malformed input no client produces, behaviour now matches the rest of the workspace rather than sylvia. Nothing in the repo matches the old error text: the only client-side string matching on contract errors is on the vesting "Account does not exist" text and the mixnet pledge cap.

### Drop `sudo`

It had an empty message type, so every call already failed, and only chain governance can call `sudo`. The export list of the new artifact is the old one minus `sudo`.

### Golden fixture: inline literals in a `wire_compat` test module

A `#[cfg(test)]` module `contracts/ecash/src/wire_compat.rs` holds, per message kind, a list of cases pairing the JSON literal with the message values, so each literal sits next to what it pins. It is written in two steps.

1. With sylvia still a dependency: for every case, build the sylvia-generated message (`contract::sv::{InstantiateMsg, ExecMsg, QueryMsg, MigrateMsg}`) and the common crate's message with the same values; assert both serialise to the literal and the literal parses back into the common crate's message; assert sylvia's contract-level deserialisers (`sv::ContractExecMsg`, `sv::ContractQueryMsg`, `sv::InstantiateMsg`, `sv::MigrateMsg`) parse the literal into the sylvia-built message. sylvia's own `get_required_deposit_amount` variant has no common-crate counterpart to serialise, so a separate test pins its name and checks the common crate parses it as `GetDefaultDepositAmount`.
2. After sylvia is removed: drop the sylvia half. Keep: the common crate's message serialises to the literal; the literal deserialises back to an equal value; `update_deposit_value`, `get_required_deposit_amount` and `GetRequiredDepositAmount` inputs deserialise to their variants.

Cases: all 9 exec variants; all 10 sylvia query variants (`get_required_deposit_amount` maps to `GetDefaultDepositAmount` on the common side); the two paged queries with their `Option` fields both set and both `None`; `InstantiateMsg`; `MigrateMsg` with an empty and a one-entry whitelist. Literals rather than a fixture file, so a wire change shows up in review as a diff of the expected JSON. The values contain no control characters, so the fixture survives the std-3 switch from serde-json-wasm to serde_json, whose only output difference for our types is the escape case of a few control characters.

### Tests: direct handler calls stay, multitests move to the harness

`contract/test.rs`'s `TestSetup` instantiates through the `instantiate` entry point with an `InstantiateMsg` and keeps calling handler methods directly with `deps.as_mut()`/`deps.as_ref()`, `env` and `MessageInfo`. `multitest.rs` keeps its 4 tests, rewritten on `ContractTester<EcashContract>`, with the `TestableNymContract` impl in the same `#[cfg(test)]` file: nothing downstream needs a testable ecash contract, so there is no `testable-ecash-contract` feature. The impl overrides `dyn_contract()` to add `.with_reply(reply)`; the shared harness trait does not change. Alternative: an optional `reply()` on `TestableNymContract`. Rejected: it widens a trait shared by every contract for the one contract with a reply handler.

### A redemption reply test against a stub multisig

A test-only stub contract stands in for the multisig: its execute returns a response with the `proposal_id` attribute. The ecash contract is instantiated with the stub's address as `multisig_addr`; `RequestRedemption` then runs the SubMsg, the `reply` entry point, and `multisig_proposal_id()`, and the test asserts the response data is the id's big-endian bytes. A second case makes the stub fail and asserts the transaction fails. Alternative: the real cw3-flex-multisig with a cw4 group. Rejected: heavy setup for a test of ecash's own dispatch, on crates the cw-plus replacement is about to change.

### Test-assertion style: typed errors stay; only the storage type moves

The ported and edited tests keep `assert_eq!` on `EcashContractError`. The std 3 change converts error assertions to `err.to_string()` across every contract in one sweep, and the `Std` variant's text changes in std 3, so converting a subset now adds churn without removing any of that sweep. In files this change already edits, `cosmwasm_std::MemoryStorage` becomes `cosmwasm_std::testing::MockStorage`, which exists in 2.2.2 and is the name std 3 keeps.

### No migration of its own (settled before this change)

The change ends at a merged, optimizer-built, checked artifact. It ships inside the next ecash migration that has a functional reason, which may turn out to be the std 3 build. Rejected: a migration right after this change (isolating the rewrite's risk is not worth an extra migration round of the funds-holding contract), and bundling with the std 3 build by rule (no reason to tie the two). The crate version stays 0.1.0; the migration that ships the build decides the bump.

## Risks / Trade-offs

- [A dispatch arm wires a variant to the wrong handler, or swaps two same-typed fields; `GetDepositsPaged { limit, start_after }` has two `Option<u32>`] → Arms bind fields by name; `instantiate`/`migrate` take whole structs; a test queries `GetDepositsPaged` through the `query` entry point with distinct `limit` and `start_after` values; the multitests drive every path they cover through the entry points.
- [A missing `reply` export would make every `RequestRedemption` fail on chain, since the SubMsg uses `reply_always`] → The stub-multisig test exercises the reply through cw-multi-test, and the artifact's export list is compared with the develop artifact's.
- [The golden fixture covers well-formed messages only] → By design: malformed input now behaves as in every other contract.
- [The contract accepts two more message names than the deployed one] → No authorisation change: one is admin-gated, the other read-only.
- [cw-multi-test may shape reply events differently from the chain] → The test asserts through `multisig_proposal_id()`, so a mismatch fails loudly rather than passing silently.
- [Per-call gas moves with the parse path] → Not gated; the localnet run of the migration that ships this build shows real figures.

## Migration Plan

No deployment in this change. Hand-off for the migration that ships this build: the `migrate` handler re-runs `queued_migrations::add_tiered_pricing`, which overwrites `deposits_with_default_price` and its amount with the all-time totals. Mainnet already ran it at code 71; running it again after any custom-price deposit would count those deposits as default-price ones and break the statistics invariant. That migration must replace the migrate body and decide `MigrateMsg` for itself. Before it, this change's artifact is checked with `cosmwasm-check` 2.2.9 (the Makefile default) and 3.0.11 (the chain's VM).

## Open Questions

None.
