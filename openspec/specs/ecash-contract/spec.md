# ecash-contract Specification

## Purpose
TBD - created by archiving change ecash-contract-spec. Update Purpose after archive.
## Requirements
### Requirement: Contract instantiation persists the runtime config, multisig and admin addresses, and the ticketbook-size invariant

The contract SHALL be instantiable exactly once via the standard CosmWasm `instantiate` entry point. The instantiation message SHALL carry `holding_account` (a string-form Cosmos SDK address reserved for the future pool-contract transition), `multisig_addr` (the cw3 multisig contract address that gates redemption finalisation), `group_addr` (the cw4 group contract referenced by future blacklist proposals), and `deposit_amount` (the default per-deposit price). The handler MUST:

- bech32-validate `multisig_addr`, `holding_account`, and `group_addr`; an invalid `group_addr` MUST surface as `InvalidGroup { addr }`, an invalid `multisig_addr` or `holding_account` MUST surface as the underlying `StdError` from `addr_validate`;
- persist `info.sender` as the `contract_admin` via `cw_controllers::Admin::set` (the message itself does not carry an admin field; the sender becomes admin by convention);
- persist the validated `multisig_addr` as the `multisig` `Admin` slot;
- snapshot `nym_network_defaults::TICKETBOOK_SIZE` into `Item<Invariants> { ticket_book_size }` under the storage key `"expected_invariants"`;
- zero-initialise `PoolCounters` (`total_deposited`, `total_redeemed`, `tickets_requested_and_not_redeemed`) using the denom of the supplied `deposit_amount`;
- persist the assembled `Config { group_addr (as cw4::Cw4Contract), holding_account, deposit_amount }`;
- zero-initialise the default-price statistics accumulators (`deposits_with_default_price` and `deposits_with_default_price_amounts`);
- record the contract name and `CARGO_PKG_VERSION` via `cw2::set_contract_version`;
- record build information via `set_build_information!`.

The instantiation handler SHALL return `Response::default()` (no events, no attributes, no data).

#### Scenario: Valid instantiation persists every state slot
- **WHEN** `instantiate` is called with valid bech32 strings for `holding_account`, `multisig_addr`, `group_addr` and a well-formed `deposit_amount`
- **THEN** the contract stores `Config` verbatim, the validated multisig address (queryable via `multisig.assert_admin`), `info.sender` as the contract admin, the snapshotted `Invariants { ticket_book_size: TICKETBOOK_SIZE }`, zeroed `PoolCounters`, and zeroed default-price stats
- **AND** `cw2::get_contract_version` returns the crate's `CARGO_PKG_VERSION`
- **AND** the returned `Response` carries no events, attributes, or data

#### Scenario: Invalid group address is rejected with a typed error
- **WHEN** `instantiate` is called with a `group_addr` that fails `Addr::validate`
- **THEN** the call returns `EcashContractError::InvalidGroup { addr }` and no state is persisted

#### Scenario: Invalid multisig address propagates the `StdError`
- **WHEN** `instantiate` is called with a `multisig_addr` that fails `Addr::validate`
- **THEN** the call returns an `EcashContractError::Std` wrapping the underlying validation error and no state is persisted

### Requirement: Migration handles version gating, default-price backfill, and whitelist seeding atomically

The contract SHALL expose a `migrate` entry point taking `MigrateMsg { initial_whitelist: Vec<WhitelistedDeposit> }`. The handler MUST:

- call `set_build_information!` to refresh stored build metadata;
- call `cw2::ensure_from_older_version` against `CONTRACT_NAME` and `CONTRACT_VERSION`; an on-chain version strictly greater than `CARGO_PKG_VERSION` MUST be rejected;
- run `queued_migrations::add_tiered_pricing(deps, initial_whitelist)`, which (a) reads `DepositStorage::total_deposits_made` and `PoolCounters::total_deposited` from the pre-migration state, (b) writes those values into `deposits_with_default_price` and `deposits_with_default_price_amounts` respectively (because every pre-migration deposit was at the default price), and (c) iterates `initial_whitelist`, validating each entry's denom and amount (see Requirement "Setting a reduced deposit price"), persisting each into `reduced_deposits`;
- return `Response::default()`.

The migration MUST fail atomically if any whitelist entry fails validation — partial whitelist seeding MUST NOT be persisted.

#### Scenario: Migration on contract with prior default-price deposits backfills both counters
- **WHEN** `migrate` is called with `initial_whitelist = []` on a contract that has performed `N > 0` deposits totalling `T` units
- **THEN** `deposits_with_default_price` reads `N` and `deposits_with_default_price_amounts` reads `T` (same denom as the contract config)

#### Scenario: Migration with a valid whitelist persists every entry
- **WHEN** `migrate` is called with two well-formed `WhitelistedDeposit` entries
- **THEN** `reduced_deposits` returns the matching `Coin` for each address on subsequent reads

#### Scenario: Migration with a single invalid whitelist entry rolls back the entire transaction
- **WHEN** `migrate` is called with `initial_whitelist = [valid_entry, invalid_entry]` where `invalid_entry` uses the wrong denom
- **THEN** the call returns `InvalidReducedDepositDenom { expected, got }` and `reduced_deposits` contains no entries from this migration

#### Scenario: Newer on-chain version is rejected
- **WHEN** `migrate` is called against an on-chain `cw2` version strictly greater than `CARGO_PKG_VERSION`
- **THEN** the call returns an error and storage is unchanged

### Requirement: Authorisation — contract admin and multisig pointer

The contract SHALL maintain two `cw_controllers::Admin` slots at storage keys `"contract_admin"` and `"multisig"`. Only the first carries admin semantics in the operational sense; the second is a stored cw3 contract pointer that uses the same wrapper purely as a generic address-equality helper.

