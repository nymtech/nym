// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::account::VestingAccountStorageKey;
use cosmwasm_std::{Addr, Coin, OverflowError, StdError, Uint128};
use mixnet_contract_common::NodeId;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VestingContractError {
    #[error("VESTING ({}): {0}", line!())]
    Std(#[from] StdError),

    #[error("VESTING: {0}")]
    OverflowError(#[from] OverflowError),

    #[error("VESTING ({}): Account does not exist - {0}", line!())]
    NoAccountForAddress(String),

    #[error("VESTING ({}): Only admin can perform this action, {0} is not admin", line!())]
    NotAdmin(String),

    #[error("VESTING ({}): Balance not found for existing account ({0}), this is a bug", line!())]
    NoBalanceForAddress(String),

    #[error("VESTING ({}): Insufficient balance for address {0} -> {1}", line!())]
    InsufficientBalance(String, u128),

    #[error("VESTING ({}): Insufficient spendable balance for address {0} -> {1}", line!())]
    InsufficientSpendable(String, u128),

    #[error(
    "VESTING ({}):Only delegation owner can perform delegation actions, {0} is not the delegation owner"
    , line!())]
    NotDelegate(String),

    #[error("VESTING ({}): Total vesting amount is inprobably low -> {0}, this is likely an error", line!())]
    ImprobableVestingAmount(u128),

    #[error("VESTING ({}): Address {0} has already bonded a node", line!())]
    AlreadyBonded(String),

    #[error("VESTING ({}): Received empty funds vector", line!())]
    EmptyFunds,

    #[error("VESTING ({}): Received wrong denom: {0}, expected {1}", line!())]
    WrongDenom(String, String),

    #[error("VESTING ({}): Received multiple denoms, expected 1", line!())]
    MultipleDenoms,

    #[error("VESTING ({}): No delegations found for account {0}, mix_identity {1}", line!())]
    NoSuchDelegation(Addr, NodeId),

    #[error("VESTING ({}): Only mixnet contract can perform this operation, got {0}", line!())]
    NotMixnetContract(Addr),

    #[error("VESTING ({}): Calculation underflowed", line!())]
    Underflow,

    #[error("VESTING ({}): No bond found for account {0}", line!())]
    NoBondFound(String),

    #[error("VESTING: Attempted to reduce mixnode bond pledge below zero! The current pledge is {current} and we attempted to reduce it by {decrease_by}.")]
    InvalidBondPledgeReduction { current: Coin, decrease_by: Coin },

    #[error("VESTING ({}): Action can only be executed by account owner -> {0}", line!())]
    NotOwner(String),

    #[error("VESTING ({}): Invalid address: {0}", line!())]
    InvalidAddress(String),

    #[error("VESTING ({}): Account already exists: {0}", line!())]
    AccountAlreadyExists(String),

    #[error("VESTING ({}): Staking account already exists: {0}", line!())]
    StakingAccountAlreadyExists(String),

    #[error("VESTING ({}): Too few coins sent for vesting account creation, sent {sent}, need at least {need}", line!())]
    MinVestingFunds { sent: u128, need: u128 },

    #[error("VESTING ({}): Maximum amount of locked coins has already been pledged: {current}, cap is {cap}", line!())]
    LockedPledgeCapReached { current: Uint128, cap: Uint128 },

    #[error("VESTING: ({}: Account owned by {owner} has unpopulated vesting periods!", line!())]
    UnpopulatedVestingPeriods { owner: Addr },

    #[error("VESTING: Vesting account associated with {0} already exists, only addresses with not existing vesting accounts can be added as staking addresses")]
    StakingAccountExists(String),

    #[error("VESTING: {address} is not permitted to perform staking on behalf of {for_account}")]
    InvalidStakingAccount { address: Addr, for_account: Addr },

    #[error("VESTING: {address} ({acc_id} has already performed {num} individual delegations towards {mix_id}. No further delegations are allowed. Please consider consolidating those delegations instead. The current cap is {cap}.")]
    TooManyDelegations {
        address: Addr,
        acc_id: VestingAccountStorageKey,
        mix_id: NodeId,
        num: u32,
        cap: u32,
    },

    #[error("VESTING: Failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },

    #[error("VESTING: {message}")]
    Other { message: String },
}
