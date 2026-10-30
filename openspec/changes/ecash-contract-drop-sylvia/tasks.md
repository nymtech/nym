## 1. Pin the sylvia wire format (sylvia still a dependency)

- [ ] 1.1 Add `#[cfg(test)] mod wire_compat;` to `contracts/ecash/src/lib.rs` and create `contracts/ecash/src/wire_compat.rs` with a const table of `(case name, JSON literal)` covering: all 9 `ExecuteMsg` variants; all 10 sylvia query variants (`get_required_deposit_amount` pairs with `QueryMsg::GetDefaultDepositAmount` on the common side); `GetBlacklistPaged` and `GetDepositsPaged` with both `Option` fields set and both `None`; `InstantiateMsg`; `MigrateMsg` with an empty and a one-entry `initial_whitelist`. Values contain no control characters; `identity_key` is a valid bs58 ed25519 key.
- [ ] 1.2 For every case, build the sylvia-generated message (`crate::contract::sv::{InstantiateMsg, ExecMsg, QueryMsg, MigrateMsg}`) and the `nym_ecash_contract_common::msg` message with the same values, and assert both `cosmwasm_std::to_json_string` outputs equal the literal.
- [ ] 1.3 For every case, assert sylvia's contract-level deserialisers (`sv::ContractExecMsg`, `sv::ContractQueryMsg`, `sv::InstantiateMsg`, `sv::MigrateMsg`) accept the common crate's serialised output via `cosmwasm_std::from_json`.
- [ ] 1.4 `cd contracts && cargo test -p nym-ecash wire_compat` passes on the unmodified contract (check the exit code, not filtered output).

## 2. Rewrite the contract on plain entry points

- [ ] 2.1 In `contracts/ecash/src/contract/mod.rs`, remove `#[entry_points]`, `#[contract]`, `#[sv::error(..)]`, every `#[sv::msg(..)]` and the sylvia imports; replace each handler's ctx parameter with the context pieces it uses (`DepsMut`/`Deps`, `Env`, `MessageInfo`), keeping names, message-field parameters and bodies otherwise unchanged; `instantiate` takes `InstantiateMsg` and `migrate` takes `MigrateMsg` whole.
- [ ] 2.2 Delete the `get_required_deposit_amount` method (the common crate's alias routes that name to `get_default_deposit_amount`); move `reply`, `handle_blacklist_proposal_reply` and `handle_redemption_proposal_reply` to plain `DepsMut`/`Env` (drop the `#[allow(deprecated)]` that only covered sylvia's legacy reply form, if nothing else needs it).
- [ ] 2.3 In `contracts/ecash/src/contract/helpers.rs`, change `create_redemption_proposal` and `create_blacklist_proposal` to take the pieces they read (deps, sender, contract address) instead of `ExecCtx`.
- [ ] 2.4 Create `contracts/ecash/src/contract/entry_points.rs` with `#[entry_point]` functions `instantiate`, `execute`, `query`, `migrate`, `reply` over `nym_ecash_contract_common::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg}` and `Reply`, each building `NymEcashContract::new()` and matching every variant; arms bind fields by name and query arms return `to_json_binary(..)`. No `sudo`. Wire the module from `contract/mod.rs`.
- [ ] 2.5 `cd contracts && cargo check -p nym-ecash --lib` passes (tests may still be red until group 3).

## 3. Port the tests

- [ ] 3.1 `contract/test.rs`: `TestSetup` instantiates through `entry_points::instantiate` with an `InstantiateMsg`, holds `OwnedDeps<MockStorage, MockApi, MockQuerier>` (`cosmwasm_std::testing::MockStorage` replaces `MemoryStorage`), and replaces `query_ctx`/`exec_ctx` with direct `deps`/`env`/`MessageInfo` access; update the 32 handler call sites; replace `sylvia::anyhow` with the `anyhow` dev-dependency. Typed `assert_eq!` on `EcashContractError` stays.
- [ ] 3.2 Rewrite `multitest.rs` on `ContractTester<EcashContract>`: a `#[cfg(test)]` `TestableNymContract` impl for `EcashContract` in the same file (no feature flag), overriding `dyn_contract()` to add `.with_reply(reply)`; port `invalid_deposit`, `wrong_deposit_amount`, `correct_default_deposit_succeeds` and `reduced_price_deposit_end_to_end` with their assertions unchanged, funding the non-master senders from the master account.
- [ ] 3.3 Add a stub multisig to `multitest.rs` (test-only contract whose execute returns a response with attribute `proposal_id`, and a failing mode) and two tests: `RequestRedemption` with a valid commitment returns response data equal to the stub's proposal id as big-endian `u64`; with the stub failing, the transaction fails.
- [ ] 3.4 Add a test that queries `GetDepositsPaged` through the `query` entry point with distinct `limit` and `start_after` values over enough deposits to tell them apart.
- [ ] 3.5 Reduce `wire_compat.rs` to its sylvia-free form: drop the sylvia half; keep for every case that the common crate's message serialises to the literal and that the literal deserialises back to an equal value; add that `update_deposit_value`, `get_required_deposit_amount` and `GetRequiredDepositAmount` inputs deserialise to their variants, that an unknown field inside `deposit_ticket_book_funds` deserialises equal to the message without it, and that `{"no_such_message":{}}` fails to deserialise as `ExecuteMsg`.
- [ ] 3.6 `cd contracts && cargo test -p nym-ecash` passes (exit code).

## 4. Remove sylvia from the graph

- [ ] 4.1 Remove `sylvia` from `contracts/ecash/Cargo.toml` (both `[dependencies]` and `[dev-dependencies]`; `anyhow` is already a dev-dependency) and the `sylvia` entry from `[workspace.dependencies]` in `contracts/Cargo.toml`.
- [ ] 4.2 Refresh `contracts/Cargo.lock` with `cargo check --workspace` (no `cargo update`); confirm `cargo tree -i sylvia` finds nothing and that `sylvia`, `sylvia-derive`, `konst` and `serde-cw-value` are gone from the lockfile, with no other package version changed.
- [ ] 4.3 `cd contracts && cargo check --workspace --all-targets` and `cargo test -p nym-ecash` pass.

## 5. Verify the artifact

- [ ] 5.1 `cargo fmt` in `contracts/`.
- [ ] 5.2 `make optimize-contracts CONTRACT_DIRS="contracts/ecash"`, then `make docker-check-contracts` with the default `cosmwasm-check` 2.2.9 and again with `COSMWASM_CHECK_VERSION=3.0.11 COSMWASM_CHECK_IMAGE=rust:1.96`; both exit 0.
- [ ] 5.3 List the new artifact's exports (`wasm-tools print <artifact> | grep '^  (export'`) and confirm they equal develop's minus `sudo`: `memory`, `instantiate`, `execute`, `query`, `migrate`, `reply`, `interface_version_8`, `allocate`, `deallocate`, `requires_staking`, `requires_iterator`, `__data_end`, `__heap_base`.
- [ ] 5.4 Record the artifact's size, per-function max locals and total locals next to develop's 522,123 B / 63 / 2,865 for the PR description.
- [ ] 5.5 `make contract-schema` then `git diff --exit-code -- contracts/ecash/schema` is clean.
- [ ] 5.6 `git diff --exit-code develop -- common/cosmwasm-smart-contracts/ecash-contract` is clean: the client-side crate is untouched, so clients built after this change send the same bytes as clients built before it.