- `contract_admin` SHALL gate `UpdateAdmin`, `UpdateDefaultDepositValue`, `SetReducedDepositPrice`, `RemoveReducedDepositPrice`. It is replaceable through `UpdateAdmin`, which dispatches `cw_controllers::Admin::execute_update_admin` (requiring the current admin to sign the transaction).
- `multisig` SHALL gate `RedeemTickets` via `assert_admin` (which on this slot is effectively "is the caller equal to the stored cw3 contract address?"). It SHALL NOT be updatable through any execute path; replacing it requires redeploying the contract.

The wrapper's full admin-transfer machinery is reachable in code on both slots but is exercised only on `contract_admin`.

#### Scenario: Non-admin call to a contract-admin-gated handler is rejected
- **WHEN** any of `UpdateDefaultDepositValue`, `SetReducedDepositPrice`, `RemoveReducedDepositPrice` is sent by an address other than the current `contract_admin`
- **THEN** the call returns `EcashContractError::Admin` wrapping `AdminError::NotAdmin` and state is unchanged

#### Scenario: Non-multisig call to `RedeemTickets` is rejected
- **WHEN** `RedeemTickets { n, gw }` is sent by an address other than the configured `multisig`
- **THEN** the call returns `EcashContractError::Admin` wrapping `AdminError::NotAdmin` and `PoolCounters::tickets_requested_and_not_redeemed` is unchanged

### Requirement: `DepositTicketBookFunds` classifies the sent amount and updates the correct statistics bucket

`ExecuteMsg::DepositTicketBookFunds { identity_key }` SHALL accept exactly one coin in the configured denom (validated via `cw_utils::must_pay`) and classify the sender at deposit time:

1. If the sent amount equals `Config::deposit_amount.amount`, the deposit is treated as a **default-price** deposit. The handler MUST call `DepositStatsStorage::new_default_deposit`, which increments `deposits_with_default_price` by 1 and adds the deposited amount to `deposits_with_default_price_amounts`. This branch fires *regardless* of whether the sender has a reduced-deposit entry.
2. Otherwise, if the sender has a `reduced_deposits[sender]` entry whose `amount` equals the sent amount, the deposit is treated as a **reduced-price** deposit. The handler MUST call `DepositStatsStorage::new_reduced_deposit`, which increments `deposits_with_custom_price[sender]` by 1 and adds the deposited amount to `deposits_with_custom_price_amounts[sender]`.
3. Otherwise, the call MUST fail with `EcashContractError::WrongAmount { received, amount }`, where `amount` is the reduced amount if the sender is whitelisted, else the default amount.

After successful classification the handler MUST:

- add the deposited amount to `PoolCounters.total_deposited.amount`;
- assign and persist a sequential `deposit_id` via `DepositStorage::save_deposit` (which decodes `identity_key` from bs58 to a 32-byte raw representation and writes it under the `"deposit"` namespace);
- emit a `deposited-funds` event with attribute `deposit-id` carrying the assigned id as a decimal string;
- set the response data to `deposit_id.to_be_bytes()` so callers can recover the id from the transaction result.

The handler MUST NOT verify that the sender controls the private key matching the supplied `identity_key`. The identity-ownership proof is delegated to off-chain nym-api signers (`post_blind_sign` at `nym-api/src/ecash/api_routes/partial_signing.rs:55`).

#### Scenario: Default-price deposit increments default counters and emits event with deposit id
- **WHEN** a sender with no reduced-deposit entry sends `DepositTicketBookFunds { identity_key }` with funds matching the default amount
- **THEN** `deposits_with_default_price` increases by 1, `deposits_with_default_price_amounts` increases by the deposited amount, `PoolCounters.total_deposited` increases by the deposited amount, a new `deposit_id` is persisted with the supplied `identity_key`, a `deposited-funds` event is emitted with the `deposit-id` attribute set to the new id, and the response data is the big-endian byte representation of the new id

#### Scenario: Whitelisted sender paying the default amount is bucketed as default-price
- **WHEN** a whitelisted sender (entry in `reduced_deposits`) sends `DepositTicketBookFunds` with funds matching the **default** amount (not their reduced amount)
- **THEN** the deposit is recorded under `deposits_with_default_price` (not under `deposits_with_custom_price[sender]`), and the deposit is persisted normally

#### Scenario: Whitelisted sender paying the reduced amount is bucketed as custom-price
- **WHEN** a whitelisted sender sends `DepositTicketBookFunds` with funds matching their reduced amount
- **THEN** `deposits_with_custom_price[sender]` increases by 1, `deposits_with_custom_price_amounts[sender]` increases by the reduced amount, and the deposit is persisted normally

#### Scenario: Whitelisted sender paying neither default nor their reduced amount is rejected with the reduced amount as the expected
- **WHEN** a whitelisted sender sends an amount equal to neither `Config::deposit_amount.amount` nor `reduced_deposits[sender].amount`
- **THEN** the call returns `EcashContractError::WrongAmount { received, amount }` where `amount` is the sender's reduced amount

#### Scenario: Non-whitelisted sender paying the wrong amount is rejected with the default amount as the expected
- **WHEN** a sender with no reduced-deposit entry sends an amount different from `Config::deposit_amount.amount`
- **THEN** the call returns `EcashContractError::WrongAmount { received, amount }` where `amount` is `Config::deposit_amount`

