// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::account::VestingAccountStorageKey;
use cosmwasm_std::{Addr, Coin, OverflowError, StdError, Uint128};
use mixnet_contract_common::NodeId;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VestingContractError {
    #[error("VESTING ({l}): {0}", l = line!())]
    Std(#[from] StdError),

    #[error("VESTING: {0}")]
    OverflowError(#[from] OverflowError),

    #[error("VESTING ({l}): Account does not exist - {0}", l = line!())]
    NoAccountForAddress(String),

    #[error("VESTING ({l}): Only admin can perform this action, {0} is not admin", l = line!())]
    NotAdmin(String),

    #[error("VESTING ({l}): Balance not found for existing account ({0}), this is a bug", l = line!())]
    NoBalanceForAddress(String),

    #[error("VESTING ({l}): Insufficient balance for address {0} -> {1}", l = line!())]
    InsufficientBalance(String, u128),

    #[error("VESTING ({l}): Insufficient spendable balance for address {0} -> {1}", l = line!())]
    InsufficientSpendable(String, u128),

    #[error(
    "VESTING ({l}):Only delegation owner can perform delegation actions, {0} is not the delegation owner"
    , l = line!())]
    NotDelegate(String),

    #[error("VESTING ({l}): Total vesting amount is inprobably low -> {0}, this is likely an error", l = line!())]
    ImprobableVestingAmount(u128),

    #[error("VESTING ({l}): Address {0} has already bonded a node", l = line!())]
    AlreadyBonded(String),

    #[error("VESTING ({l}): Received empty funds vector", l = line!())]
    EmptyFunds,

    #[error("VESTING ({l}): Received wrong denom: {0}, expected {1}", l = line!())]
    WrongDenom(String, String),

    #[error("VESTING ({l}): Received multiple denoms, expected 1", l = line!())]
    MultipleDenoms,

    #[error("VESTING ({l}): No delegations found for account {0}, mix_identity {1}", l = line!())]
    NoSuchDelegation(Addr, NodeId),

    #[error("VESTING ({l}): Only mixnet contract can perform this operation, got {0}", l = line!())]
    NotMixnetContract(Addr),

    #[error("VESTING ({l}): Calculation underflowed", l = line!())]
    Underflow,

    #[error("VESTING ({l}): No bond found for account {0}", l = line!())]
    NoBondFound(String),

    #[error("VESTING: Attempted to reduce mixnode bond pledge below zero! The current pledge is {current} and we attempted to reduce it by {decrease_by}.")]
    InvalidBondPledgeReduction { current: Coin, decrease_by: Coin },

    #[error("VESTING ({l}): Action can only be executed by account owner -> {0}", l = line!())]
    NotOwner(String),

    #[error("VESTING ({l}): Invalid address: {0}", l = line!())]
    InvalidAddress(String),

    #[error("VESTING ({l}): Account already exists: {0}", l = line!())]
    AccountAlreadyExists(String),

    #[error("VESTING ({l}): Staking account already exists: {0}", l = line!())]
    StakingAccountAlreadyExists(String),

    #[error("VESTING ({l}): Too few coins sent for vesting account creation, sent {sent}, need at least {need}", l = line!())]
    MinVestingFunds { sent: u128, need: u128 },

    #[error("VESTING ({l}): Maximum amount of locked coins has already been pledged: {current}, cap is {cap}", l = line!())]
    LockedPledgeCapReached { current: Uint128, cap: Uint128 },

    #[error("VESTING: ({l}: Account owned by {owner} has unpopulated vesting periods!", l = line!())]
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
