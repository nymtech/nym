## ADDED Requirements

### Requirement: Entry points dispatch on the common crate's message types

The contract SHALL export exactly five CosmWasm entry points: `instantiate`, `execute`, `query`, `migrate` and `reply`. It SHALL NOT export `sudo`. The `instantiate`, `execute`, `query` and `migrate` entry points SHALL deserialise their message into `nym_ecash_contract_common::msg::InstantiateMsg`, `ExecuteMsg`, `QueryMsg` and `MigrateMsg` respectively, the types every client serialises; the contract crate SHALL NOT define its own copy of any of them. The `reply` entry point SHALL take the raw `cosmwasm_std::Reply` and dispatch by `Reply::id` as described in the redemption-proposal reply requirement.

Message parsing SHALL follow `cw_serde` semantics: variant names are snake_case and externally tagged, `serde` aliases declared on a variant are accepted, and unknown fields inside a recognised variant are ignored. A message whose variant name is neither a variant nor an alias of the target type SHALL be rejected by the entry point before any handler runs, leaving storage unchanged.

The JSON the common crate's types serialise SHALL be byte-identical to the JSON the sylvia-generated types of the previous build serialised for the same values, for every variant of `ExecuteMsg` and `QueryMsg` and for `InstantiateMsg` and `MigrateMsg`. A golden fixture of that JSON, captured from the sylvia-generated types, SHALL be kept in the contract crate's tests, and every fixture entry SHALL deserialise into the common crate's type as the variant its top-level key names.

#### Scenario: Every pinned message deserialises into the common crate's type
- **WHEN** each entry of the golden fixture is deserialised into the common crate's message type for its entry point
- **THEN** deserialisation succeeds and the resulting variant is the one whose snake_case name is the fixture entry's top-level key (or, for an alias, the variant declaring it)

#### Scenario: Hand-written messages serialise to the pinned bytes
- **WHEN** every variant of `ExecuteMsg` and `QueryMsg`, and `InstantiateMsg` and `MigrateMsg`, is built with the fixture's values and serialised with `cosmwasm_std::to_json_vec`
- **THEN** each output equals the corresponding golden fixture entry byte for byte

#### Scenario: Unknown fields are ignored
- **WHEN** `execute` receives `{"deposit_ticket_book_funds":{"identity_key":"<valid bs58 key>","unexpected":1}}` with the default deposit attached
- **THEN** the deposit is processed exactly as if `unexpected` were absent

#### Scenario: Unknown variant is rejected before any handler
- **WHEN** `execute` receives `{"no_such_message":{}}`
- **THEN** the call fails with a message-parse error and no storage key changes

#### Scenario: The artifact exports no `sudo`
- **WHEN** the export section of the optimised `ecash` artifact is listed
- **THEN** it contains `instantiate`, `execute`, `query`, `migrate` and `reply`, and does not contain `sudo`

## MODIFIED Requirements

### Requirement: `UpdateDefaultDepositValue` is admin-gated and refuses values below the ticketbook size

`ExecuteMsg::UpdateDefaultDepositValue { new_deposit: Coin }` SHALL be callable only by the contract admin (`contract_admin.assert_admin`). The variant SHALL be accepted under its snake_case name `update_default_deposit_value` and under the alias `update_deposit_value` (its name before the tiered-pricing change, kept as a `serde` alias in `nym-ecash-contract-common`); both names SHALL behave identically. The handler MUST consult the on-chain `Invariants::ticket_book_size` via `get_ticketbook_size`; if the snapshotted value differs from the current crate `TICKETBOOK_SIZE`, the call MUST fail with `TicketBookSizeChanged { at_init, current }` *before* any further work. If `new_deposit.amount < TICKETBOOK_SIZE`, the call MUST fail with `DepositBelowTicketBookSize { amount, ticket_book_size }`. On success the handler SHALL overwrite `Config::deposit_amount` with `new_deposit` and emit a `wasm` event attribute `updated_deposit` carrying the new value as its `Coin::to_string()` form.

Note: the handler does *not* validate that `new_deposit.denom` matches the existing config's denom. Changing the denom is permitted by the current handler but is operationally hazardous (existing reduced-deposit entries and statistics use the old denom). Reviewers should treat denom changes as a coordinated migration concern, not an admin operation.

#### Scenario: Admin successfully bumps the default price
- **WHEN** the contract admin sends `UpdateDefaultDepositValue { new_deposit }` with `new_deposit.amount >= TICKETBOOK_SIZE` and the network-defaults `TICKETBOOK_SIZE` matches the snapshotted invariant
- **THEN** `Config::deposit_amount` equals `new_deposit` and the response contains a `wasm` attribute `updated_deposit = new_deposit.to_string()`

#### Scenario: The legacy `update_deposit_value` name is accepted
- **WHEN** the contract admin sends `{"update_deposit_value":{"new_deposit":<coin>}}` with a valid `new_deposit`
- **THEN** the call has the same effect and response as `{"update_default_deposit_value":{"new_deposit":<coin>}}`

#### Scenario: Non-admin call is rejected
- **WHEN** any non-admin sends `UpdateDefaultDepositValue`
- **THEN** the call returns `EcashContractError::Admin` and `Config::deposit_amount` is unchanged

#### Scenario: Value below ticketbook size is rejected
- **WHEN** the admin sends `UpdateDefaultDepositValue { new_deposit }` with `new_deposit.amount < TICKETBOOK_SIZE`
- **THEN** the call returns `DepositBelowTicketBookSize { amount: new_deposit.amount, ticket_book_size: TICKETBOOK_SIZE }`

### Requirement: Default-deposit queries

The contract SHALL expose one query for the default deposit amount, `QueryMsg::GetDefaultDepositAmount {}`, returning `Coin = Config::deposit_amount`. For backwards compatibility the variant SHALL be accepted under three names: `get_default_deposit_amount`, `get_required_deposit_amount` and `GetRequiredDepositAmount` (the latter two are `serde` aliases on the variant in `nym-ecash-contract-common`). All three names SHALL dispatch to the same handler and return the same value.

The query SHALL succeed unconditionally for any sender (queries are read-only).

#### Scenario: All three names return identical data
- **WHEN** `{"get_default_deposit_amount":{}}`, `{"get_required_deposit_amount":{}}` and `{"GetRequiredDepositAmount":{}}` are queried on the same contract state
- **THEN** all three return the same `Coin`, equal to `Config::deposit_amount`