#### Scenario: Missing, mismatched, or multi-denom funds are rejected by `must_pay` before amount classification
- **WHEN** `DepositTicketBookFunds` is sent with no attached coins, with multiple denom coins, or with a single coin in a denom different from `Config::deposit_amount.denom`
- **THEN** the call returns `EcashContractError::InvalidDeposit(PaymentError::NoFunds)`, `EcashContractError::InvalidDeposit(PaymentError::MultipleDenoms)`, or `EcashContractError::InvalidDeposit(PaymentError::MissingDenom(<expected-denom>))` respectively, before any classification, counter, or storage write

### Requirement: Deposit identity-key payload is opaque to the contract; ownership is enforced off-chain

The contract SHALL accept `identity_key: String` as an opaque payload at deposit submission time. The handler MUST NOT verify control of the corresponding ed25519 private key. The handler MUST persist the payload such that it round-trips losslessly through `Deposit::to_bytes` → 32-byte raw storage → `Deposit::try_from_bytes` → bs58 string; any payload that fails to decode to exactly 32 bytes via `bs58::decode` MUST surface as `EcashContractError::MalformedEd25519Identity` (raised by `Deposit::to_bytes` during the `save_deposit` flow). The proof-of-control check is performed off-chain by nym-api signers when honouring `post_blind_sign`.

#### Scenario: Sender can claim any well-formed ed25519 pubkey
- **WHEN** sender `A` submits `DepositTicketBookFunds { identity_key: B_pubkey }` where `B_pubkey` is sender `B`'s ed25519 public key, paying the correct amount
- **THEN** the deposit is accepted and `GetDeposit { deposit_id }` returns `Deposit { bs58_encoded_ed25519_pubkey: B_pubkey }`

#### Scenario: Malformed bs58 ed25519 payload is rejected at save time
- **WHEN** `DepositTicketBookFunds { identity_key: "not-a-valid-bs58-key" }` is submitted with the correct funds
- **THEN** the call returns `EcashContractError::MalformedEd25519Identity` and no deposit is persisted, no counters are incremented

### Requirement: Deposit ids are sequential `u32` starting at 0, never recycled

The contract SHALL maintain a single `Item<u32>` counter at the storage key `"deposit_ids"`. The counter SHALL be unset on a freshly instantiated contract and treated as zero. Each successful `DepositTicketBookFunds` invocation SHALL:

- read the current counter value (defaulting to 0 when absent), which becomes the new deposit's id;
- persist `counter + 1` as the new counter value;
- write the 32-byte raw ed25519 pubkey for that id under the `"deposit"` storage namespace (bypassing JSON serialisation for storage efficiency — see `StoredDeposits`).

`DepositStorage::total_deposits_made(storage)` and `DepositStorage::latest_deposit(storage)` SHALL both read the counter directly — `total_deposits_made` returns `unwrap_or(0)` (count of deposits ever performed), `latest_deposit` returns `may_load` (the *next* unused id, which is also the count). Deposit ids SHALL NOT be recycled even if the deposit can later be invalidated off-chain.

#### Scenario: First deposit on a fresh contract gets id 0
- **WHEN** the first-ever `DepositTicketBookFunds` is processed
- **THEN** the assigned `deposit_id` is `0`, the persisted counter becomes `1`, and `total_deposits_made` returns `1`

#### Scenario: Subsequent deposits get strictly increasing ids
- **WHEN** three deposits are processed in sequence
- **THEN** they receive ids `0`, `1`, `2` and the counter becomes `3`

### Requirement: Deposit statistics invariant — default count plus per-account custom counts equals total

The contract SHALL maintain the invariant `DepositStatsStorage::get_total_deposits_made_with_default_price(storage) + sum(deposits_with_custom_price[a] for a in addrs) == DepositStorage::total_deposits_made(storage)` at all times after instantiation. This invariant MUST hold across every successful and failed transaction (a failed deposit MUST NOT touch any counter). The `#[cfg(test)] assert_counts_consistent` helper in `DepositStatsStorage` SHALL exist solely to validate this invariant in tests and is part of the maintenance contract for any code that writes to the `"deposit"` namespace.

#### Scenario: Mixed default and reduced deposits maintain the invariant
- **WHEN** two default-price deposits and one reduced-price deposit by address `alice` are processed in any order
- **THEN** `deposits_with_default_price` reads `2`, `deposits_with_custom_price[alice]` reads `1`, `total_deposits_made` reads `3`, and `2 + 1 == 3`

#### Scenario: A rejected `DepositTicketBookFunds` does not touch any counter
- **WHEN** `DepositTicketBookFunds` is rejected with `WrongAmount`
- **THEN** none of `deposit_ids`, `deposits_with_default_price`, `deposits_with_custom_price` change

### Requirement: `UpdateDefaultDepositValue` is admin-gated and refuses values below the ticketbook size

`ExecuteMsg::UpdateDefaultDepositValue { new_deposit: Coin }` SHALL be callable only by the contract admin (`contract_admin.assert_admin`). The handler MUST consult the on-chain `Invariants::ticket_book_size` via `get_ticketbook_size`; if the snapshotted value differs from the current crate `TICKETBOOK_SIZE`, the call MUST fail with `TicketBookSizeChanged { at_init, current }` *before* any further work. If `new_deposit.amount < TICKETBOOK_SIZE`, the call MUST fail with `DepositBelowTicketBookSize { amount, ticket_book_size }`. On success the handler SHALL overwrite `Config::deposit_amount` with `new_deposit` and emit a `wasm` event attribute `updated_deposit` carrying the new value as its `Coin::to_string()` form.

Note: the handler does *not* validate that `new_deposit.denom` matches the existing config's denom. Changing the denom is permitted by the current handler but is operationally hazardous (existing reduced-deposit entries and statistics use the old denom). Reviewers should treat denom changes as a coordinated migration concern, not an admin operation.

