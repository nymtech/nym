// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, Uint128};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NymPoolContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),

    #[error("this sender is not authorised to revoke this grant. its neither the admin or the original (and still whitelisted) granter")]
    UnauthorizedGrantRevocation,

    #[error("the specified address is already a whitelisted granter")]
    AlreadyAGranter,

    #[error("{addr} is not a permitted granter")]
    InvalidGranter { addr: String },

    #[error("invalid coin denomination. got {got}, but expected {expected}")]
    InvalidDenom { expected: String, got: String },

    #[error("there already exists an active grant for {grantee}. it was granted by {granter} at block height {created_at_height}")]
    GrantAlreadyExist {
        granter: String,
        grantee: String,
        created_at_height: u64,
    },

    #[error("could not find any active grants for {grantee}")]
    GrantNotFound { grantee: String },

    #[error("the provided timestamp value ({timestamp}) is set in the past. the current block timestamp is {current_block_timestamp}")]
    TimestampInThePast {
        timestamp: u64,
        current_block_timestamp: u64,
    },

    #[error("there are not enough tokens to process this request. {available} are available, but {required} is needed.")]
    InsufficientTokens { available: Coin, required: Coin },

    #[error("the period length can't be zero")]
    ZeroAllowancePeriod,

    #[error("the provided coin value is zero")]
    ZeroAmount,

    #[error("the periodic spend limit of {periodic} was set to be higher than the total spend limit {total_limit}")]
    PeriodicGrantOverSpendLimit { periodic: Coin, total_limit: Coin },

    #[error("the accumulation spend limit of {accumulation} was set to be lower than the periodic grant amount of {periodic_grant}")]
    AccumulationBelowGrantAmount {
        accumulation: Coin,
        periodic_grant: Coin,
    },

    #[error("the accumulation spend limit of {accumulation} was set to be higher than the total spend limit of {total_limit}")]
    AccumulationOverSpendLimit {
        accumulation: Coin,
        total_limit: Coin,
    },

    #[error("the specified delayed allowance would never be available. it would become active at {available_timestamp} yet it expires at {expiration_timestamp}")]
    UnattainableDelayedAllowance {
        expiration_timestamp: u64,
        available_timestamp: u64,
    },

    #[error("could not unlock {requested} tokens from {grantee}. it only has {locked} locked")]
    InsufficientLockedTokens {
        grantee: String,
        locked: Uint128,
        requested: Uint128,
    },

    #[error("attempted to spend more tokens than permitted by the current allowance")]
    SpendingAboveAllowance,

    #[error("attempted to send an empty allowance usage request")]
    EmptyUsageRequest,

    #[error("the associated grant has already expired")]
    GrantExpired,

    #[error("the associated grant hasn't expired yet")]
    GrantNotExpired,

    #[error("this grant is not available yet. it will become usable at {available_at_timestamp}")]
    GrantNotYetAvailable { available_at_timestamp: u64 },
}
