## Why

The ecash contract is the only contract built on sylvia, and sylvia has no release for cosmwasm-std 3 (its last tag is 2.0.0-rc.1 against 3.0.0-rc.0; the upstream port, CosmWasm/sylvia#495, does not compile against 3.0.1 or 3.0.11, and the repository has been inactive since mid-2025). Removing sylvia is a precondition for the cosmwasm-std 3.0.11 upgrade, and doing it now, while the contracts still build against 2.2.2, lets the rewrite be checked against the sylvia-generated wire format it replaces. The rewrite also collapses the two copies of the ecash messages (sylvia's generated one, which the contract parses, and the hand-written one in `nym-ecash-contract-common`, which every client sends) into one, removing a mismatch that nothing enforces today.

## What Changes

- The contract exposes plain `#[entry_point]` functions (`instantiate`, `execute`, `query`, `migrate`, `reply`) that dispatch on `nym_ecash_contract_common::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg}`, following the other contracts in the workspace.
- Handlers take `DepsMut`/`Deps`, `Env` and `MessageInfo` instead of sylvia's context structs; their logic, storage keys, events, errors and reply ids are unchanged.
- The empty `sudo` entry point that sylvia exported is no longer exported.
- The contract accepts the aliases the common crate already declares and sylvia never accepted: `update_deposit_value` for `UpdateDefaultDepositValue`, and `GetRequiredDepositAmount` alongside `get_required_deposit_amount` for `GetDefaultDepositAmount`. Every message the deployed contract accepts today is still accepted with the same result.
- The error text for an unrecognised message changes from sylvia's "Unsupported message received: ..." to serde's unknown-variant error. Nothing matches on it.
- A wire-compatibility test, written while sylvia is still a dependency, pins the JSON of every message variant to what the sylvia-generated types produce and accept, and stays behind as a golden fixture.
- The four sylvia multitests move to the `TestableNymContract` harness; a new App-level test covers the redemption reply path, which has no test today.
- `sylvia` leaves `contracts/ecash/Cargo.toml` and the `contracts/` workspace.
- No mainnet migration in this change: the new build ships with the next ecash migration that has a functional reason. `MigrateMsg` keeps its shape.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ecash-contract`: adds a requirement fixing the entry-point set and the message types the contract parses (wire-compatible with the sylvia build, no `sudo`); the default-deposit query requirement now describes one query variant accepted under three names instead of two variants; `UpdateDefaultDepositValue` is also accepted as `update_deposit_value`.

## Impact

- Code: `contracts/ecash/src/contract/{mod.rs, helpers.rs, test.rs, queued_migrations.rs}`, `contracts/ecash/src/multitest.rs` (replaced by a harness-based test module), `contracts/ecash/src/lib.rs`, `contracts/ecash/Cargo.toml`, `contracts/Cargo.toml` (workspace `sylvia` entry), `contracts/Cargo.lock`.
- Wire: no change for any client; the contract parses the types clients already send. The JSON schema (`contracts/ecash/schema/`) and the TypeScript clients generated from it are unchanged, since both already come from `nym-ecash-contract-common`.
- Deployment: none in this change. The artifact is built with the optimizer and checked with `cosmwasm-check`; mainnet stays on code 71 until a functional ecash migration picks the build up.
- Dependencies: `sylvia` (and with it `sylvia-derive`, `serde-cw-value`, `konst` and the rest of its private graph) leaves the contracts lockfile.