#### Scenario: Admin successfully bumps the default price
- **WHEN** the contract admin sends `UpdateDefaultDepositValue { new_deposit }` with `new_deposit.amount >= TICKETBOOK_SIZE` and the network-defaults `TICKETBOOK_SIZE` matches the snapshotted invariant
- **THEN** `Config::deposit_amount` equals `new_deposit` and the response contains a `wasm` attribute `updated_deposit = new_deposit.to_string()`

#### Scenario: Non-admin call is rejected
- **WHEN** any non-admin sends `UpdateDefaultDepositValue`
- **THEN** the call returns `EcashContractError::Admin` and `Config::deposit_amount` is unchanged

#### Scenario: Value below ticketbook size is rejected
- **WHEN** the admin sends `UpdateDefaultDepositValue { new_deposit }` with `new_deposit.amount < TICKETBOOK_SIZE`
- **THEN** the call returns `DepositBelowTicketBookSize { amount: new_deposit.amount, ticket_book_size: TICKETBOOK_SIZE }`

### Requirement: `SetReducedDepositPrice` is admin-gated with denom, strict-less-than, and ticketbook-size validation

`ExecuteMsg::SetReducedDepositPrice { address, deposit }` SHALL be callable only by the contract admin. The handler MUST validate `address` via `addr_validate`, then call `add_reduced_deposit_address`, which MUST:

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

### Requirement: `RemoveReducedDepositPrice` is admin-gated and requires an existing entry

`ExecuteMsg::RemoveReducedDepositPrice { address }` SHALL be callable only by the contract admin. The handler MUST validate `address`, then check `reduced_deposits.has(storage, address)`; if no entry exists, the call MUST fail with `NoReducedDepositPrice { address }`. On success the entry is removed from `reduced_deposits` and the response carries `wasm` attributes `action = "remove_reduced_deposit_price"` and `address`.

Removal does not retroactively affect historical statistics for the removed address — past `deposits_with_custom_price[address]` and `..._amounts[address]` entries remain.

#### Scenario: Admin removes an existing entry
- **WHEN** the admin sends `RemoveReducedDepositPrice` for an address with an existing reduced entry
- **THEN** `reduced_deposits[address]` is absent, historical `deposits_with_custom_price[address]` is unchanged, and the response carries `action` and `address` attributes

#### Scenario: Removing a non-existent entry is rejected
- **WHEN** the admin sends `RemoveReducedDepositPrice` for an address with no reduced entry
- **THEN** the call returns `NoReducedDepositPrice { address }`

### Requirement: `UpdateAdmin` requires the current admin and validates the new address

`ExecuteMsg::UpdateAdmin { admin }` SHALL be callable only by the current `contract_admin`. The handler MUST `addr_validate` the supplied string, then call `cw_controllers::Admin::execute_update_admin(deps, info, Some(new_admin))`. The cw_controllers handshake performs the sender-equality check internally. On success the new address replaces the current `contract_admin` slot, and the response carries the cw_controllers-standard attributes (`action = "update_admin"`, `admin = <new>`, `sender = <old>`).

The handler always passes `Some(new_admin)` — admin renunciation (passing `None`) is not exposed through the public surface today.

#### Scenario: Current admin transfers admin rights
- **WHEN** the current admin sends `UpdateAdmin { admin: new_admin }` with a valid bech32 string
- **THEN** subsequent `contract_admin.assert_admin` checks succeed only for `new_admin`, and the old admin's calls to admin-gated handlers fail

#### Scenario: Non-admin call is rejected
- **WHEN** an address other than the current admin sends `UpdateAdmin`
- **THEN** the call returns `EcashContractError::Admin` wrapping `AdminError::NotAdmin`

#### Scenario: Invalid bech32 address is rejected
- **WHEN** the current admin sends `UpdateAdmin { admin: "not-bech32" }`
- **THEN** the call returns an `EcashContractError::Std` wrapping the underlying `addr_validate` failure

### Requirement: `RequestRedemption` validates the commitment and dispatches a multisig propose SubMsg

`ExecuteMsg::RequestRedemption { commitment_bs58, number_of_tickets }` SHALL be callable by any address. The handler MUST:

- decode `commitment_bs58` via `bs58::decode(...).into_vec()`; on decode failure or if the decoded length is not exactly 32 bytes (sha256 digest length), the call MUST fail with `MalformedRedemptionCommitment`;
- construct a `cw3` `Propose` message via `create_batch_redemption_proposal`, with title `BATCH_REDEMPTION_PROPOSAL_TITLE = "ecash-redemption"`, description equal to `commitment_bs58`, and a single embedded `ExecuteMsg::RedeemTickets { n: number_of_tickets, gw: <sender> }` targeting the ecash contract itself;
- wrap that proposal in a `SubMsg::reply_always(..., REDEMPTION_PROPOSAL_REPLY_ID)`;
- return `Response::new().add_submessage(submsg)`.

The handler does NOT itself transfer funds, mark tickets as redeemed, or modify any storage on the ecash contract directly. The actual redemption effect (incrementing `tickets_requested_and_not_redeemed`) only fires when the multisig contract subsequently executes the embedded `RedeemTickets` after a successful vote (see Requirement "Legacy `RedeemTickets`").

#### Scenario: Valid commitment dispatches a multisig propose with the right shape
- **WHEN** any sender sends `RequestRedemption` with a 32-byte sha256 digest encoded in bs58 and `number_of_tickets = N`
- **THEN** the response carries a single `SubMsg` with `id == REDEMPTION_PROPOSAL_REPLY_ID`, targeting the multisig contract, encoding a `Propose` with title `"ecash-redemption"`, description equal to the input `commitment_bs58`, and a single inner `WasmMsg::Execute` calling `RedeemTickets { n: N, gw: <sender> }` on the ecash contract

