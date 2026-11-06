## Why

The ecash contract is the only contract built on sylvia, and sylvia has no release for cosmwasm-std 3 (its last tag is 2.0.0-rc.1 against 3.0.0-rc.0; the upstream port, CosmWasm/sylvia#495, does not compile against 3.0.1 or 3.0.11, and the repository has been inactive since mid-2025). Removing sylvia is a precondition for the cosmwasm-std 3.0.11 upgrade, and doing it now, while the contracts still build against 2.2.2, lets the rewrite be checked against the sylvia-generated wire format it replaces. The rewrite also collapses the two copies of the ecash messages (sylvia's generated one, which the contract parses, and the hand-written one in `nym-ecash-contract-common`, which every client sends) into one, removing a mismatch that nothing enforces today.

## What Changes

- The contract exposes plain `#[entry_point]` functions (`instantiate`, `execute`, `query`, `migrate`, `reply`) that dispatch on `nym_ecash_contract_common::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg}`, following the other contracts in the workspace.
- The contract takes the layout the other contracts share: entry points in `contract.rs`, `try_*` handlers in `transactions.rs`, `query_*` handlers in `queries.rs`, storage handles in `storage.rs` behind a `pub const` instance. Handlers take `DepsMut`/`Deps`, `Env` and `MessageInfo` instead of sylvia's context structs; their logic, storage keys, events, errors and reply ids are unchanged.
- The empty `sudo` entry point that sylvia exported is no longer exported.
- The contract accepts the aliases the common crate already declares and sylvia never accepted: `update_deposit_value` for `UpdateDefaultDepositValue`, and `GetRequiredDepositAmount` alongside `get_required_deposit_amount` for `GetDefaultDepositAmount`. Every message the deployed contract accepts today is still accepted with the same result.
- The error text for an unrecognised message changes from sylvia's "Unsupported message received: ..." to serde's unknown-variant error. Nothing matches on it.
- A wire-compatibility test, written while sylvia is still a dependency, pins the JSON of every message variant to what the sylvia-generated types produce and accept, and stays behind as a golden fixture.
- Tests move to the `TestableNymContract` harness, next to the code they test; a new App-level test covers the redemption reply path, which has no test today.
- `sylvia` leaves `contracts/ecash/Cargo.toml` and the `contracts/` workspace.
- The tiered-pricing migration (`add_tiered_pricing`), which has run on every network, is removed with its tests. `MigrateMsg` becomes `{}` like the other contracts' and `WhitelistedDeposit`, used only by it, leaves `nym-ecash-contract-common`. The migrate handler only refreshes build information and the cw2 version.
- No mainnet migration in this change: the new build ships with the next ecash migration that has a functional reason.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ecash-contract`: adds a requirement fixing the entry-point set and the message types the contract parses (wire-compatible with the sylvia build, no `sudo`); the default-deposit query requirement now describes one query variant accepted under three names instead of two variants; `UpdateDefaultDepositValue` is also accepted as `update_deposit_value`; the tiered-pricing migration requirement is replaced by one for an empty `MigrateMsg` that only refreshes build information and rejects downgrades, and the ticketbook-size and error-variant requirements drop their mentions of migration whitelist seeding.

## Impact

- Code: `contracts/ecash/src/` restructured (`contract/` directory and `multitest.rs` replaced by `contract.rs`, `transactions.rs`, `queries.rs`, `storage.rs`, `testing.rs`, `queued_migrations.rs`; `helpers.rs`, `deposit.rs`, `deposit_stats.rs`, `constants.rs` stay), `contracts/ecash/Cargo.toml`, `contracts/Cargo.toml` (workspace `sylvia` entry), `contracts/Cargo.lock`, `common/cosmwasm-smart-contracts/ecash-contract/src/msg.rs` (`MigrateMsg`, `WhitelistedDeposit`).
- Wire: no change for any client; the contract parses the types clients already send, and the execute, query and instantiate messages and every response type are untouched. The JSON schema (`contracts/ecash/schema/`) changes only in the migrate message; the TypeScript clients already declare `MigrateMsg {}` and are unchanged.
- Deployment: none in this change. The artifact is built with the optimizer and checked with `cosmwasm-check`; mainnet stays on code 71 until a functional ecash migration picks the build up.
- Dependencies: `sylvia` (and with it `sylvia-derive`, `serde-cw-value`, `konst` and the rest of its private graph) leaves the contracts lockfile.
