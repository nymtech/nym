// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::account::VestingAccountStorageKey;
use cosmwasm_std::{Addr, Coin, OverflowError, StdError, Uint128};
use mixnet_contract_common::NodeId;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VestingContractError {
    #[error("VESTING: {0}")]
    Std(#[from] StdError),

    #[error("VESTING: {0}")]
    OverflowError(#[from] OverflowError),

    #[error("VESTING: Account does not exist - {0}")]
    NoAccountForAddress(String),

    #[error("VESTING: Only admin can perform this action, {0} is not admin")]
    NotAdmin(String),

    #[error("VESTING: Balance not found for existing account ({0}), this is a bug")]
    NoBalanceForAddress(String),

    #[error("VESTING: Insufficient balance for address {0} -> {1}")]
    InsufficientBalance(String, u128),

    #[error("VESTING: Insufficient spendable balance for address {0} -> {1}")]
    InsufficientSpendable(String, u128),

    #[error(
    "VESTING:Only delegation owner can perform delegation actions, {0} is not the delegation owner"
    )]
    NotDelegate(String),

    #[error("VESTING: Total vesting amount is inprobably low -> {0}, this is likely an error")]
    ImprobableVestingAmount(u128),

    #[error("VESTING: Address {0} has already bonded a node")]
    AlreadyBonded(String),

    #[error("VESTING: Received empty funds vector")]
    EmptyFunds,

    #[error("VESTING: Received wrong denom: {0}, expected {1}")]
    WrongDenom(String, String),

    #[error("VESTING: Received multiple denoms, expected 1")]
    MultipleDenoms,

    #[error("VESTING: No delegations found for account {0}, mix_identity {1}")]
    NoSuchDelegation(Addr, NodeId),

    #[error("VESTING: Only mixnet contract can perform this operation, got {0}")]
    NotMixnetContract(Addr),

    #[error("VESTING: Calculation underflowed")]
    Underflow,

    #[error("VESTING: No bond found for account {0}")]
    NoBondFound(String),

    #[error("VESTING: Attempted to reduce mixnode bond pledge below zero! The current pledge is {current} and we attempted to reduce it by {decrease_by}.")]
    InvalidBondPledgeReduction { current: Coin, decrease_by: Coin },

    #[error("VESTING: Action can only be executed by account owner -> {0}")]
    NotOwner(String),

    #[error("VESTING: Invalid address: {0}")]
    InvalidAddress(String),

    #[error("VESTING: Account already exists: {0}")]
    AccountAlreadyExists(String),

    #[error("VESTING: Staking account already exists: {0}")]
    StakingAccountAlreadyExists(String),

    #[error("VESTING: Too few coins sent for vesting account creation, sent {sent}, need at least {need}")]
    MinVestingFunds { sent: u128, need: u128 },

    #[error("VESTING: Maximum amount of locked coins has already been pledged: {current}, cap is {cap}")]
    LockedPledgeCapReached { current: Uint128, cap: Uint128 },

    #[error("VESTING: (Account owned by {owner} has unpopulated vesting periods!")]
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