#### Scenario: Non-bs58 commitment is rejected
- **WHEN** `RequestRedemption { commitment_bs58: "!!!" }` is sent
- **THEN** the call returns `MalformedRedemptionCommitment` and no SubMsg is dispatched

#### Scenario: Bs58-decodable but wrong-length commitment is rejected
- **WHEN** `RequestRedemption` is sent with a `commitment_bs58` whose decoded bytes have length != 32
- **THEN** the call returns `MalformedRedemptionCommitment`

### Requirement: The redemption-proposal reply handler captures the multisig proposal id and surfaces it as response data

The contract SHALL register a single `reply` entry point that dispatches by `Reply::id`. For `REDEMPTION_PROPOSAL_REPLY_ID`, the handler MUST call `Reply::multisig_proposal_id()`, which extracts the `proposal_id` attribute (`PROPOSAL_ID_ATTRIBUTE_NAME = "proposal_id"`) from the `wasm` event of the embedded multisig result. On success the handler MUST return `Response::new().set_data(proposal_id.to_be_bytes())` so callers can recover the multisig-issued id from the transaction result. On failure of the embedded SubMsg, the handler MUST return `EcashContractError::Std(StdError::generic_err(reply_err))`. If the `proposal_id` attribute is missing or malformed, the handler MUST return `MissingProposalId` or `MalformedProposalId` respectively.

#### Scenario: Successful multisig propose flows the proposal id back to the gateway
- **WHEN** the redemption-proposal SubMsg succeeds and the multisig contract emits a `wasm` event with attribute `proposal_id = "42"`
- **THEN** the reply handler returns `Response::new().set_data([0,0,0,0,0,0,0,42])` (big-endian u64)

#### Scenario: Missing proposal_id attribute is reported as a typed error
- **WHEN** the SubMsg succeeds but no `wasm` event carries a `proposal_id` attribute
- **THEN** the reply handler returns `MissingProposalId`

#### Scenario: Failed SubMsg propagates as a generic StdError
- **WHEN** the SubMsg fails with error string `e`
- **THEN** the reply handler returns `EcashContractError::Std(StdError::generic_err(e))`

#### Scenario: Unknown reply id is rejected
- **WHEN** the reply entry point is invoked with `Reply::id` not equal to `REDEMPTION_PROPOSAL_REPLY_ID` or `BLACKLIST_PROPOSAL_REPLY_ID`
- **THEN** the handler returns `InvalidReplyId { id }`

### Requirement: Legacy `RedeemTickets` is multisig-gated, bumps the unredeemed-tickets counter, and emits a single event

`ExecuteMsg::RedeemTickets { n, gw }` is **dead code retained on the public surface for backwards compatibility only**. It SHALL be callable **only** by the configured `multisig` address (`multisig.assert_admin`). The handler MUST:

- ignore `gw` at runtime (the argument is preserved in the transaction body but not consumed);
- increment `PoolCounters.tickets_requested_and_not_redeemed` by `n` (as `u64`);
- emit `Event::new("ticket_redemption").add_attribute("moved_to_holding_account", "false")`.

The handler does not move funds. No known active consumer depends on the counter increment or the event; both are remnants of the deprecated semantics. A follow-on change may remove this variant entirely.

#### Scenario: Multisig successfully redeems n tickets
- **WHEN** the configured multisig sends `RedeemTickets { n: 5, gw: "<gateway-addr>" }`
- **THEN** `PoolCounters.tickets_requested_and_not_redeemed` increases by 5, and the response carries a `ticket_redemption` event with `moved_to_holding_account = "false"`

#### Scenario: Non-multisig caller is rejected
- **WHEN** an address other than the configured multisig sends `RedeemTickets`
- **THEN** the call returns `EcashContractError::Admin` and the counter is unchanged

### Requirement: Stubbed blacklist execute handlers always return `UnimplementedBlacklisting`

`ExecuteMsg::ProposeToBlacklist { public_key }` and `ExecuteMsg::AddToBlacklist { public_key }` SHALL be present in the public schema but SHALL always return `EcashContractError::UnimplementedBlacklisting`. The handlers MUST NOT touch storage, dispatch SubMsgs, or emit events.

The blacklist storage map (`blacklist: Map<BlacklistKey, Blacklisting>` at storage key `"blacklist"`), the reply handler for `BLACKLIST_PROPOSAL_REPLY_ID`, and the `create_blacklist_proposal` helper remain wired in source. They are preserved as the starting point for the redesign and are NOT reachable from the current public ExecuteMsg surface.

#### Scenario: `ProposeToBlacklist` always errors
- **WHEN** any sender (admin, multisig, or other) sends `ProposeToBlacklist { public_key }`
- **THEN** the call returns `EcashContractError::UnimplementedBlacklisting` and the `blacklist` storage is unchanged

#### Scenario: `AddToBlacklist` always errors
- **WHEN** any sender sends `AddToBlacklist { public_key }`
- **THEN** the call returns `EcashContractError::UnimplementedBlacklisting` and the `blacklist` storage is unchanged

### Requirement: The ticketbook-size invariant tripwire catches uncoordinated network-defaults bumps

The contract SHALL persist `Item<Invariants> { ticket_book_size }` at storage key `"expected_invariants"` on instantiation, snapshotting `nym_network_defaults::TICKETBOOK_SIZE`. Every priced operation that consults the ticketbook size (currently `UpdateDefaultDepositValue` and `SetReducedDepositPrice` / migration whitelist seeding via `add_reduced_deposit_address`) SHALL call `NymEcashContract::get_ticketbook_size`, which compares the stored value to the crate's current `TICKETBOOK_SIZE`. A mismatch MUST surface as `TicketBookSizeChanged { at_init, current }`, halting the operation before any state mutation.

