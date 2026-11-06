## ADDED Requirements

### Requirement: Entry points dispatch on the common crate's message types

The contract SHALL export exactly five CosmWasm entry points: `instantiate`, `execute`, `query`, `migrate` and `reply`. It SHALL NOT export `sudo`. The `instantiate`, `execute`, `query` and `migrate` entry points SHALL deserialise their message into `nym_ecash_contract_common::msg::InstantiateMsg`, `ExecuteMsg`, `QueryMsg` and `MigrateMsg` respectively, the types every client serialises; the contract crate SHALL NOT define its own copy of any of them. The `reply` entry point SHALL take the raw `cosmwasm_std::Reply` and dispatch by `Reply::id` as described in the redemption-proposal reply requirement.

Message parsing SHALL follow `cw_serde` semantics: variant names are snake_case and externally tagged, `serde` aliases declared on a variant are accepted, and unknown fields inside a recognised variant are ignored. A message whose variant name is neither a variant nor an alias of the target type SHALL be rejected by the entry point before any handler runs, leaving storage unchanged.

The JSON the common crate's types serialise SHALL be byte-identical to the JSON the sylvia-generated types of the previous build serialised for the same values, for every variant of `ExecuteMsg` and `QueryMsg` and for `InstantiateMsg`. (`MigrateMsg` is exempt: it is sent only by the admin, to the code being migrated to, and is now empty; see the migration requirement.) A golden fixture of that JSON, captured from the sylvia-generated types, SHALL be kept in the contract crate's tests, and every fixture entry SHALL deserialise into the common crate's type as the variant its top-level key names.

#### Scenario: Every pinned message deserialises into the common crate's type
- **WHEN** each entry of the golden fixture is deserialised into the common crate's message type for its entry point
- **THEN** deserialisation succeeds and the resulting variant is the one whose snake_case name is the fixture entry's top-level key (or, for an alias, the variant declaring it)

#### Scenario: Hand-written messages serialise to the pinned bytes
- **WHEN** every variant of `ExecuteMsg` and `QueryMsg`, and `InstantiateMsg`, is built with the fixture's values and serialised with `cosmwasm_std::to_json_vec`
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

### Requirement: Migration refreshes build information and rejects downgrades

The contract SHALL expose a `migrate` entry point taking `MigrateMsg {}`, a message with no fields. The handler MUST:

- call `set_build_information!` to refresh stored build metadata;
- call `cw2::ensure_from_older_version` against `CONTRACT_NAME` and `CONTRACT_VERSION`; an on-chain version strictly greater than `CARGO_PKG_VERSION` MUST be rejected;
- return `Response::default()`.

The handler MUST NOT modify any other storage. Unknown fields are ignored like everywhere else, so a leftover `initial_whitelist` field from the tiered-pricing migration is accepted and has no effect.

#### Scenario: Migration leaves contract state untouched
- **WHEN** `migrate` is called with `{}` on a contract holding deposits, deposit statistics and reduced-deposit entries
- **THEN** the call succeeds, and the deposit counter, `PoolCounters`, every deposit-statistics item and map, and `reduced_deposits` are unchanged

#### Scenario: A leftover whitelist is ignored
- **WHEN** `migrate` is called with `{"initial_whitelist":[{"address":<addr>,"deposit":<coin>}]}`
- **THEN** the call succeeds and `reduced_deposits` gains no entry

#### Scenario: Newer on-chain version is rejected
- **WHEN** `migrate` is called against an on-chain `cw2` version strictly greater than `CARGO_PKG_VERSION`
- **THEN** the call returns an error and storage is unchanged

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

### Requirement: `SetReducedDepositPrice` is admin-gated with denom, strict-less-than, and ticketbook-size validation

`ExecuteMsg::SetReducedDepositPrice { address, deposit }` SHALL be callable only by the contract admin. The handler MUST validate `address` via `addr_validate`, then:

- reject `deposit.denom != Config::deposit_amount.denom` with `InvalidReducedDepositDenom { expected, got }`;
- reject `deposit.amount >= Config::deposit_amount.amount` with `ReducedDepositNotReduced { reduced, default }` (the reduced amount must be **strictly less than** the default);
- reject `deposit.amount < TICKETBOOK_SIZE` with `DepositBelowTicketBookSize { amount, ticket_book_size }` (subject to the same `get_ticketbook_size` tripwire as Requirement "UpdateDefaultDepositValue");
- persist `reduced_deposits[address] = deposit` (overwriting any existing entry).

On success the handler SHALL emit `wasm` attributes `action = "set_reduced_deposit_price"`, `address`, and `deposit` (as `Coin::to_string()`).

#### Scenario: Admin sets a valid reduced price
- **WHEN** the admin sends `SetReducedDepositPrice { address, deposit }` with matching denom, amount strictly less than default, and amount at least the ticketbook size
- **THEN** `reduced_deposits[address] = deposit` and the response carries attributes `action = "set_reduced_deposit_price"`, `address`, `deposit`

#### Scenario: Overwriting an existing entry succeeds
- **WHEN** the admin sends `SetReducedDepositPrice` for an address that already has a reduced entry
- **THEN** the existing entry is replaced with the new value (no error, no archiving of the old value)

#### Scenario: Reduced amount not strictly less than default is rejected
- **WHEN** the admin sends `SetReducedDepositPrice` with `deposit.amount == Config::deposit_amount.amount`
- **THEN** the call returns `ReducedDepositNotReduced { reduced, default }` with both fields equal

