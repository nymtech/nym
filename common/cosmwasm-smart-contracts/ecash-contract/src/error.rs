// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, StdError};
use cw_controllers::AdminError;
use cw_utils::PaymentError;
use thiserror::Error;

/// Errors surfaced by the ecash contract. Each reachable variant is named in at
/// least one scenario of `openspec/specs/ecash-contract/spec.md`.
#[derive(Error, Debug, PartialEq)]
pub enum EcashContractError {
    /// Wrapper for any underlying `cosmwasm_std::StdError` (storage faults,
    /// address validation, etc.).
    #[error(transparent)]
    Std(#[from] StdError),

    /// Raised by `cw_utils::must_pay` on `DepositTicketBookFunds` when funds
    /// are missing, multi-denom, or in the wrong denom. Inner variants
    /// `NoFunds`, `MultipleDenoms`, `MissingDenom` are all reachable.
    #[error("Invalid deposit")]
    InvalidDeposit(#[from] PaymentError),

    /// `DepositTicketBookFunds` with the right denom but a non-matching amount.
    /// `amount` is the reduced amount (if the sender is whitelisted) or the
    /// default amount.
    #[error("received wrong amount for deposit. got: {received}. required: {amount}")]
    WrongAmount { received: Coin, amount: Coin },

    /// **Unreachable** - preserved for forward compatibility (no current
    /// execute path triggers this).
    #[error("There aren't enough funds in the contract")]
    NotEnoughFunds,

    /// Wrapper for `cw_controllers::AdminError`. Raised by every admin-gated
    /// and multisig-gated handler when the sender is wrong.
    #[error(transparent)]
    Admin(#[from] AdminError),

    /// Redemption-proposal reply could not find a `proposal_id` attribute on
    /// the multisig `wasm` event.
    #[error("could not find proposal id inside the multisig reply SubMsg")]
    MissingProposalId,

    /// Redemption-proposal reply found a `proposal_id` attribute that could
    /// not be parsed as `u64`. Realistically unreachable.
    #[error("the proposal id returned by the multisig contract could not be parsed into an u64")]
    MalformedProposalId,

    /// Instantiation given a `group_addr` that failed bech32 validation.
    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    /// **Unreachable** - no current execute path triggers this.
    #[error("Unauthorized")]
    Unauthorized,

    /// **Unreachable** - preserved for future SemVer comparisons during migration.
    #[error("Failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },

    /// Reply dispatcher saw an `id` that does not match
    /// `BLACKLIST_PROPOSAL_REPLY_ID` or `REDEMPTION_PROPOSAL_REPLY_ID`.
    #[error("received an invalid reply id: {id}. it does not correspond to any sent SubMsg")]
    InvalidReplyId { id: u64 },

    /// **Unreachable** - preserved for the (future) typed-deposit-info feature.
    #[error("reached the maximum of 255 different deposit types")]
    MaximumDepositTypesReached,

    /// **Unreachable** - preserved for the (future) typed-deposit-info feature.
    #[error("compressed deposit info {typ} does not corresponds to any known type")]
    UnknownCompressedDepositInfoType { typ: u8 },

    /// **Unreachable** - preserved for the (future) typed-deposit-info feature.
    #[error("deposit info {typ} does not corresponds to any previously seen type")]
    UnknownDepositInfoType { typ: String },

    /// `DepositTicketBookFunds` with an `identity_key` that fails to bs58-decode
    /// to exactly 32 bytes. Raised inside `Deposit::to_bytes` during
    /// `save_deposit`.
    #[error("the provided ed25519 identity was malformed")]
    MalformedEd25519Identity,

    /// `nym_network_defaults::TICKETBOOK_SIZE` has diverged from the value
    /// snapshotted at instantiation in `Item<Invariants>`. Tripwire for
    /// uncoordinated network-defaults bumps.
    #[error("the ticket book size has changed since the contract was created! This was not expected! It used to be {at_init} but it's {current} now! Please let the developers know ASAP!")]
    TicketBookSizeChanged { at_init: u64, current: u64 },

    /// `RequestRedemption` with a `commitment_bs58` that does not decode to a
    /// 32-byte sha256 digest.
    #[error("the provided tickets redemption commitment is malformed")]
    MalformedRedemptionCommitment,

    /// Always thrown by `ProposeToBlacklist` and `AddToBlacklist` until the
    /// blacklist redesign lands.
    #[error("the account blacklisting hasn't been fully implemented yet")]
    UnimplementedBlacklisting,

    /// `SetReducedDepositPrice` (or migration whitelist seeding) given a coin
    /// whose denom does not match `Config::deposit_amount.denom`.
    #[error("reduced deposit must use the same denom as the default deposit (expected '{expected}', got '{got}')")]
    InvalidReducedDepositDenom { expected: String, got: String },

    /// `SetReducedDepositPrice` (or migration whitelist seeding) given a
    /// reduced amount not strictly less than the current default.
    #[error(
        "reduced deposit amount ({reduced}) must be strictly less than the default ({default})"
    )]
    ReducedDepositNotReduced {
        reduced: cosmwasm_std::Uint128,
        default: cosmwasm_std::Uint128,
    },

    /// `RemoveReducedDepositPrice` invoked for an address with no current
    /// reduced-deposit entry.
    #[error("address '{address}' does not have a custom reduced deposit price set")]
    NoReducedDepositPrice { address: String },

    /// `UpdateDefaultDepositValue` or `SetReducedDepositPrice` given an amount
    /// below `nym_network_defaults::TICKETBOOK_SIZE`.
    #[error(
        "deposit amount ({amount}) must be at least the ticket book size ({ticket_book_size})"
    )]
    DepositBelowTicketBookSize {
        amount: cosmwasm_std::Uint128,
        ticket_book_size: u64,
    },
}