#### Scenario: Snapshot equals current crate constant — operation proceeds
- **WHEN** the snapshotted `Invariants::ticket_book_size` equals `nym_network_defaults::TICKETBOOK_SIZE` and an admin sends a valid `UpdateDefaultDepositValue`
- **THEN** the operation succeeds normally

#### Scenario: Snapshot differs from current crate constant — operation halted
- **WHEN** the snapshotted `Invariants::ticket_book_size` is `T_init` and the contract has been redeployed with a new crate constant `T_current != T_init`, then an admin sends `UpdateDefaultDepositValue` or migration seeds a reduced-deposit entry
- **THEN** the call returns `TicketBookSizeChanged { at_init: T_init, current: T_current }` before any further work

### Requirement: Default-deposit queries

The contract SHALL expose two equivalent queries for the default deposit amount, kept aliased for backwards compatibility:

- `QueryMsg::GetDefaultDepositAmount {}` — returns `Coin = Config::deposit_amount`.
- `QueryMsg::GetRequiredDepositAmount {}` (also accepted via the `get_required_deposit_amount` `serde` alias on `GetDefaultDepositAmount`) — equivalent to the above; the handler delegates to `get_default_deposit_amount`.

Both queries SHALL succeed unconditionally for any sender (queries are read-only).

#### Scenario: Both query variants return identical data
- **WHEN** both `GetDefaultDepositAmount {}` and `GetRequiredDepositAmount {}` are queried on the same contract state
- **THEN** both return the same `Coin`, equal to `Config::deposit_amount`

### Requirement: Reduced-deposit queries

The contract SHALL expose two queries for tier-pricing state:

- `QueryMsg::GetReducedDepositAmount { address }` — validates `address` via `addr_validate`, then returns `Option<Coin> = reduced_deposits.may_load(storage, addr)`. An invalid bech32 input MUST return an `EcashContractError::Std` wrapping the underlying validation error.
- `QueryMsg::GetAllWhitelistedAccounts {}` — returns `WhitelistedAccountsResponse { whitelisted_accounts }` enumerating all entries in `reduced_deposits` in ascending address order.

`GetAllWhitelistedAccounts` is unpaginated by design (the whitelist is expected to remain small).

#### Scenario: Reduced amount is returned when an entry exists
- **WHEN** address `alice` has `reduced_deposits[alice] = Coin { 10, "unym" }` and `GetReducedDepositAmount { address: alice }` is queried
- **THEN** the response is `Some(Coin { 10, "unym" })`

#### Scenario: Absent entry returns None
- **WHEN** address `bob` has no entry and `GetReducedDepositAmount { address: bob }` is queried
- **THEN** the response is `None`

#### Scenario: All whitelisted accounts are enumerated
- **WHEN** the contract has entries for `alice` and `bob` and `GetAllWhitelistedAccounts {}` is queried
- **THEN** the response contains both entries

### Requirement: Deposit-by-id and latest-deposit queries

The contract SHALL expose:

- `QueryMsg::GetDeposit { deposit_id }` — returns `DepositResponse { id: deposit_id, deposit: Option<Deposit> }`. The `deposit` field is `Some` if a deposit was ever persisted under that id (deposits are not deletable). It is `None` if the id has not yet been issued (i.e. `id >= total_deposits_made`).
- `QueryMsg::GetLatestDeposit {}` — returns `LatestDepositResponse { deposit: Option<DepositData> }`. The handler MUST consult `DepositStorage::latest_deposit`, which returns the id of the most recently assigned deposit (`counter - 1` when the counter has been incremented at least once, else `None`), and then load that id. On a fresh contract with no prior deposit, the response is `LatestDepositResponse { deposit: None }`; after any successful deposit, the response is `LatestDepositResponse { deposit: Some(DepositData { id, deposit }) }` where `id` is the most recent assignment.

#### Scenario: Existing deposit is returned by id
- **WHEN** a deposit was persisted at id `0` with `identity_key = K` and `GetDeposit { deposit_id: 0 }` is queried
- **THEN** the response is `DepositResponse { id: 0, deposit: Some(Deposit { bs58_encoded_ed25519_pubkey: K }) }`

#### Scenario: Unknown id returns None
- **WHEN** `GetDeposit { deposit_id: 999 }` is queried and the counter is less than 999
- **THEN** the response is `DepositResponse { id: 999, deposit: None }`

#### Scenario: Fresh contract latest-deposit query returns None
- **WHEN** `GetLatestDeposit {}` is queried on a contract with no deposits
- **THEN** the response is `LatestDepositResponse { deposit: None }`

#### Scenario: After deposits exist, latest-deposit query returns the most recent assignment
- **WHEN** two deposits have been processed (ids `0` and `1`) and `GetLatestDeposit {}` is queried
- **THEN** the response is `LatestDepositResponse { deposit: Some(DepositData { id: 1, deposit: <the deposit persisted at id 1> }) }`

### Requirement: Paginated deposits query

`QueryMsg::GetDepositsPaged { limit, start_after }` SHALL return at most `limit.unwrap_or(DEPOSITS_PAGE_DEFAULT_LIMIT).min(DEPOSITS_PAGE_MAX_LIMIT)` deposits in ascending id order, starting strictly after `start_after`. The defaults `DEPOSITS_PAGE_DEFAULT_LIMIT = 50` and `DEPOSITS_PAGE_MAX_LIMIT = 100` are part of the public surface and changing them is a behavioural change. The response SHALL be `PagedDepositsResponse { deposits, start_next_after }`, where `start_next_after` is the id of the last entry returned (or `None` if zero entries were returned).