#### Scenario: Mismatched denom is rejected
- **WHEN** the admin sends `SetReducedDepositPrice` with a denom different from `Config::deposit_amount.denom`
- **THEN** the call returns `InvalidReducedDepositDenom { expected, got }`

### Requirement: Default-deposit queries

The contract SHALL expose one query for the default deposit amount, `QueryMsg::GetDefaultDepositAmount {}`, returning `Coin = Config::deposit_amount`. For backwards compatibility the variant SHALL be accepted under three names: `get_default_deposit_amount`, `get_required_deposit_amount` and `GetRequiredDepositAmount` (the latter two are `serde` aliases on the variant in `nym-ecash-contract-common`). All three names SHALL dispatch to the same handler and return the same value.

The query SHALL succeed unconditionally for any sender (queries are read-only).

#### Scenario: All three names return identical data
- **WHEN** `{"get_default_deposit_amount":{}}`, `{"get_required_deposit_amount":{}}` and `{"GetRequiredDepositAmount":{}}` are queried on the same contract state
- **THEN** all three return the same `Coin`, equal to `Config::deposit_amount`

### Requirement: The ticketbook-size invariant tripwire catches uncoordinated network-defaults bumps

The contract SHALL persist `Item<Invariants> { ticket_book_size }` at storage key `"expected_invariants"` on instantiation, snapshotting `nym_network_defaults::TICKETBOOK_SIZE`. Every priced operation that consults the ticketbook size (currently `UpdateDefaultDepositValue` and `SetReducedDepositPrice`) SHALL call `get_ticketbook_size`, which compares the stored value to the crate's current `TICKETBOOK_SIZE`. A mismatch MUST surface as `TicketBookSizeChanged { at_init, current }`, halting the operation before any state mutation.

#### Scenario: Snapshot equals current crate constant - operation proceeds
- **WHEN** the snapshotted `Invariants::ticket_book_size` equals `nym_network_defaults::TICKETBOOK_SIZE` and an admin sends a valid `UpdateDefaultDepositValue`
- **THEN** the operation succeeds normally

#### Scenario: Snapshot differs from current crate constant - operation halted
- **WHEN** the snapshotted `Invariants::ticket_book_size` is `T_init` and the contract has been redeployed with a new crate constant `T_current != T_init`, then an admin sends `UpdateDefaultDepositValue` or `SetReducedDepositPrice`
- **THEN** the call returns `TicketBookSizeChanged { at_init: T_init, current: T_current }` before any further work

### Requirement: Public error variants

The `EcashContractError` enum forms part of the public surface - its variants are observable through transaction failure messages and are encoded in JSON schema artefacts. The following variants MUST be reachable through the current public execute surface and MUST NOT be removed without a coordinated schema bump:

- `Std`, `Admin` - wrapper variants over `StdError` and `AdminError`.
- `InvalidDeposit(PaymentError)` - wrapper variant raised by `cw_utils::must_pay` on `DepositTicketBookFunds` when funds are missing, multi-denom, or in the wrong denom. The inner `PaymentError` variants `NoFunds`, `MultipleDenoms`, `MissingDenom(<denom>)` are all reachable.
- `WrongAmount { received, amount }` - `DepositTicketBookFunds` with the right denom but a non-matching amount.
- `InvalidGroup { addr }` - instantiation with an unparseable group address.
- `MalformedRedemptionCommitment` - `RequestRedemption` with a non-32-byte commitment.
- `MalformedEd25519Identity` - `DepositTicketBookFunds` with a non-32-byte bs58 identity payload at save time.
- `InvalidReducedDepositDenom { expected, got }` - denom mismatch in `SetReducedDepositPrice`.
- `ReducedDepositNotReduced { reduced, default }` - reduced amount not strictly less than default.
- `DepositBelowTicketBookSize { amount, ticket_book_size }` - reduced or default amount below the ticketbook-size floor.
- `NoReducedDepositPrice { address }` - `RemoveReducedDepositPrice` against an absent entry.
- `TicketBookSizeChanged { at_init, current }` - invariant tripwire mismatch.
- `UnimplementedBlacklisting` - `ProposeToBlacklist` or `AddToBlacklist` (always thrown).
- `InvalidReplyId { id }` - reply dispatcher with an unknown id.
- `MissingProposalId` / `MalformedProposalId` - multisig-reply parsing failures.

`NotEnoughFunds`, `MaximumDepositTypesReached`, `UnknownCompressedDepositInfoType { typ }`, `UnknownDepositInfoType { typ }`, `Unauthorized`, and `SemVerFailure { value, error_message }` are present in the enum but are **not reachable** through any current public execute path. They are preserved for forward compatibility (notably for the eventual blacklist redesign and deposit-info-typing feature work) and SHOULD NOT be relied upon by downstream tooling.

#### Scenario: Each reachable variant has at least one triggering scenario in this spec
- **WHEN** a reviewer cross-references the reachable variants listed above against the requirements in this spec
- **THEN** every reachable variant appears as the documented error of at least one scenario

## REMOVED Requirements

### Requirement: Migration handles version gating, default-price backfill, and whitelist seeding atomically

**Reason**: The tiered-pricing migration (`queued_migrations::add_tiered_pricing`) has run on every network, mainnet at code 71. Running it again would overwrite the default-price counters with all-time totals that include custom-price deposits, so it is removed rather than kept as a dormant step.

**Migration**: Replaced by "Migration refreshes build information and rejects downgrades". `MigrateMsg` is now `{}`; reduced-deposit entries are set through `SetReducedDepositPrice`.