#### Scenario: Default limits clamp to the maximum
- **WHEN** `GetDepositsPaged { limit: Some(1000), start_after: None }` is queried on a contract with 200 deposits
- **THEN** the response contains 100 entries (ids 0..=99) and `start_next_after = Some(99)`

#### Scenario: Pagination via `start_after`
- **WHEN** the previous query returned `start_next_after = Some(99)` and a follow-up sends `GetDepositsPaged { limit: None, start_after: Some(99) }`
- **THEN** the response starts at id 100

### Requirement: `GetDepositsStatistics` reassembles the global and per-account picture

`QueryMsg::GetDepositsStatistics {}` SHALL return `DepositsStatistics` populated from:

- `total_deposits_made` ← `DepositStorage::total_deposits_made(storage)`;
- `total_deposited` ← `PoolCounters::total_deposited`;
- `total_deposits_made_with_default_price` ← `deposits_with_default_price`;
- `total_deposited_with_default_price` ← `deposits_with_default_price_amounts`;
- `total_deposits_made_with_custom_price` ← `sum(deposits_with_custom_price[a])` across all addresses;
- `total_deposited_with_custom_price` ← `sum(deposits_with_custom_price_amounts[a])` across all addresses (using `Config::deposit_amount.denom`);
- `deposits_made_with_custom_price: HashMap<String, u32>` and `deposited_with_custom_price: HashMap<String, Coin>` ← per-account aggregates from the custom-price maps, keyed by the stringified `Addr`.

The query MUST be a single read pass (no execute-side write) and is callable by any sender.

#### Scenario: Statistics reflect the post-mixed-deposit state
- **WHEN** two default-price deposits of value 75 each, and one reduced-price deposit by `alice` of value 10, have been processed; then `GetDepositsStatistics {}` is queried
- **THEN** the response has `total_deposits_made = 3`, `total_deposits_made_with_default_price = 2`, `total_deposited_with_default_price.amount = 150`, `total_deposits_made_with_custom_price = 1`, `total_deposited_with_custom_price.amount = 10`, and `deposits_made_with_custom_price["alice"] = 1`

### Requirement: Blacklist queries succeed and return empty on a freshly deployed contract

`QueryMsg::GetBlacklistedAccount { public_key }` and `QueryMsg::GetBlacklistPaged { limit, start_after }` SHALL be callable by any sender and SHALL NOT error on a contract that has no blacklist entries.

- `GetBlacklistedAccount` returns `BlacklistedAccountResponse { account: Option<Blacklisting> }`. On a contract where no entry exists for `public_key`, the response is `BlacklistedAccountResponse { account: None }`.
- `GetBlacklistPaged` returns `PagedBlacklistedAccountResponse` with at most `limit.unwrap_or(BLACKLIST_PAGE_DEFAULT_LIMIT).min(BLACKLIST_PAGE_MAX_LIMIT)` entries (`BLACKLIST_PAGE_DEFAULT_LIMIT = 50`, `BLACKLIST_PAGE_MAX_LIMIT = 75`). On a contract with no entries, the response is empty (`accounts: []`, `start_next_after: None`).

Because the blacklist execute handlers always error (Requirement "Stubbed blacklist execute handlers"), the only way for blacklist storage to contain entries on a live deployment is through the reply path for `BLACKLIST_PROPOSAL_REPLY_ID` — which is itself unreachable from the public ExecuteMsg surface today. Consumers MUST NOT treat an empty blacklist as a security guarantee.

#### Scenario: Querying an absent blacklist entry returns None
- **WHEN** `GetBlacklistedAccount { public_key: K }` is queried on a contract that has never blacklisted `K`
- **THEN** the response is `BlacklistedAccountResponse { account: None }`

#### Scenario: Paginated blacklist on empty contract returns empty
- **WHEN** `GetBlacklistPaged { limit: None, start_after: None }` is queried on a freshly deployed contract
- **THEN** the response is `PagedBlacklistedAccountResponse { accounts: [], per_page: 50, start_next_after: None }`

### Requirement: Public storage layout

The following raw CosmWasm storage keys SHALL be considered part of the public contract surface. Any change to these keys is a breaking change for already-deployed contracts and MUST be accompanied by a coordinated migration:

- `"contract_admin"` — `cw_controllers::Admin` slot for the contract admin (single Cosmos SDK address).
- `"multisig"` — `cw_controllers::Admin` slot for the multisig contract address.
- `"config"` — `Item<Config> { group_addr, holding_account, deposit_amount }`.
- `"pool_counters"` — `Item<PoolCounters> { total_deposited, total_redeemed, tickets_requested_and_not_redeemed }`.
- `"expected_invariants"` — `Item<Invariants> { ticket_book_size }`.
- `"deposit_ids"` — `Item<u32>` deposit-id counter (next free id).
- `"deposit"` — custom raw-bytes namespace for stored deposits (32-byte ed25519 pubkeys), keyed by big-endian `u32`. **Not** a `cw_storage_plus::Map`.
- `"reduced_deposits"` — `Map<Addr, Coin>` per-address reduced-deposit price.
- `"blacklist"` — `Map<String, Blacklisting>` reserved for the future blacklist redesign; unreachable through public execute paths today.
- `"deposits_with_default_price"` — `Item<u32>` count of default-price deposits.
- `"deposits_with_default_price_amounts"` — `Item<Coin>` total amount of default-price deposits.
- `"deposits_with_custom_price"` — `Map<Addr, u32>` per-account count of custom-price deposits.
- `"deposits_with_custom_price_amounts"` — `Map<Addr, Coin>` per-account total amount of custom-price deposits.

In addition, the two reply-id constants `BLACKLIST_PROPOSAL_REPLY_ID = 7759` and `REDEMPTION_PROPOSAL_REPLY_ID = 2137` form part of the public surface for any contract upgrade — changing them invalidates in-flight submessage replies.

#### Scenario: Reading any storage key listed above on a v1 deployment succeeds with the documented type
- **WHEN** a CosmWasm raw-state inspection tool reads any of the listed keys on a contract deployed at the spec's anchor version
- **THEN** the deserialised value matches the documented type (`Item`, `Map`, raw bytes, or `Admin` slot as listed)

### Requirement: Public event surface

The following events SHALL form the public contract surface consumed by chain indexers and downstream tooling. Renaming events or attribute keys MUST be treated as a breaking change:

- `deposited-funds` event, emitted by `DepositTicketBookFunds` on success. Carries attribute `deposit-id` (decimal string form of the new `u32`). Event type constant: `DEPOSITED_FUNDS_EVENT_TYPE`. Attribute constant: `DEPOSIT_ID`.
- `ticket_redemption` event, emitted by `RedeemTickets` on success. Carries attribute `moved_to_holding_account = "false"` (the literal string `"false"`). This event is a remnant of the deprecated `RedeemTickets` flow (see the "Legacy `RedeemTickets`" requirement) and is preserved by the contract today but not consumed by any known live indexer.
- The auto-generated `wasm` event emitted by the redemption-proposal reply handler — carries attribute `proposal_id` (decimal string form of the multisig-issued `u64`). Attribute constant: `PROPOSAL_ID_ATTRIBUTE_NAME`. The `wasm` event name itself is the cosmwasm-std auto-event; the attribute is added via `Response::add_attribute`.
- The auto-generated `wasm` event emitted by `UpdateDefaultDepositValue` — carries attribute `updated_deposit = Coin::to_string()`.
- The auto-generated `wasm` event emitted by `SetReducedDepositPrice` — carries attributes `action = "set_reduced_deposit_price"`, `address`, `deposit = Coin::to_string()`.
- The auto-generated `wasm` event emitted by `RemoveReducedDepositPrice` — carries attributes `action = "remove_reduced_deposit_price"`, `address`.
- The auto-generated `wasm` events from `UpdateAdmin` and the embedded multisig propose carry standard cw_controllers / cw3 attributes; the ecash contract does not add additional attributes on those paths.

Handlers that explicitly return `Response::default()` (instantiation, migration) MUST NOT emit any events.

#### Scenario: Successful default-price deposit emits exactly one `deposited-funds` event with the new id
- **WHEN** a default-price `DepositTicketBookFunds` succeeds and assigns id `42`
- **THEN** the response carries exactly one `deposited-funds` event with attribute `deposit-id = "42"`

#### Scenario: Successful `RedeemTickets` carries the legacy `moved_to_holding_account = "false"` attribute
- **WHEN** a `RedeemTickets { n: 3, gw: "..." }` succeeds
- **THEN** the response carries exactly one `ticket_redemption` event with attribute `moved_to_holding_account = "false"`

#### Scenario: Instantiation emits no events
- **WHEN** the contract is instantiated successfully
- **THEN** the response has no events and no attributes

### Requirement: Public error variants

The `EcashContractError` enum forms part of the public surface — its variants are observable through transaction failure messages and are encoded in JSON schema artefacts. The following variants MUST be reachable through the current public execute / migration surface and MUST NOT be removed without a coordinated schema bump:

- `Std`, `Admin` — wrapper variants over `StdError` and `AdminError`.
- `InvalidDeposit(PaymentError)` — wrapper variant raised by `cw_utils::must_pay` on `DepositTicketBookFunds` when funds are missing, multi-denom, or in the wrong denom. The inner `PaymentError` variants `NoFunds`, `MultipleDenoms`, `MissingDenom(<denom>)` are all reachable.
- `WrongAmount { received, amount }` — `DepositTicketBookFunds` with the right denom but a non-matching amount.
- `InvalidGroup { addr }` — instantiation with an unparseable group address.
- `MalformedRedemptionCommitment` — `RequestRedemption` with a non-32-byte commitment.
- `MalformedEd25519Identity` — `DepositTicketBookFunds` with a non-32-byte bs58 identity payload at save time.
- `InvalidReducedDepositDenom { expected, got }` — denom mismatch in `SetReducedDepositPrice` or migration whitelist seeding.
- `ReducedDepositNotReduced { reduced, default }` — reduced amount not strictly less than default.
- `DepositBelowTicketBookSize { amount, ticket_book_size }` — reduced or default amount below the ticketbook-size floor.
- `NoReducedDepositPrice { address }` — `RemoveReducedDepositPrice` against an absent entry.
- `TicketBookSizeChanged { at_init, current }` — invariant tripwire mismatch.
- `UnimplementedBlacklisting` — `ProposeToBlacklist` or `AddToBlacklist` (always thrown).
- `InvalidReplyId { id }` — reply dispatcher with an unknown id.
- `MissingProposalId` / `MalformedProposalId` — multisig-reply parsing failures.

`NotEnoughFunds`, `MaximumDepositTypesReached`, `UnknownCompressedDepositInfoType { typ }`, `UnknownDepositInfoType { typ }`, `Unauthorized`, and `SemVerFailure { value, error_message }` are present in the enum but are **not reachable** through any current public execute path. They are preserved for forward compatibility (notably for the eventual blacklist redesign and deposit-info-typing feature work) and SHOULD NOT be relied upon by downstream tooling.

#### Scenario: Each reachable variant has at least one triggering scenario in this spec
- **WHEN** a reviewer cross-references the reachable variants listed above against the requirements in this spec
- **THEN** every reachable variant appears as the documented error of at least one scenario

